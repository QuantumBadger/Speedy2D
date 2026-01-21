/*
 *  Copyright 2021 QuantumBadger
 *
 *  Licensed under the Apache License, Version 2.0 (the "License");
 *  you may not use this file except in compliance with the License.
 *  You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 *  Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS,
 *  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 */

use std::cell::Cell;
use std::convert::{TryFrom, TryInto};
use std::ffi::CString;
use std::num::NonZeroU32;
use std::rc::Rc;

use glutin::config::{Config, ConfigTemplateBuilder};
use glutin::context::{
    ContextApi,
    ContextAttributesBuilder,
    NotCurrentGlContext,
    PossiblyCurrentContext,
    Version
};
use glutin::display::{GetGlDisplay, GlDisplay};
use glutin::surface::{
    GlSurface,
    Surface,
    SurfaceAttributesBuilder,
    SwapInterval,
    WindowSurface
};
use raw_window_handle::HasRawWindowHandle;
use winit::dpi::{LogicalSize, PhysicalPosition, PhysicalSize};
use winit::error::EventLoopError;
use winit::event::{
    ElementState as GlutinElementState,
    Event as GlutinEvent,
    KeyEvent,
    MouseScrollDelta as GlutinMouseScrollDelta,
    TouchPhase,
    WindowEvent as GlutinWindowEvent
};
use winit::event_loop::{
    ControlFlow,
    EventLoop,
    EventLoopBuilder,
    EventLoopClosed,
    EventLoopProxy
};
use winit::keyboard::{Key, KeyLocation, NamedKey};
use winit::monitor::MonitorHandle;
use winit::platform::scancode::PhysicalKeyExtScancode;
use winit::window::{
    CursorGrabMode,
    Icon,
    Window as GlutinWindow,
    Window,
    WindowBuilder,
    WindowLevel
};

use crate::dimen::{IVec2, UVec2, Vec2, Vector2};
use crate::error::{BacktraceError, ErrorMessage};
use crate::glbackend::constants::GL_VERSION;
use crate::glbackend::{GLBackend, GLBackendGlow};
use crate::glutin_winit::{DisplayBuilder, GlWindow};
use crate::window::{
    DrawingWindowHandler,
    EventLoopSendError,
    FileDragState,
    ModifiersState,
    MouseButton,
    MouseScrollDistance,
    UserEventSender,
    VirtualKeyCode,
    WindowCreationError,
    WindowCreationMode,
    WindowCreationOptions,
    WindowEventLoopAction,
    WindowFullscreenMode,
    WindowHandler,
    WindowHelper,
    WindowPosition,
    WindowSize,
    WindowStartupInfo
};
use crate::GLRenderer;

pub(crate) struct WindowHelperGlutin<UserEventType: 'static>
{
    window: Rc<Window>,
    event_proxy: EventLoopProxy<UserEventGlutin<UserEventType>>,
    redraw_requested: Cell<bool>,
    terminate_requested: bool,
    physical_size: UVec2,
    is_mouse_grabbed: Cell<bool>
}

impl<UserEventType> WindowHelperGlutin<UserEventType>
{
    #[inline]
    pub fn new(
        window: &Rc<Window>,
        event_proxy: EventLoopProxy<UserEventGlutin<UserEventType>>,
        initial_physical_size: UVec2
    ) -> Self
    {
        WindowHelperGlutin {
            window: Rc::clone(window),
            event_proxy,
            redraw_requested: Cell::new(false),
            terminate_requested: false,
            physical_size: initial_physical_size,
            is_mouse_grabbed: Cell::new(false)
        }
    }

    #[inline]
    #[must_use]
    pub fn is_redraw_requested(&self) -> bool
    {
        self.redraw_requested.get()
    }

    #[inline]
    pub fn set_redraw_requested(&mut self, redraw_requested: bool)
    {
        self.redraw_requested.set(redraw_requested);
    }

    #[inline]
    pub fn get_event_loop_action(&self) -> WindowEventLoopAction
    {
        match self.terminate_requested {
            true => WindowEventLoopAction::Exit,
            false => WindowEventLoopAction::Continue
        }
    }

    pub fn terminate_loop(&mut self)
    {
        self.terminate_requested = true;
    }

