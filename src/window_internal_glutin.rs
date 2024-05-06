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
use glutin_winit::{DisplayBuilder, GlWindow};
use raw_window_handle::HasRawWindowHandle;
use winit::dpi::{LogicalSize, PhysicalPosition, PhysicalSize};
use winit::error::EventLoopError;
use winit::event::{
    ElementState as GlutinElementState,
    Event as GlutinEvent,
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
use winit::keyboard::Key as GlutinVirtualKeyCode;
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
use crate::window::{
    DrawingWindowHandler,
    EventLoopSendError,
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
            window: Rc::clone(&window),
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
                        surface.resize(&context, w, h);
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
                    let virtual_key_code = VirtualKeyCode::from(event.logical_key);

                    match event.state {
                        GlutinElementState::Pressed => {
                            event.text.unwrap().chars().for_each(|c| {
                                handler.on_keyboard_char(helper, c);
                            });

                            if !event.repeat {
                                handler.on_key_down(
                                    helper,
                                    Some(virtual_key_code),
                                    event.physical_key.to_scancode().unwrap_or(0)
                                );
                            }
                        }
                        GlutinElementState::Released => {
                            handler.on_key_up(
                                helper,
                                Some(virtual_key_code),
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

fn gl_config_picker(mut configs: Box<dyn Iterator<Item = Config> + '_>) -> Config
{
    configs.next().unwrap()
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

        let window = match glutin_winit::finalize_window(
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

impl From<GlutinVirtualKeyCode> for VirtualKeyCode
{
    fn from(_virtual_key_code: GlutinVirtualKeyCode) -> Self
    {
        // TODO
        return VirtualKeyCode::A;
        /*
        match virtual_key_code {
            GlutinVirtualKeyCode::Key1 => VirtualKeyCode::Key1,
            GlutinVirtualKeyCode::Key2 => VirtualKeyCode::Key2,
            GlutinVirtualKeyCode::Key3 => VirtualKeyCode::Key3,
            GlutinVirtualKeyCode::Key4 => VirtualKeyCode::Key4,
            GlutinVirtualKeyCode::Key5 => VirtualKeyCode::Key5,
            GlutinVirtualKeyCode::Key6 => VirtualKeyCode::Key6,
            GlutinVirtualKeyCode::Key7 => VirtualKeyCode::Key7,
            GlutinVirtualKeyCode::Key8 => VirtualKeyCode::Key8,
            GlutinVirtualKeyCode::Key9 => VirtualKeyCode::Key9,
            GlutinVirtualKeyCode::Key0 => VirtualKeyCode::Key0,
            GlutinVirtualKeyCode::A => VirtualKeyCode::A,
            GlutinVirtualKeyCode::B => VirtualKeyCode::B,
            GlutinVirtualKeyCode::C => VirtualKeyCode::C,
            GlutinVirtualKeyCode::D => VirtualKeyCode::D,
            GlutinVirtualKeyCode::E => VirtualKeyCode::E,
            GlutinVirtualKeyCode::F => VirtualKeyCode::F,
            GlutinVirtualKeyCode::G => VirtualKeyCode::G,
            GlutinVirtualKeyCode::H => VirtualKeyCode::H,
            GlutinVirtualKeyCode::I => VirtualKeyCode::I,
            GlutinVirtualKeyCode::J => VirtualKeyCode::J,
            GlutinVirtualKeyCode::K => VirtualKeyCode::K,
            GlutinVirtualKeyCode::L => VirtualKeyCode::L,
            GlutinVirtualKeyCode::M => VirtualKeyCode::M,
            GlutinVirtualKeyCode::N => VirtualKeyCode::N,
            GlutinVirtualKeyCode::O => VirtualKeyCode::O,
            GlutinVirtualKeyCode::P => VirtualKeyCode::P,
            GlutinVirtualKeyCode::Q => VirtualKeyCode::Q,
            GlutinVirtualKeyCode::R => VirtualKeyCode::R,
            GlutinVirtualKeyCode::S => VirtualKeyCode::S,
            GlutinVirtualKeyCode::T => VirtualKeyCode::T,
            GlutinVirtualKeyCode::U => VirtualKeyCode::U,
            GlutinVirtualKeyCode::V => VirtualKeyCode::V,
            GlutinVirtualKeyCode::W => VirtualKeyCode::W,
            GlutinVirtualKeyCode::X => VirtualKeyCode::X,
            GlutinVirtualKeyCode::Y => VirtualKeyCode::Y,
            GlutinVirtualKeyCode::Z => VirtualKeyCode::Z,
            GlutinVirtualKeyCode::Escape => VirtualKeyCode::Escape,
            GlutinVirtualKeyCode::F1 => VirtualKeyCode::F1,
            GlutinVirtualKeyCode::F2 => VirtualKeyCode::F2,
            GlutinVirtualKeyCode::F3 => VirtualKeyCode::F3,
            GlutinVirtualKeyCode::F4 => VirtualKeyCode::F4,
            GlutinVirtualKeyCode::F5 => VirtualKeyCode::F5,
            GlutinVirtualKeyCode::F6 => VirtualKeyCode::F6,
            GlutinVirtualKeyCode::F7 => VirtualKeyCode::F7,
            GlutinVirtualKeyCode::F8 => VirtualKeyCode::F8,
            GlutinVirtualKeyCode::F9 => VirtualKeyCode::F9,
            GlutinVirtualKeyCode::F10 => VirtualKeyCode::F10,
            GlutinVirtualKeyCode::F11 => VirtualKeyCode::F11,
            GlutinVirtualKeyCode::F12 => VirtualKeyCode::F12,
            GlutinVirtualKeyCode::F13 => VirtualKeyCode::F13,
            GlutinVirtualKeyCode::F14 => VirtualKeyCode::F14,
            GlutinVirtualKeyCode::F15 => VirtualKeyCode::F15,
            GlutinVirtualKeyCode::F16 => VirtualKeyCode::F16,
            GlutinVirtualKeyCode::F17 => VirtualKeyCode::F17,
            GlutinVirtualKeyCode::F18 => VirtualKeyCode::F18,
            GlutinVirtualKeyCode::F19 => VirtualKeyCode::F19,
            GlutinVirtualKeyCode::F20 => VirtualKeyCode::F20,
            GlutinVirtualKeyCode::F21 => VirtualKeyCode::F21,
            GlutinVirtualKeyCode::F22 => VirtualKeyCode::F22,
            GlutinVirtualKeyCode::F23 => VirtualKeyCode::F23,
            GlutinVirtualKeyCode::F24 => VirtualKeyCode::F24,
            GlutinVirtualKeyCode::Snapshot => VirtualKeyCode::PrintScreen,
            GlutinVirtualKeyCode::Scroll => VirtualKeyCode::ScrollLock,
            GlutinVirtualKeyCode::Pause => VirtualKeyCode::PauseBreak,
            GlutinVirtualKeyCode::Insert => VirtualKeyCode::Insert,
            GlutinVirtualKeyCode::Home => VirtualKeyCode::Home,
            GlutinVirtualKeyCode::Delete => VirtualKeyCode::Delete,
            GlutinVirtualKeyCode::End => VirtualKeyCode::End,
            GlutinVirtualKeyCode::PageDown => VirtualKeyCode::PageDown,
            GlutinVirtualKeyCode::PageUp => VirtualKeyCode::PageUp,
            GlutinVirtualKeyCode::Left => VirtualKeyCode::Left,
            GlutinVirtualKeyCode::Up => VirtualKeyCode::Up,
            GlutinVirtualKeyCode::Right => VirtualKeyCode::Right,
            GlutinVirtualKeyCode::Down => VirtualKeyCode::Down,
            GlutinVirtualKeyCode::Back => VirtualKeyCode::Backspace,
            GlutinVirtualKeyCode::Return => VirtualKeyCode::Return,
            GlutinVirtualKeyCode::Space => VirtualKeyCode::Space,
            GlutinVirtualKeyCode::Compose => VirtualKeyCode::Compose,
            GlutinVirtualKeyCode::Caret => VirtualKeyCode::Caret,
            GlutinVirtualKeyCode::Numlock => VirtualKeyCode::Numlock,
            GlutinVirtualKeyCode::Numpad0 => VirtualKeyCode::Numpad0,
            GlutinVirtualKeyCode::Numpad1 => VirtualKeyCode::Numpad1,
            GlutinVirtualKeyCode::Numpad2 => VirtualKeyCode::Numpad2,
            GlutinVirtualKeyCode::Numpad3 => VirtualKeyCode::Numpad3,
            GlutinVirtualKeyCode::Numpad4 => VirtualKeyCode::Numpad4,
            GlutinVirtualKeyCode::Numpad5 => VirtualKeyCode::Numpad5,
            GlutinVirtualKeyCode::Numpad6 => VirtualKeyCode::Numpad6,
            GlutinVirtualKeyCode::Numpad7 => VirtualKeyCode::Numpad7,
            GlutinVirtualKeyCode::Numpad8 => VirtualKeyCode::Numpad8,
            GlutinVirtualKeyCode::Numpad9 => VirtualKeyCode::Numpad9,
            GlutinVirtualKeyCode::NumpadAdd => VirtualKeyCode::NumpadAdd,
            GlutinVirtualKeyCode::NumpadDivide => VirtualKeyCode::NumpadDivide,
            GlutinVirtualKeyCode::NumpadDecimal => VirtualKeyCode::NumpadDecimal,
            GlutinVirtualKeyCode::NumpadComma => VirtualKeyCode::NumpadComma,
            GlutinVirtualKeyCode::NumpadEnter => VirtualKeyCode::NumpadEnter,
            GlutinVirtualKeyCode::NumpadEquals => VirtualKeyCode::NumpadEquals,
            GlutinVirtualKeyCode::NumpadMultiply => VirtualKeyCode::NumpadMultiply,
            GlutinVirtualKeyCode::NumpadSubtract => VirtualKeyCode::NumpadSubtract,
            GlutinVirtualKeyCode::AbntC1 => VirtualKeyCode::AbntC1,
            GlutinVirtualKeyCode::AbntC2 => VirtualKeyCode::AbntC2,
            GlutinVirtualKeyCode::Apostrophe => VirtualKeyCode::Apostrophe,
            GlutinVirtualKeyCode::Apps => VirtualKeyCode::Apps,
            GlutinVirtualKeyCode::Asterisk => VirtualKeyCode::Asterisk,
            GlutinVirtualKeyCode::At => VirtualKeyCode::At,
            GlutinVirtualKeyCode::Ax => VirtualKeyCode::Ax,
            GlutinVirtualKeyCode::Backslash => VirtualKeyCode::Backslash,
            GlutinVirtualKeyCode::Calculator => VirtualKeyCode::Calculator,
            GlutinVirtualKeyCode::Capital => VirtualKeyCode::Capital,
            GlutinVirtualKeyCode::Colon => VirtualKeyCode::Colon,
            GlutinVirtualKeyCode::Comma => VirtualKeyCode::Comma,
            GlutinVirtualKeyCode::Convert => VirtualKeyCode::Convert,
            GlutinVirtualKeyCode::Equals => VirtualKeyCode::Equals,
            GlutinVirtualKeyCode::Grave => VirtualKeyCode::Grave,
            GlutinVirtualKeyCode::Kana => VirtualKeyCode::Kana,
            GlutinVirtualKeyCode::Kanji => VirtualKeyCode::Kanji,
            GlutinVirtualKeyCode::LAlt => VirtualKeyCode::LAlt,
            GlutinVirtualKeyCode::LBracket => VirtualKeyCode::LBracket,
            GlutinVirtualKeyCode::LControl => VirtualKeyCode::LControl,
            GlutinVirtualKeyCode::LShift => VirtualKeyCode::LShift,
            GlutinVirtualKeyCode::LWin => VirtualKeyCode::LWin,
            GlutinVirtualKeyCode::Mail => VirtualKeyCode::Mail,
            GlutinVirtualKeyCode::MediaSelect => VirtualKeyCode::MediaSelect,
            GlutinVirtualKeyCode::MediaStop => VirtualKeyCode::MediaStop,
            GlutinVirtualKeyCode::Minus => VirtualKeyCode::Minus,
            GlutinVirtualKeyCode::Mute => VirtualKeyCode::Mute,
            GlutinVirtualKeyCode::MyComputer => VirtualKeyCode::MyComputer,
            GlutinVirtualKeyCode::NavigateForward => VirtualKeyCode::NavigateForward,
            GlutinVirtualKeyCode::NavigateBackward => VirtualKeyCode::NavigateBackward,
            GlutinVirtualKeyCode::NextTrack => VirtualKeyCode::NextTrack,
            GlutinVirtualKeyCode::NoConvert => VirtualKeyCode::NoConvert,
            GlutinVirtualKeyCode::OEM102 => VirtualKeyCode::OEM102,
            GlutinVirtualKeyCode::Period => VirtualKeyCode::Period,
            GlutinVirtualKeyCode::PlayPause => VirtualKeyCode::PlayPause,
            GlutinVirtualKeyCode::Plus => VirtualKeyCode::Plus,
            GlutinVirtualKeyCode::Power => VirtualKeyCode::Power,
            GlutinVirtualKeyCode::PrevTrack => VirtualKeyCode::PrevTrack,
            GlutinVirtualKeyCode::RAlt => VirtualKeyCode::RAlt,
            GlutinVirtualKeyCode::RBracket => VirtualKeyCode::RBracket,
            GlutinVirtualKeyCode::RControl => VirtualKeyCode::RControl,
            GlutinVirtualKeyCode::RShift => VirtualKeyCode::RShift,
            GlutinVirtualKeyCode::RWin => VirtualKeyCode::RWin,
            GlutinVirtualKeyCode::Semicolon => VirtualKeyCode::Semicolon,
            GlutinVirtualKeyCode::Slash => VirtualKeyCode::Slash,
            GlutinVirtualKeyCode::Sleep => VirtualKeyCode::Sleep,
            GlutinVirtualKeyCode::Stop => VirtualKeyCode::Stop,
            GlutinVirtualKeyCode::Sysrq => VirtualKeyCode::Sysrq,
            GlutinVirtualKeyCode::Tab => VirtualKeyCode::Tab,
            GlutinVirtualKeyCode::Underline => VirtualKeyCode::Underline,
            GlutinVirtualKeyCode::Unlabeled => VirtualKeyCode::Unlabeled,
            GlutinVirtualKeyCode::VolumeDown => VirtualKeyCode::VolumeDown,
            GlutinVirtualKeyCode::VolumeUp => VirtualKeyCode::VolumeUp,
            GlutinVirtualKeyCode::Wake => VirtualKeyCode::Wake,
            GlutinVirtualKeyCode::WebBack => VirtualKeyCode::WebBack,
            GlutinVirtualKeyCode::WebFavorites => VirtualKeyCode::WebFavorites,
            GlutinVirtualKeyCode::WebForward => VirtualKeyCode::WebForward,
            GlutinVirtualKeyCode::WebHome => VirtualKeyCode::WebHome,
            GlutinVirtualKeyCode::WebRefresh => VirtualKeyCode::WebRefresh,
            GlutinVirtualKeyCode::WebSearch => VirtualKeyCode::WebSearch,
            GlutinVirtualKeyCode::WebStop => VirtualKeyCode::WebStop,
            GlutinVirtualKeyCode::Yen => VirtualKeyCode::Yen,
            GlutinVirtualKeyCode::Copy => VirtualKeyCode::Copy,
            GlutinVirtualKeyCode::Paste => VirtualKeyCode::Paste,
            GlutinVirtualKeyCode::Cut => VirtualKeyCode::Cut
        }*/
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
