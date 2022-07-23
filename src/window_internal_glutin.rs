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
use std::rc::Rc;

use glutin::dpi::{LogicalSize, PhysicalPosition, PhysicalSize};
use glutin::event::{
    ElementState as GlutinElementState,
    Event as GlutinEvent,
    MouseScrollDelta as GlutinMouseScrollDelta,
    TouchPhase,
    VirtualKeyCode as GlutinVirtualKeyCode,
    WindowEvent as GlutinWindowEvent
};
use glutin::event_loop::{ControlFlow, EventLoop, EventLoopClosed, EventLoopProxy};
use glutin::monitor::MonitorHandle;
use glutin::window::{
    Icon,
    Window as GlutinWindow,
    WindowBuilder as GlutinWindowBuilder
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
    window_context: Rc<glutin::ContextWrapper<glutin::PossiblyCurrent, GlutinWindow>>,
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
        context: &Rc<glutin::ContextWrapper<glutin::PossiblyCurrent, GlutinWindow>>,
        event_proxy: EventLoopProxy<UserEventGlutin<UserEventType>>,
        initial_physical_size: UVec2
    ) -> Self
    {
        WindowHelperGlutin {
            window_context: context.clone(),
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
        self.window_context.window().set_window_icon(Some(
            Icon::from_rgba(data, size.x, size.y).map_err(|err| {
                ErrorMessage::msg_with_cause("Icon data was invalid", err)
            })?
        ));

        Ok(())
    }

    pub fn set_cursor_visible(&self, visible: bool)
    {
        self.window_context.window().set_cursor_visible(visible);
    }

    pub fn set_cursor_grab(
        &self,
        grabbed: bool
    ) -> Result<(), BacktraceError<ErrorMessage>>
    {
        let central_position = self.physical_size / 2;
        self.window_context
            .window()
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

        match self.window_context.window().set_cursor_grab(grabbed) {
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
        self.window_context.window().set_resizable(resizable);
    }

    #[inline]
    pub fn request_redraw(&self)
    {
        self.redraw_requested.set(true);
    }

    pub fn set_title(&self, title: &str)
    {
        self.window_context.window().set_title(title);
    }

    pub fn set_fullscreen_mode(&self, mode: WindowFullscreenMode)
    {
        let window = self.window_context.window();

        window.set_fullscreen(match mode {
            WindowFullscreenMode::Windowed => None,
            WindowFullscreenMode::FullscreenBorderless => {
                Some(glutin::window::Fullscreen::Borderless(None))
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

        self.window_context
            .window()
            .set_inner_size(glutin::dpi::PhysicalSize::new(size.x, size.y));
    }

    pub fn set_size_scaled_pixels<S: Into<Vec2>>(&self, size: S)
    {
        let size = size.into();

        self.window_context
            .window()
            .set_inner_size(glutin::dpi::LogicalSize::new(size.x, size.y));
    }

    pub fn set_position_pixels<P: Into<IVec2>>(&self, position: P)
    {
        let position = position.into();

        self.window_context.window().set_outer_position(
            glutin::dpi::PhysicalPosition::new(position.x, position.y)
        );
    }

    pub fn set_position_scaled_pixels<P: Into<Vec2>>(&self, position: P)
    {
        let position = position.into();

        self.window_context.window().set_outer_position(
            glutin::dpi::LogicalPosition::new(position.x, position.y)
        );
    }

    #[inline]
    #[must_use]
    pub fn get_scale_factor(&self) -> f64
    {
        self.window_context.window().scale_factor()
    }

    pub fn create_user_event_sender(&self) -> UserEventSender<UserEventType>
    {
        UserEventSender::new(UserEventSenderGlutin::new(self.event_proxy.clone()))
    }
}

pub(crate) struct WindowGlutin<UserEventType: 'static>
{
    event_loop: EventLoop<UserEventGlutin<UserEventType>>,
    window_context: Rc<glutin::ContextWrapper<glutin::PossiblyCurrent, GlutinWindow>>,
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
            EventLoop::with_user_event();

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

        let mut window_builder = GlutinWindowBuilder::new()
            .with_title(title)
            .with_resizable(options.resizable)
            .with_always_on_top(options.always_on_top)
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
                window_builder = window_builder.with_fullscreen(Option::Some(
                    glutin::window::Fullscreen::Borderless(Option::Some(
                        primary_monitor.clone()
                    ))
                ));
            }
        }

        let window_context = create_best_context(&window_builder, &event_loop, &options)
            .ok_or_else(|| {
                BacktraceError::new(WindowCreationError::SuitableContextNotFound)
            })?;

        let window_context = Rc::new(match unsafe { window_context.make_current() } {
            Ok(window_context) => window_context,
            Err((_, err)) => {
                return Err(BacktraceError::new_with_cause(
                    WindowCreationError::MakeContextCurrentFailed,
                    err
                ));
            }
        });

        if let WindowCreationMode::Windowed {
            position: Some(position),
            ..
        } = &options.mode
        {
            position_window(&primary_monitor, window_context.window(), position);
        }

        // Show window after positioning to avoid the window jumping around
        window_context.window().set_visible(true);

        // Set the position again to work around an issue on Linux
        if let WindowCreationMode::Windowed {
            position: Some(position),
            ..
        } = &options.mode
        {
            position_window(&primary_monitor, window_context.window(), position);
        }

        let glow_context = unsafe {
            glow::Context::from_loader_function(|ptr| {
                window_context.get_proc_address(ptr) as *const _
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

        Result::Ok(WindowGlutin {
            event_loop,
            window_context,
            gl_backend
        })
    }

    pub fn create_user_event_sender(&self) -> UserEventSender<UserEventType>
    {
        UserEventSender::new(UserEventSenderGlutin::new(self.event_loop.create_proxy()))
    }

    pub fn get_inner_size_pixels(&self) -> UVec2
    {
        self.window_context.window().inner_size().into()
    }

    fn loop_handle_event<Handler>(
        window_context: &glutin::ContextWrapper<glutin::PossiblyCurrent, GlutinWindow>,
        handler: &mut DrawingWindowHandler<UserEventType, Handler>,
        event: GlutinEvent<UserEventGlutin<UserEventType>>,
        helper: &mut WindowHelper<UserEventType>
    ) -> WindowEventLoopAction
    where
        Handler: WindowHandler<UserEventType> + 'static
    {
        match event {
            GlutinEvent::LoopDestroyed => return WindowEventLoopAction::Exit,

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
                    window_context.resize(physical_size);
                    helper.inner().physical_size = physical_size.into();
                    handler.on_resize(helper, physical_size.into())
                }

                GlutinWindowEvent::CloseRequested => return WindowEventLoopAction::Exit,

                GlutinWindowEvent::CursorMoved { position, .. } => {
                    let position = Vector2::new(position.x, position.y).into_f32();

                    if helper.inner().is_mouse_grabbed.get() {
                        let central_position = helper.inner().physical_size / 2;
                        window_context
                            .window()
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

                GlutinWindowEvent::KeyboardInput { input, .. } => {
                    let virtual_key_code =
                        input.virtual_keycode.map(VirtualKeyCode::from);

                    match input.state {
                        GlutinElementState::Pressed => {
                            handler.on_key_down(helper, virtual_key_code, input.scancode)
                        }
                        GlutinElementState::Released => {
                            handler.on_key_up(helper, virtual_key_code, input.scancode)
                        }
                    }
                }

                GlutinWindowEvent::ReceivedCharacter(character) => {
                    handler.on_keyboard_char(helper, character)
                }

                GlutinWindowEvent::ModifiersChanged(state) => {
                    handler.on_keyboard_modifiers_changed(helper, state.into())
                }

                _ => {}
            },

            GlutinEvent::RedrawRequested(_) => {
                helper.inner().set_redraw_requested(true);
            }

            GlutinEvent::RedrawEventsCleared => {
                if helper.inner().is_redraw_requested() {
                    helper.inner().set_redraw_requested(false);
                    handler.on_draw(helper);
                    window_context.swap_buffers().unwrap();
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
        let window_context = self.window_context.clone();
        let event_loop = self.event_loop;

        let initial_viewport_size_pixels = window_context.window().inner_size().into();

        let mut handler = DrawingWindowHandler::new(handler, renderer);

        let mut helper = WindowHelper::new(WindowHelperGlutin::new(
            &window_context,
            event_loop.create_proxy(),
            initial_viewport_size_pixels
        ));

        handler.on_start(
            &mut helper,
            WindowStartupInfo::new(
                initial_viewport_size_pixels,
                window_context.window().scale_factor()
            )
        );

        match helper.inner().get_event_loop_action() {
            WindowEventLoopAction::Continue => {
                // Do nothing
            }
            WindowEventLoopAction::Exit => {
                log::info!("Start callback requested exit!");
                std::mem::drop(handler);
                std::process::exit(0);
            }
        }

        let mut handler = Option::Some(handler);

        event_loop.run(
            move |event: GlutinEvent<UserEventGlutin<UserEventType>>,
                  _,
                  control_flow: &mut ControlFlow| {
                *control_flow = {
                    if handler.is_none() {
                        ControlFlow::Exit
                    } else {
                        let action = WindowGlutin::loop_handle_event(
                            &window_context,
                            handler.as_mut().unwrap(),
                            event,
                            &mut helper
                        );

                        match action {
                            WindowEventLoopAction::Continue => {
                                if helper.inner().is_redraw_requested() {
                                    ControlFlow::Poll
                                } else {
                                    ControlFlow::Wait
                                }
                            }
                            WindowEventLoopAction::Exit => {
                                handler = Option::None;
                                ControlFlow::Exit
                            }
                        }
                    }
                }
            }
        )
    }

    #[inline]
    #[must_use]
    pub fn gl_backend(&self) -> &Rc<dyn GLBackend>
    {
        &self.gl_backend
    }
}

fn create_best_context<UserEventType>(
    window_builder: &GlutinWindowBuilder,
    event_loop: &EventLoop<UserEventType>,
    options: &WindowCreationOptions
) -> Option<glutin::WindowedContext<glutin::NotCurrent>>
{
    for vsync in &[options.vsync, true, false] {
        for multisampling in &[options.multisampling, 16, 8, 4, 2, 1, 0] {
            log::info!("Trying vsync={}, multisampling={}...", vsync, multisampling);

            let mut windowed_context = glutin::ContextBuilder::new()
                .with_vsync(*vsync)
                .with_gl(glutin::GlRequest::Specific(glutin::Api::OpenGl, (2, 0)));

            if *multisampling > 1 {
                windowed_context = windowed_context.with_multisampling(*multisampling);
            }

            let result =
                windowed_context.build_windowed(window_builder.clone(), event_loop);

            match result {
                Ok(context) => {
                    log::info!("Context created");
                    return Option::Some(context);
                }
                Err(err) => {
                    log::info!("Failed with error: {:?}", err);
                }
            }
        }
    }

    log::error!("Failed to create any context.");
    Option::None
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

impl From<glutin::event::MouseButton> for MouseButton
{
    fn from(button: glutin::event::MouseButton) -> Self
    {
        match button {
            glutin::event::MouseButton::Left => MouseButton::Left,
            glutin::event::MouseButton::Right => MouseButton::Right,
            glutin::event::MouseButton::Middle => MouseButton::Middle,
            glutin::event::MouseButton::Other(id) => MouseButton::Other(id)
        }
    }
}

impl From<GlutinVirtualKeyCode> for VirtualKeyCode
{
    fn from(virtual_key_code: GlutinVirtualKeyCode) -> Self
    {
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
        }
    }
}

impl From<glutin::event::ModifiersState> for ModifiersState
{
    fn from(state: glutin::event::ModifiersState) -> Self
    {
        ModifiersState {
            ctrl: state.ctrl(),
            alt: state.alt(),
            shift: state.shift(),
            logo: state.logo()
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

#[derive(Clone)]
pub struct UserEventSenderGlutin<UserEventType: 'static>
{
    event_proxy: EventLoopProxy<UserEventGlutin<UserEventType>>
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