    pub fn set_icon_from_rgba_pixels(
        &self,
        data: Vec<u8>,
        size: UVec2
    ) -> Result<(), BacktraceError<ErrorMessage>>
    {
        self.window.set_window_icon(Some(
            Icon::from_rgba(data, size.x, size.y).map_err(|err| {
                ErrorMessage::msg_with_cause("Icon data was invalid", err)
            })?
        ));

        Ok(())
    }

    pub fn set_cursor_visible(&self, visible: bool)
    {
        self.window.set_cursor_visible(visible);
    }

    pub fn set_cursor_grab(
        &self,
        grabbed: bool
    ) -> Result<(), BacktraceError<ErrorMessage>>
    {
        let central_position = self.physical_size / 2;
        self.window
            .set_cursor_position(PhysicalPosition::new(
                central_position.x as i32,
                central_position.y as i32
            ))
            .map_err(|err| {
                ErrorMessage::msg_with_cause(
                    "Failed to move cursor to center of window",
                    err
                )
            })?;

        let result = if grabbed {
            self.window
                .set_cursor_grab(CursorGrabMode::Locked)
                .or_else(|_| self.window.set_cursor_grab(CursorGrabMode::Confined))
        } else {
            self.window.set_cursor_grab(CursorGrabMode::None)
        };

        match result {
            Ok(_) => {
                self.is_mouse_grabbed.set(grabbed);
                if self
                    .event_proxy
                    .send_event(UserEventGlutin::MouseGrabStatusChanged(grabbed))
                    .is_err()
                {
                    log::error!("Failed to notify app of cursor grab: event loop closed");
                }
                Ok(())
            }
            Err(err) => Err(ErrorMessage::msg_with_cause("Could not grab cursor", err))
        }
    }

    pub fn set_resizable(&self, resizable: bool)
    {
        self.window.set_resizable(resizable);
    }

    #[inline]
    pub fn request_redraw(&self)
    {
        self.redraw_requested.set(true);
    }

    pub fn set_title(&self, title: &str)
    {
        self.window.set_title(title);
    }

    pub fn set_fullscreen_mode(&self, mode: WindowFullscreenMode)
    {
        let window = &self.window;

        window.set_fullscreen(match mode {
            WindowFullscreenMode::Windowed => None,
            WindowFullscreenMode::FullscreenBorderless => {
                Some(winit::window::Fullscreen::Borderless(None))
            }
        });

        let is_fullscreen = match mode {
            WindowFullscreenMode::Windowed => false,
            WindowFullscreenMode::FullscreenBorderless => true
        };

        if self
            .event_proxy
            .send_event(UserEventGlutin::FullscreenStatusChanged(is_fullscreen))
            .is_err()
        {
            log::error!(
                "Failed to notify app of fullscreen status change: event loop closed"
            );
        }
    }

    pub fn set_size_pixels<S: Into<UVec2>>(&self, size: S)
    {
        let size = size.into();

        let _ = self
            .window
            .request_inner_size(PhysicalSize::new(size.x, size.y));
    }

    pub fn get_size_pixels(&self) -> UVec2
    {
        let size = self.window.inner_size();

        UVec2::new(size.width, size.height)
    }

    pub fn set_size_scaled_pixels<S: Into<Vec2>>(&self, size: S)
    {
        let size = size.into();

        let _ = self
            .window
            .request_inner_size(LogicalSize::new(size.x, size.y));
    }

    pub fn set_position_pixels<P: Into<IVec2>>(&self, position: P)
    {
        let position = position.into();

        self.window
            .set_outer_position(PhysicalPosition::new(position.x, position.y));
    }

    pub fn set_position_scaled_pixels<P: Into<Vec2>>(&self, position: P)
    {
        let position = position.into();

        self.window
            .set_outer_position(winit::dpi::LogicalPosition::new(position.x, position.y));
    }

    #[inline]
    #[must_use]
    pub fn get_scale_factor(&self) -> f64
    {
        self.window.scale_factor()
    }

    pub fn create_user_event_sender(&self) -> UserEventSender<UserEventType>
    {
        UserEventSender::new(UserEventSenderGlutin::new(self.event_proxy.clone()))
    }
}

pub(crate) struct WindowGlutin<UserEventType: 'static>
{
    event_loop: EventLoop<UserEventGlutin<UserEventType>>,
    window: Rc<Window>,
    context: Rc<PossiblyCurrentContext>,
    surface: Rc<Surface<WindowSurface>>,
    gl_backend: Rc<dyn GLBackend>
}

impl<UserEventType: 'static> WindowGlutin<UserEventType>
{
    pub fn new(
        title: &str,
        options: WindowCreationOptions
    ) -> Result<WindowGlutin<UserEventType>, BacktraceError<WindowCreationError>>
    {
        let event_loop: EventLoop<UserEventGlutin<UserEventType>> =
            EventLoopBuilder::with_user_event().build()?;

        let primary_monitor = event_loop
            .primary_monitor()
            .or_else(|| {
                log::error!(
                    "Couldn't find primary monitor. Using first available monitor."
                );
                event_loop.available_monitors().next()
            })
            .ok_or_else(|| {
                BacktraceError::new(WindowCreationError::PrimaryMonitorNotFound)
            })?;

        for (num, monitor) in event_loop.available_monitors().enumerate() {
            log::debug!(
                "Monitor #{}{}: {}",
                num,
                if monitor == primary_monitor {
                    " (primary)"
                } else {
                    ""
                },
                match &monitor.name() {
                    None => "<unnamed>",
                    Some(name) => name.as_str()
                }
            );
        }

        let mut window_builder = WindowBuilder::new()
            .with_title(title)
            .with_resizable(options.resizable)
            .with_window_level(
                if options.always_on_top {
                    WindowLevel::AlwaysOnTop
                } else {
                    WindowLevel::Normal
                }
            )
            .with_maximized(options.maximized)
            .with_visible(false)
            .with_transparent(options.transparent)
            .with_decorations(options.decorations);

        match &options.mode {
            WindowCreationMode::Windowed { size, .. } => {
                window_builder = window_builder
                    .with_inner_size(compute_window_size(&primary_monitor, size));
            }

            WindowCreationMode::FullscreenBorderless => {
                window_builder = window_builder.with_fullscreen(Some(
                    winit::window::Fullscreen::Borderless(Some(primary_monitor.clone()))
                ));
            }
        }

        let (context, window, surface) =
            create_best_context(&window_builder, &event_loop, &options).ok_or_else(
                || BacktraceError::new(WindowCreationError::SuitableContextNotFound)
            )?;

        if let WindowCreationMode::Windowed {
            position: Some(position),
            ..
        } = &options.mode
        {
            position_window(&primary_monitor, &window, position);
        }

        // Show window after positioning to avoid the window jumping around
        window.set_visible(true);

        // Set the position again to work around an issue on Linux
        if let WindowCreationMode::Windowed {
            position: Some(position),
            ..
        } = &options.mode
        {
            position_window(&primary_monitor, &window, position);
        }

        let glow_context = unsafe {
            glow::Context::from_loader_function(|ptr| {
                context.display().get_proc_address(
                    CString::new(ptr)
                        .expect("Invalid GL function name string")
                        .as_c_str()
                ) as *const _
            })
        };

        let gl_backend = Rc::new(GLBackendGlow::new(glow_context));

        if let Some(error_name) = gl_backend.gl_get_error_name() {
            log::warn!(
                "Ignoring error in GL bindings during startup: {}",
                error_name
            );
        }

        let version = unsafe { gl_backend.gl_get_string(GL_VERSION) };

        log::info!("Using OpenGL version: {}", version);

        unsafe {
            gl_backend.gl_enable_debug_message_callback();
        };

        Ok(WindowGlutin {
            event_loop,
            window: Rc::new(window),
            context: Rc::new(context),
            surface: Rc::new(surface),
            gl_backend
        })
    }

    pub fn create_user_event_sender(&self) -> UserEventSender<UserEventType>
    {
        UserEventSender::new(UserEventSenderGlutin::new(self.event_loop.create_proxy()))
    }

    pub fn get_inner_size_pixels(&self) -> UVec2
    {
        self.window.inner_size().into()
    }

    fn loop_handle_event<Handler>(
        window: &Rc<Window>,
        context: &Rc<PossiblyCurrentContext>,
        surface: &Rc<Surface<WindowSurface>>,
        handler: &mut DrawingWindowHandler<UserEventType, Handler>,
        event: GlutinEvent<UserEventGlutin<UserEventType>>,
        helper: &mut WindowHelper<UserEventType>
    ) -> WindowEventLoopAction
    where
        Handler: WindowHandler<UserEventType> + 'static
    {
        match event {
            GlutinEvent::LoopExiting => return WindowEventLoopAction::Exit,

            GlutinEvent::UserEvent(event) => match event {
                UserEventGlutin::MouseGrabStatusChanged(grabbed) => {
                    handler.on_mouse_grab_status_changed(helper, grabbed)
                }
                UserEventGlutin::FullscreenStatusChanged(fullscreen) => {
                    handler.on_fullscreen_status_changed(helper, fullscreen)
                }
                UserEventGlutin::UserEvent(event) => handler.on_user_event(helper, event)
            },

            GlutinEvent::WindowEvent { event, .. } => match event {
                GlutinWindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                    log::info!("Scale factor changed: {:?}", scale_factor);
                    handler.on_scale_factor_changed(helper, scale_factor)
                }

                GlutinWindowEvent::Resized(physical_size) => {
                    log::info!("Resized: {:?}", physical_size);
                    if let (Ok(w), Ok(h)) = (
                        NonZeroU32::try_from(physical_size.width),
                        NonZeroU32::try_from(physical_size.height)
                    ) {
                        surface.resize(context, w, h);
                    }
                    helper.inner().physical_size = physical_size.into();
                    handler.on_resize(helper, physical_size.into())
                }

                GlutinWindowEvent::CloseRequested => return WindowEventLoopAction::Exit,

                GlutinWindowEvent::CursorMoved { position, .. } => {
                    let position = Vector2::new(position.x, position.y).into_f32();

                    if helper.inner().is_mouse_grabbed.get() {
                        let central_position = helper.inner().physical_size / 2;
                        window
                            .set_cursor_position(PhysicalPosition::new(
                                central_position.x as i32,
                                central_position.y as i32
                            ))
                            .unwrap();

                        let position = position - central_position.into_f32();

                        if position.magnitude_squared() > 0.0001 {
                            handler.on_mouse_move(helper, position);
                        }
                    } else {
                        handler.on_mouse_move(helper, position);
                    };
                }

                GlutinWindowEvent::MouseInput { state, button, .. } => match state {
                    GlutinElementState::Pressed => {
                        handler.on_mouse_button_down(helper, button.into())
                    }
                    GlutinElementState::Released => {
                        handler.on_mouse_button_up(helper, button.into())
                    }
                },

                GlutinWindowEvent::MouseWheel {
                    delta,
                    phase: TouchPhase::Moved,
                    ..
                } => {
                    let distance = match delta {
                        GlutinMouseScrollDelta::LineDelta(x, y) => {
                            MouseScrollDistance::Lines {
                                x: x as f64,
                                y: y as f64,
                                z: 0.0
                            }
                        }
                        GlutinMouseScrollDelta::PixelDelta(pos) => {
                            MouseScrollDistance::Pixels {
                                x: pos.x,
                                y: pos.y,
                                z: 0.0
                            }
                        }
                    };

                    handler.on_mouse_wheel_scroll(helper, distance);
                }

                GlutinWindowEvent::KeyboardInput { event, .. } => {
                    let virtual_key_code = VirtualKeyCode::try_from(&event).ok();

                    match event.state {
                        GlutinElementState::Pressed => {
                            if let Some(text) = event.text {
                                text.chars().for_each(|c| {
                                    handler.on_keyboard_char(helper, c);
                                });
                            }

                            if !event.repeat {
                                handler.on_key_down(
                                    helper,
                                    virtual_key_code,
                                    event.physical_key.to_scancode().unwrap_or(0)
                                );
                            }
                        }
                        GlutinElementState::Released => {
                            handler.on_key_up(
                                helper,
                                virtual_key_code,
                                event.physical_key.to_scancode().unwrap_or(0)
                            );
                        }
                    }
                }

                GlutinWindowEvent::ModifiersChanged(state) => {
                    handler.on_keyboard_modifiers_changed(helper, state.state().into())
                }

                GlutinWindowEvent::RedrawRequested => {
                    helper.inner().set_redraw_requested(true);
                }

                GlutinWindowEvent::HoveredFile(path) => {
                    handler.on_file_drag(helper, FileDragState::Hover(path));
                }

                GlutinWindowEvent::DroppedFile(path) => {
                    handler.on_file_drag(helper, FileDragState::Dropped(path));
                }

                GlutinWindowEvent::HoveredFileCancelled => {
                    handler.on_file_drag(helper, FileDragState::Cancelled);
                }

                _ => {}
            },

            GlutinEvent::AboutToWait => {
                if helper.inner().is_redraw_requested() {
                    helper.inner().set_redraw_requested(false);
                    handler.on_draw(helper);
                    surface.swap_buffers(context).unwrap();
                }
            }

            _ => {}
        }

        helper.inner().get_event_loop_action()
    }

    pub fn run_loop<Handler>(self, handler: Handler, renderer: GLRenderer) -> !
    where
        Handler: WindowHandler<UserEventType> + 'static
    {
        let window = self.window;
        let context = self.context;
        let surface = self.surface;
        let event_loop = self.event_loop;

        let initial_viewport_size_pixels = window.inner_size().into();

        let mut handler = DrawingWindowHandler::new(handler, renderer);

        let mut helper = WindowHelper::new(WindowHelperGlutin::new(
            &window,
            event_loop.create_proxy(),
            initial_viewport_size_pixels
        ));

        handler.on_start(
            &mut helper,
            WindowStartupInfo::new(initial_viewport_size_pixels, window.scale_factor())
        );

        match helper.inner().get_event_loop_action() {
            WindowEventLoopAction::Continue => {
                // Do nothing
            }
            WindowEventLoopAction::Exit => {
                log::info!("Start callback requested exit!");
                drop(handler);
                std::process::exit(0);
            }
        }

        let mut handler = Some(handler);

        let result = event_loop.run(
            move |event: GlutinEvent<UserEventGlutin<UserEventType>>, target| {
                if handler.is_none() {
                    target.exit();
                } else {
                    let action = WindowGlutin::loop_handle_event(
                        &window,
                        &context,
                        &surface,
                        handler.as_mut().unwrap(),
                        event,
                        &mut helper
                    );

                    match action {
                        WindowEventLoopAction::Continue => {
                            if helper.inner().is_redraw_requested() {
                                target.set_control_flow(ControlFlow::Poll)
                            } else {
                                target.set_control_flow(ControlFlow::Wait)
                            }
                        }
                        WindowEventLoopAction::Exit => {
                            handler = None;
                            target.exit();
                        }
                    }
                }
            }
        );

        if let Err(err) = result {
            log::error!("Exited loop with error: {err:?}");
            std::process::exit(1);
        }

        std::process::exit(0);
    }

    #[inline]
    #[must_use]
    pub fn gl_backend(&self) -> &Rc<dyn GLBackend>
    {
        &self.gl_backend
    }
}

fn gl_config_picker(mut configs: Box<dyn Iterator<Item = Config> + '_>)
    -> Option<Config>
{
    configs.next()
}

fn create_best_context<UserEventType>(
    window_builder: &WindowBuilder,
    event_loop: &EventLoop<UserEventType>,
    options: &WindowCreationOptions
) -> Option<(PossiblyCurrentContext, Window, Surface<WindowSurface>)>
{
    for multisampling in &[options.multisampling, 16, 8, 4, 2, 1, 0] {
        log::info!("Trying multisampling={}...", multisampling);

        let mut template = ConfigTemplateBuilder::new();

        if *multisampling > 1 {
            template = template.with_multisampling(
                (*multisampling)
                    .try_into()
                    .expect("Multisampling level out of bounds")
            );
        }

        let result = DisplayBuilder::new()
            .with_window_builder(Some(window_builder.clone()))
            .build(event_loop, template, gl_config_picker);

        let (window, gl_config) = match result {
            Ok((Some(window), config)) => {
                log::info!("Window created");
                (window, config)
            }
            Ok((None, _)) => {
                log::info!("Failed with null window");
                continue;
            }
            Err(err) => {
                log::info!("Failed with error: {:?}", err);
                continue;
            }
        };

        let gl_display = gl_config.display();

        let context_attributes = ContextAttributesBuilder::new()
            .with_context_api(ContextApi::OpenGl(Some(Version::new(2, 0))))
            .build(Some(window.raw_window_handle()));

        let context =
            match unsafe { gl_display.create_context(&gl_config, &context_attributes) } {
                Ok(context) => context,
                Err(err) => {
                    log::info!("Failed to create context with error: {err:?}");
                    continue;
                }
            };

        let window = match crate::glutin_winit::finalize_window(
            event_loop,
            window_builder.clone(),
            &gl_config
        ) {
            Ok(window) => window,
            Err(err) => {
                log::info!("Failed to finalize window with error: {err:?}");
                continue;
            }
        };

        let attrs = window.build_surface_attributes(SurfaceAttributesBuilder::default());

        let surface = match unsafe {
            gl_config
                .display()
                .create_window_surface(&gl_config, &attrs)
        } {
            Ok(surface) => surface,
            Err(err) => {
                log::info!("Failed to finalize surface with error: {err:?}");
                continue;
            }
        };

        let context = match context.make_current(&surface) {
            Ok(context) => context,
            Err(err) => {
                log::info!("Failed to make context current with error: {err:?}");
                continue;
            }
        };

        if options.vsync {
            if let Err(err) = surface.set_swap_interval(
                &context,
                SwapInterval::Wait(NonZeroU32::new(1).unwrap())
            ) {
                log::error!("Error setting vsync, continuing anyway: {err:?}");
            }
        }

        return Some((context, window, surface));
    }

    log::error!("Failed to create any context.");
    None
}

fn position_window(
    monitor: &MonitorHandle,
    window: &GlutinWindow,
    position: &WindowPosition
)
{
    let monitor_position = monitor.position();

    match position {
        WindowPosition::Center => {
            let monitor_size = monitor.size();
            let outer_size = window.outer_size();

            log::info!(
                "Centering window. Monitor size: {:?}. Window outer size: {:?}.",
                monitor_size,
                outer_size
            );

            window.set_outer_position(PhysicalPosition::new(
                monitor_position.x
                    + ((monitor_size.width as i32 - outer_size.width as i32) / 2),
                monitor_position.y
                    + ((monitor_size.height as i32 - outer_size.height as i32) / 2)
            ));
        }

        WindowPosition::PrimaryMonitorPixelsFromTopLeft(position) => window
            .set_outer_position(PhysicalPosition::new(
                monitor_position.x + position.x,
                monitor_position.y + position.y
            ))
    }
}

fn compute_window_size(monitor: &MonitorHandle, size: &WindowSize) -> PhysicalSize<u32>
{
    let monitor_size = monitor.size();

    match size {
        WindowSize::PhysicalPixels(size) => PhysicalSize::new(size.x, size.y),

        WindowSize::ScaledPixels(size) => {
            LogicalSize::new(size.x, size.y).to_physical(monitor.scale_factor())
        }

        WindowSize::MarginPhysicalPixels(margin) => {
            let margin_physical_px = std::cmp::min(
                *margin,
                std::cmp::min(monitor_size.width, monitor_size.height) / 4
            );

            PhysicalSize::new(
                monitor_size.width - 2 * margin_physical_px,
                monitor_size.height - 2 * margin_physical_px
            )
        }

        WindowSize::MarginScaledPixels(margin) => {
            let margin_physical_px = std::cmp::min(
                (*margin as f64 * monitor.scale_factor()).round() as u32,
                std::cmp::min(monitor_size.width, monitor_size.height) / 4
            );

            PhysicalSize::new(
                monitor_size.width - 2 * margin_physical_px,
                monitor_size.height - 2 * margin_physical_px
            )
        }
    }
}

impl From<winit::event::MouseButton> for MouseButton
{
    fn from(button: winit::event::MouseButton) -> Self
    {
        match button {
            winit::event::MouseButton::Left => MouseButton::Left,
            winit::event::MouseButton::Right => MouseButton::Right,
            winit::event::MouseButton::Middle => MouseButton::Middle,
            winit::event::MouseButton::Other(id) => MouseButton::Other(id),
            winit::event::MouseButton::Back => MouseButton::Back,
            winit::event::MouseButton::Forward => MouseButton::Forward
        }
    }
}

impl TryFrom<&KeyEvent> for VirtualKeyCode
{
    type Error = ();

    fn try_from(event: &KeyEvent) -> Result<Self, Self::Error>
    {
        let lr_variant =
            |left: VirtualKeyCode, right: VirtualKeyCode| match event.location {
                KeyLocation::Standard | KeyLocation::Left => left,
                KeyLocation::Right | KeyLocation::Numpad => right
            };

        let numpad_variant =
            |normal: VirtualKeyCode, numpad: VirtualKeyCode| match event.location {
                KeyLocation::Standard | KeyLocation::Left | KeyLocation::Right => normal,
                KeyLocation::Numpad => numpad
            };

        Ok(match event.logical_key.clone() {
            Key::Named(virtual_key_code) => match virtual_key_code {
                NamedKey::Alt => lr_variant(Self::LAlt, Self::RAlt),
                NamedKey::AltGraph => Self::RAlt,
                NamedKey::ArrowDown => Self::Down,
                NamedKey::ArrowLeft => Self::Left,
                NamedKey::ArrowRight => Self::Right,
                NamedKey::ArrowUp => Self::Up,
                NamedKey::AudioVolumeDown => Self::VolumeDown,
                NamedKey::AudioVolumeMute => Self::Mute,
                NamedKey::AudioVolumeUp => Self::VolumeUp,
                NamedKey::Backspace => Self::Backspace,
                NamedKey::BrowserBack => Self::WebBack,
                NamedKey::BrowserFavorites => Self::WebFavorites,
                NamedKey::BrowserForward => Self::WebForward,
                NamedKey::BrowserHome => Self::WebHome,
                NamedKey::BrowserRefresh => Self::WebRefresh,
                NamedKey::BrowserSearch => Self::WebSearch,
                NamedKey::BrowserStop => Self::WebStop,
                NamedKey::Compose => Self::Compose,
                NamedKey::Control => lr_variant(Self::LControl, Self::RControl),
                NamedKey::Convert => Self::Convert,
                NamedKey::Copy => Self::Copy,
                NamedKey::Cut => Self::Cut,
                NamedKey::Delete => Self::Delete,
                NamedKey::End => Self::End,
                NamedKey::Enter => numpad_variant(Self::Return, Self::NumpadEnter),
                NamedKey::Escape => Self::Escape,
                NamedKey::F1 => Self::F1,
                NamedKey::F2 => Self::F2,
                NamedKey::F3 => Self::F3,
                NamedKey::F4 => Self::F4,
                NamedKey::F5 => Self::F5,
                NamedKey::F6 => Self::F6,
                NamedKey::F7 => Self::F7,
                NamedKey::F8 => Self::F8,
                NamedKey::F9 => Self::F9,
                NamedKey::F10 => Self::F10,
                NamedKey::F11 => Self::F11,
                NamedKey::F12 => Self::F12,
                NamedKey::F13 => Self::F13,
                NamedKey::F14 => Self::F14,
                NamedKey::F15 => Self::F15,
                NamedKey::F16 => Self::F16,
                NamedKey::F17 => Self::F17,
                NamedKey::F18 => Self::F18,
                NamedKey::F19 => Self::F19,
                NamedKey::F20 => Self::F20,
                NamedKey::F21 => Self::F21,
                NamedKey::F22 => Self::F22,
                NamedKey::F23 => Self::F23,
                NamedKey::F24 => Self::F24,
                NamedKey::GoBack => Self::NavigateBackward,
                NamedKey::GoHome => Self::Home,
                NamedKey::Home => Self::Home,
                NamedKey::Insert => Self::Insert,
                NamedKey::KanaMode => Self::Kana,
                NamedKey::KanjiMode => Self::Kanji,
                NamedKey::LaunchMail => Self::Mail,
                NamedKey::MediaPlayPause => Self::PlayPause,
                NamedKey::MediaStop => Self::MediaStop,
                NamedKey::NavigatePrevious => Self::NavigateBackward,
                NamedKey::NonConvert => Self::NoConvert,
                NamedKey::NumLock => Self::Numlock,
                NamedKey::PageDown => Self::PageDown,
                NamedKey::PageUp => Self::PageUp,
                NamedKey::Paste => Self::Paste,
                NamedKey::Power => Self::Power,
                NamedKey::PrintScreen => Self::PrintScreen,
                NamedKey::ScrollLock => Self::ScrollLock,
                NamedKey::Shift => lr_variant(Self::LShift, Self::RShift),
                NamedKey::Tab => Self::Tab,
                NamedKey::Super => lr_variant(Self::LWin, Self::RWin),
                _ => return Err(())
            },
            Key::Character(c) => match c.chars().next().unwrap_or('\0') {
                'A' | 'a' => Self::A,
                'B' | 'b' => Self::B,
                'C' | 'c' => Self::C,
                'D' | 'd' => Self::D,
                'E' | 'e' => Self::E,
                'F' | 'f' => Self::F,
                'G' | 'g' => Self::G,
                'H' | 'h' => Self::H,
                'I' | 'i' => Self::I,
                'J' | 'j' => Self::J,
                'K' | 'k' => Self::K,
                'L' | 'l' => Self::L,
                'M' | 'm' => Self::M,
                'N' | 'n' => Self::N,
                'O' | 'o' => Self::O,
                'P' | 'p' => Self::P,
                'Q' | 'q' => Self::Q,
                'R' | 'r' => Self::R,
                'S' | 's' => Self::S,
                'T' | 't' => Self::T,
                'U' | 'u' => Self::U,
                'V' | 'v' => Self::V,
                'W' | 'w' => Self::W,
                'X' | 'x' => Self::X,
                'Y' | 'y' => Self::Y,
                'Z' | 'z' => Self::Z,
                '0' => numpad_variant(Self::Key0, Self::Numpad0),
                '1' => numpad_variant(Self::Key1, Self::Numpad1),
                '2' => numpad_variant(Self::Key2, Self::Numpad2),
                '3' => numpad_variant(Self::Key3, Self::Numpad3),
                '4' => numpad_variant(Self::Key4, Self::Numpad4),
                '5' => numpad_variant(Self::Key5, Self::Numpad5),
                '6' => numpad_variant(Self::Key6, Self::Numpad6),
                '7' => numpad_variant(Self::Key7, Self::Numpad7),
                '8' => numpad_variant(Self::Key8, Self::Numpad8),
                '9' => numpad_variant(Self::Key9, Self::Numpad9),
                '+' => numpad_variant(Self::Plus, Self::NumpadAdd),
                '-' => numpad_variant(Self::Minus, Self::NumpadSubtract),
                '*' => numpad_variant(Self::Asterisk, Self::NumpadMultiply),
                '/' => numpad_variant(Self::Slash, Self::NumpadDivide),
                ',' => numpad_variant(Self::Comma, Self::NumpadComma),
                '.' => numpad_variant(Self::Period, Self::NumpadDecimal),
                '=' => numpad_variant(Self::Equals, Self::NumpadEquals),
                '^' => Self::Caret,
                '\'' => Self::Apostrophe,
                '\\' => Self::Backslash,
                ':' => Self::Colon,
                '`' => Self::Grave,
                '(' => Self::LBracket,
                ')' => Self::RBracket,
                '\t' => Self::Tab,

                _ => return Err(())
            },
            Key::Unidentified(_) | Key::Dead(_) => return Err(())
        })
    }
}

impl From<winit::keyboard::ModifiersState> for ModifiersState
{
    fn from(state: winit::keyboard::ModifiersState) -> Self
    {
        ModifiersState {
            ctrl: state.control_key(),
            alt: state.alt_key(),
            shift: state.shift_key(),
            logo: state.super_key()
        }
    }
}

impl From<PhysicalSize<u32>> for UVec2
{
    fn from(value: PhysicalSize<u32>) -> Self
    {
        Self::new(value.width, value.height)
    }
}

pub(crate) enum UserEventGlutin<UserEventType: 'static>
{
    MouseGrabStatusChanged(bool),
    FullscreenStatusChanged(bool),
    UserEvent(UserEventType)
}

pub struct UserEventSenderGlutin<UserEventType: 'static>
{
    event_proxy: EventLoopProxy<UserEventGlutin<UserEventType>>
}

impl<UserEventType> Clone for UserEventSenderGlutin<UserEventType>
{
    fn clone(&self) -> Self
    {
        UserEventSenderGlutin {
            event_proxy: self.event_proxy.clone()
        }
    }
}

impl<UserEventType> UserEventSenderGlutin<UserEventType>
{
    fn new(event_proxy: EventLoopProxy<UserEventGlutin<UserEventType>>) -> Self
    {
        Self { event_proxy }
    }

    pub fn send_event(&self, event: UserEventType) -> Result<(), EventLoopSendError>
    {
        self.event_proxy
            .send_event(UserEventGlutin::UserEvent(event))
            .map_err(|err| match err {
                EventLoopClosed(_) => EventLoopSendError::EventLoopNoLongerExists
            })
    }
}

impl From<EventLoopError> for BacktraceError<WindowCreationError>
{
    fn from(value: EventLoopError) -> Self
    {
        Self::new_with_cause(WindowCreationError::EventLoopCreationFailed, value)
    }
}
