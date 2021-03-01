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

use std::ffi::CStr;
use std::fmt::{Display, Formatter};
use std::rc::Rc;

use gl::types::*;
use glutin::dpi::{LogicalSize, PhysicalPosition, PhysicalSize};
use glutin::event::{
    ElementState as GlutinElementState,
    Event as GlutinEvent,
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

use crate::dimen::Vector2;
use crate::error::{BacktraceError, ErrorMessage};
use crate::{GLRenderer, Graphics2D};

/// Error occuring when sending a user event.
#[derive(Clone, Debug, Hash, Eq, PartialEq, Copy)]
pub enum EventLoopSendError
{
    /// Send failed as the event loop no longer exists.
    EventLoopNoLongerExists
}

/// Allows user events to be sent to the event loop from other threads.
#[derive(Clone)]
pub struct UserEventSender<UserEventType: 'static>
{
    event_proxy: EventLoopProxy<UserEventType>
}

impl<UserEventType> UserEventSender<UserEventType>
{
    /// Sends a user-defined event to the event loop. This will cause
    /// [WindowHandler::on_user_event] to be invoked on the event loop
    /// thread.
    ///
    /// This may be invoked from a different thread to the one running the event
    /// loop.
    pub fn send_event(&self, event: UserEventType) -> Result<(), EventLoopSendError>
    {
        self.event_proxy.send_event(event).map_err(|err| match err {
            EventLoopClosed(_) => EventLoopSendError::EventLoopNoLongerExists
        })
    }
}

pub(crate) struct WindowImplHelper<UserEventType: 'static>
{
    window_context: Rc<glutin::ContextWrapper<glutin::PossiblyCurrent, GlutinWindow>>,
    event_proxy: EventLoopProxy<UserEventType>
}

impl<UserEventType> Clone for WindowImplHelper<UserEventType>
{
    fn clone(&self) -> Self
    {
        Self {
            window_context: self.window_context.clone(),
            event_proxy: self.event_proxy.clone()
        }
    }
}

impl<UserEventType> WindowImplHelper<UserEventType>
{
    #[inline]
    fn new(
        context: &Rc<glutin::ContextWrapper<glutin::PossiblyCurrent, GlutinWindow>>,
        event_proxy: EventLoopProxy<UserEventType>
    ) -> Self
    {
        WindowImplHelper {
            window_context: context.clone(),
            event_proxy
        }
    }

    pub fn set_icon_from_rgba_pixels(
        &self,
        data: Vec<u8>,
        size: Vector2<u32>
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
        self.window_context
            .window()
            .set_cursor_grab(grabbed)
            .map_err(|err| ErrorMessage::msg_with_cause("Could not grab cursor", err))
    }

    pub fn set_resizable(&self, resizable: bool)
    {
        self.window_context.window().set_resizable(resizable);
    }

    #[inline]
    pub fn request_redraw(&self)
    {
        self.window_context.window().request_redraw();
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
    }

    pub fn set_size_pixels<S: Into<Vector2<u32>>>(&self, size: S)
    {
        let size = size.into();

        self.window_context
            .window()
            .set_inner_size(glutin::dpi::PhysicalSize::new(size.x, size.y));
    }

    pub fn set_position_pixels<P: Into<Vector2<i32>>>(&self, position: P)
    {
        let position = position.into();

        self.window_context.window().set_outer_position(
            glutin::dpi::PhysicalPosition::new(position.x, position.y)
        );
    }

    pub fn set_size_scaled_pixels<S: Into<Vector2<f32>>>(&self, size: S)
    {
        let size = size.into();

        self.window_context
            .window()
            .set_inner_size(glutin::dpi::LogicalSize::new(size.x, size.y));
    }

    pub fn set_position_scaled_pixels<P: Into<Vector2<f32>>>(&self, position: P)
    {
        let position = position.into();

        self.window_context.window().set_outer_position(
            glutin::dpi::LogicalPosition::new(position.x, position.y)
        );
    }

    #[inline]
    pub fn get_scale_factor(&self) -> f64
    {
        self.window_context.window().scale_factor()
    }

    pub fn create_user_event_sender(&self) -> UserEventSender<UserEventType>
    {
        UserEventSender {
            event_proxy: self.event_proxy.clone()
        }
    }
}

pub(crate) trait WindowImplHandler<UserEventType>
{
    fn on_start(&mut self, info: WindowStartupInfo) -> WindowEventLoopAction;

    fn on_user_event(&mut self, user_event: UserEventType) -> WindowEventLoopAction;

    fn on_resize(&mut self, size_pixels: Vector2<u32>) -> WindowEventLoopAction;

    fn on_scale_factor_changed(&mut self, scale_factor: f64) -> WindowEventLoopAction;

    fn on_draw(&mut self) -> WindowEventLoopAction;

    fn on_mouse_move(&mut self, position: Vector2<f32>) -> WindowEventLoopAction;

    fn on_mouse_button_down(&mut self, button: MouseButton) -> WindowEventLoopAction;

    fn on_mouse_button_up(&mut self, button: MouseButton) -> WindowEventLoopAction;

    fn on_key_down(
        &mut self,
        virtual_key_code: Option<VirtualKeyCode>,
        scancode: KeyScancode
    ) -> WindowEventLoopAction;

    fn on_key_up(
        &mut self,
        virtual_key_code: Option<VirtualKeyCode>,
        scancode: KeyScancode
    ) -> WindowEventLoopAction;

    fn on_keyboard_char(&mut self, unicode_character: char) -> WindowEventLoopAction;

    fn on_keyboard_modifiers_changed(
        &mut self,
        state: ModifiersState
    ) -> WindowEventLoopAction;
}

extern "system" fn gl_log_callback(
    _source: GLenum,
    _gltype: GLenum,
    _id: GLuint,
    severity: GLenum,
    length: GLsizei,
    message: *const GLchar,
    _user_param: *mut std::os::raw::c_void
)
{
    let msg = if length < 0 {
        unsafe { String::from_utf8_lossy(std::ffi::CStr::from_ptr(message).to_bytes()) }
    } else {
        unsafe {
            String::from_utf8_lossy(std::slice::from_raw_parts(
                message as *const u8,
                length as usize
            ))
        }
    };

    match severity {
        gl::DEBUG_SEVERITY_HIGH => log::error!("GL debug log: {}", msg),
        gl::DEBUG_SEVERITY_MEDIUM => log::warn!("GL debug log: {}", msg),
        gl::DEBUG_SEVERITY_LOW => log::info!("GL debug log: {}", msg),
        _ => log::debug!("GL debug log: {}", msg)
    }
}

fn gl_setup_debug_log_callback()
{
    if !gl::DebugMessageCallback::is_loaded() {
        log::error!("Cannot register GL debug log: function not loaded");
        return;
    }

    log::info!("Setting up GL debug log");

    unsafe {
        gl::DebugMessageCallback(Some(gl_log_callback), std::ptr::null());
        gl::Enable(gl::DEBUG_OUTPUT);
        gl::Enable(gl::DEBUG_OUTPUT_SYNCHRONOUS);
    }
}

/// Error occurring when creating a window.
#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq)]
pub enum WindowCreationError
{
    /// Could not find the primary monitor.
    PrimaryMonitorNotFound,
    /// Could not find a suitable graphics context. Speedy2D attempts to find
    /// the best possible context configuration by trying multiple options for
    /// vsync and multisampling.
    SuitableContextNotFound,
    /// Failed to make the graphics context current.
    MakeContextCurrentFailed,
    /// Failed to instantiate the renderer.
    RendererCreationFailed
}

impl Display for WindowCreationError
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result
    {
        match self {
            WindowCreationError::PrimaryMonitorNotFound => {
                f.write_str("Primary monitor not found")
            }
            WindowCreationError::SuitableContextNotFound => {
                f.write_str("Could not find a suitable graphics context")
            }
            WindowCreationError::MakeContextCurrentFailed => {
                f.write_str("Failed to make the graphics context current")
            }
            WindowCreationError::RendererCreationFailed => {
                f.write_str("Failed to create the renderer")
            }
        }
    }
}

pub(crate) struct WindowImpl<UserEventType: 'static>
{
    event_loop: EventLoop<UserEventType>,
    window_context: Rc<glutin::ContextWrapper<glutin::PossiblyCurrent, GlutinWindow>>,
    helper: WindowImplHelper<UserEventType>
}

impl<UserEventType> WindowImpl<UserEventType>
{
    pub(crate) fn new(
        title: &str,
        options: WindowCreationOptions
    ) -> Result<WindowImpl<UserEventType>, BacktraceError<WindowCreationError>>
    {
        let event_loop: EventLoop<UserEventType> = EventLoop::with_user_event();

        let primary_monitor = event_loop.primary_monitor().ok_or_else(|| {
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

        let mut window_builder = GlutinWindowBuilder::new().with_title(title);

        match &options.mode() {
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

        match &options.mode() {
            WindowCreationMode::Windowed { position, .. } => {
                if let Some(position) = position {
                    position_window(&primary_monitor, window_context.window(), position);
                }
            }

            WindowCreationMode::FullscreenBorderless => {
                // Nothing to do
            }
        }

        gl::load_with(|ptr| window_context.get_proc_address(ptr) as *const _);

        let version = unsafe {
            let data = CStr::from_ptr(gl::GetString(gl::VERSION) as *const _).to_bytes();
            String::from_utf8_lossy(data)
        };

        log::info!("Using OpenGL version: {}", version);

        gl_setup_debug_log_callback();

        let helper = WindowImplHelper::new(&window_context, event_loop.create_proxy());

        Result::Ok(WindowImpl {
            event_loop,
            window_context,
            helper
        })
    }

    pub(crate) fn create_user_event_sender(&self) -> UserEventSender<UserEventType>
    {
        UserEventSender {
            event_proxy: self.event_loop.create_proxy()
        }
    }

    pub(crate) fn get_inner_size_pixels(&self) -> Vector2<u32>
    {
        self.window_context.window().inner_size().into()
    }

    fn loop_iter<Handler>(
        window_context: &glutin::ContextWrapper<glutin::PossiblyCurrent, GlutinWindow>,
        handler: &mut Handler,
        event: GlutinEvent<UserEventType>
    ) -> WindowEventLoopAction
    where
        Handler: WindowImplHandler<UserEventType> + 'static
    {
        match event {
            GlutinEvent::LoopDestroyed => WindowEventLoopAction::Exit,

            GlutinEvent::UserEvent(event) => handler.on_user_event(event),

            GlutinEvent::WindowEvent { event, .. } => match event {
                GlutinWindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                    log::info!("Scale factor changed: {:?}", scale_factor);
                    handler.on_scale_factor_changed(scale_factor)
                }

                GlutinWindowEvent::Resized(physical_size) => {
                    log::info!("Resized: {:?}", physical_size);
                    window_context.resize(physical_size);
                    unsafe {
                        gl::Viewport(
                            0,
                            0,
                            physical_size.width as i32,
                            physical_size.height as i32
                        )
                    }
                    handler.on_resize(physical_size.into())
                }

                GlutinWindowEvent::CloseRequested => WindowEventLoopAction::Exit,

                GlutinWindowEvent::CursorMoved { position, .. } => handler
                    .on_mouse_move(Vector2::new(position.x as f32, position.y as f32)),

                GlutinWindowEvent::MouseInput { state, button, .. } => match state {
                    GlutinElementState::Pressed => {
                        handler.on_mouse_button_down(MouseButton::from(button))
                    }
                    GlutinElementState::Released => {
                        handler.on_mouse_button_up(MouseButton::from(button))
                    }
                },

                GlutinWindowEvent::KeyboardInput { input, .. } => {
                    let virtual_key_code =
                        input.virtual_keycode.map(VirtualKeyCode::from);

                    match input.state {
                        GlutinElementState::Pressed => {
                            handler.on_key_down(virtual_key_code, input.scancode)
                        }
                        GlutinElementState::Released => {
                            handler.on_key_up(virtual_key_code, input.scancode)
                        }
                    }
                }

                GlutinWindowEvent::ReceivedCharacter(character) => {
                    handler.on_keyboard_char(character)
                }

                GlutinWindowEvent::ModifiersChanged(state) => {
                    handler.on_keyboard_modifiers_changed(ModifiersState::from(state))
                }

                _ => WindowEventLoopAction::Continue
            },

            GlutinEvent::RedrawRequested(_) => {
                let result = handler.on_draw();
                window_context.swap_buffers().unwrap();
                result
            }

            _ => WindowEventLoopAction::Continue
        }
    }

    pub(crate) fn run_loop<Handler>(self, mut handler: Handler) -> !
    where
        Handler: WindowImplHandler<UserEventType> + 'static
    {
        let window_context = self.window_context;

        match handler.on_start(WindowStartupInfo::new(
            window_context.window().inner_size().into(),
            window_context.window().scale_factor()
        )) {
            WindowEventLoopAction::Continue => {}
            WindowEventLoopAction::Exit => {
                log::info!("Start callback requested exit!");
                std::mem::drop(handler);
                std::process::exit(0);
            }
        }

        let mut handler = Option::Some(handler);

        self.event_loop.run(
            move |event: GlutinEvent<UserEventType>,
                  _,
                  control_flow: &mut ControlFlow| {
                *control_flow = {
                    if handler.is_none() {
                        ControlFlow::Exit
                    } else {
                        match WindowImpl::loop_iter(
                            &window_context,
                            handler.as_mut().unwrap(),
                            event
                        ) {
                            WindowEventLoopAction::Continue => ControlFlow::Wait,
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

    pub(crate) fn helper(&self) -> &WindowImplHelper<UserEventType>
    {
        &self.helper
    }
}

fn create_best_context<UserEventType>(
    window_builder: &GlutinWindowBuilder,
    event_loop: &EventLoop<UserEventType>,
    options: &WindowCreationOptions
) -> Option<glutin::WindowedContext<glutin::NotCurrent>>
{
    for vsync in &[true, false] {
        if *vsync && !options.vsync() {
            continue;
        }

        for multisampling in &[16, 8, 4, 2, 1, 0] {
            if *multisampling > 1 && *multisampling > options.multisampling() {
                continue;
            }

            log::info!("Trying vsync={}, multisampling={}...", vsync, multisampling);

            let mut windowed_context = glutin::ContextBuilder::new()
                .with_vsync(*vsync)
                .with_gl_profile(glutin::GlProfile::Core)
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

/// A set of callbacks for an active window. If a callback is not implemented,
/// it will do nothing by default, so it is only necessary to implement the
/// callbacks you actually need.
pub trait WindowHandler<UserEventType = ()>
{
    /// Invoked once when the window first starts.
    #[allow(unused_variables)]
    #[inline]
    fn on_start(
        &mut self,
        helper: &mut WindowHelper<UserEventType>,
        info: WindowStartupInfo
    )
    {
    }

    /// Invoked when a user-defined event is received, allowing you to wake up
    /// the event loop to handle events from other threads.
    ///
    /// See [WindowHelper::create_user_event_sender].
    #[allow(unused_variables)]
    #[inline]
    fn on_user_event(
        &mut self,
        helper: &mut WindowHelper<UserEventType>,
        user_event: UserEventType
    )
    {
    }

    /// Invoked when the window is resized.
    #[allow(unused_variables)]
    #[inline]
    fn on_resize(
        &mut self,
        helper: &mut WindowHelper<UserEventType>,
        size_pixels: Vector2<u32>
    )
    {
    }

    /// Invoked when the window scale factor changes.
    #[allow(unused_variables)]
    #[inline]
    fn on_scale_factor_changed(
        &mut self,
        helper: &mut WindowHelper<UserEventType>,
        scale_factor: f64
    )
    {
    }

    /// Invoked when the contents of the window needs to be redrawn.
    ///
    /// It is possible to request a redraw from any callback using
    /// [WindowHelper::request_redraw].
    #[allow(unused_variables)]
    #[inline]
    fn on_draw(
        &mut self,
        helper: &mut WindowHelper<UserEventType>,
        graphics: &mut Graphics2D
    )
    {
    }

    /// Invoked when the mouse changes position.
    #[allow(unused_variables)]
    #[inline]
    fn on_mouse_move(
        &mut self,
        helper: &mut WindowHelper<UserEventType>,
        position: Vector2<f32>
    )
    {
    }

    /// Invoked when a mouse button is pressed.
    #[allow(unused_variables)]
    #[inline]
    fn on_mouse_button_down(
        &mut self,
        helper: &mut WindowHelper<UserEventType>,
        button: MouseButton
    )
    {
    }

    /// Invoked when a mouse button is released.
    #[allow(unused_variables)]
    #[inline]
    fn on_mouse_button_up(
        &mut self,
        helper: &mut WindowHelper<UserEventType>,
        button: MouseButton
    )
    {
    }

    /// Invoked when a keyboard key is pressed.
    ///
    /// To detect when a character is typed, see the
    /// [WindowHandler::on_keyboard_char] callback.
    #[allow(unused_variables)]
    #[inline]
    fn on_key_down(
        &mut self,
        helper: &mut WindowHelper<UserEventType>,
        virtual_key_code: Option<VirtualKeyCode>,
        scancode: KeyScancode
    )
    {
    }

    /// Invoked when a keyboard key is released.
    #[allow(unused_variables)]
    #[inline]
    fn on_key_up(
        &mut self,
        helper: &mut WindowHelper<UserEventType>,
        virtual_key_code: Option<VirtualKeyCode>,
        scancode: KeyScancode
    )
    {
    }

    /// Invoked when a character is typed on the keyboard.
    ///
    /// This is invoked in addition to the [WindowHandler::on_key_up] and
    /// [WindowHandler::on_key_down] callbacks.
    #[allow(unused_variables)]
    #[inline]
    fn on_keyboard_char(
        &mut self,
        helper: &mut WindowHelper<UserEventType>,
        unicode_codepoint: char
    )
    {
    }

    /// Invoked when the state of the modifier keys has changed.
    #[allow(unused_variables)]
    #[inline]
    fn on_keyboard_modifiers_changed(
        &mut self,
        helper: &mut WindowHelper<UserEventType>,
        state: ModifiersState
    )
    {
    }
}

pub(crate) struct DrawingWindowHandler<H, UserEventType>
where
    H: WindowHandler<UserEventType>,
    UserEventType: 'static
{
    window_handler: H,
    renderer: GLRenderer,
    helper: WindowHelper<UserEventType>
}

impl<H, UserEventType> DrawingWindowHandler<H, UserEventType>
where
    H: WindowHandler<UserEventType>,
    UserEventType: 'static
{
    pub(crate) fn new(
        window_handler: H,
        renderer: GLRenderer,
        helper: WindowHelper<UserEventType>
    ) -> Self
    {
        DrawingWindowHandler {
            window_handler,
            renderer,
            helper
        }
    }
}

impl<Handler, UserEventType> WindowImplHandler<UserEventType>
    for DrawingWindowHandler<Handler, UserEventType>
where
    Handler: WindowHandler<UserEventType>
{
    #[inline]
    fn on_start(&mut self, info: WindowStartupInfo) -> WindowEventLoopAction
    {
        self.window_handler.on_start(&mut self.helper, info);
        self.helper.get_event_loop_action()
    }

    fn on_user_event(&mut self, user_event: UserEventType) -> WindowEventLoopAction
    {
        self.window_handler
            .on_user_event(&mut self.helper, user_event);
        self.helper.get_event_loop_action()
    }

    #[inline]
    fn on_resize(&mut self, size_pixels: Vector2<u32>) -> WindowEventLoopAction
    {
        self.renderer.set_viewport_size_pixels(size_pixels);
        self.window_handler.on_resize(&mut self.helper, size_pixels);
        self.helper.get_event_loop_action()
    }

    #[inline]
    fn on_scale_factor_changed(&mut self, scale_factor: f64) -> WindowEventLoopAction
    {
        self.window_handler
            .on_scale_factor_changed(&mut self.helper, scale_factor);
        self.helper.get_event_loop_action()
    }

    #[inline]
    fn on_draw(&mut self) -> WindowEventLoopAction
    {
        let renderer = &mut self.renderer;
        let window_handler = &mut self.window_handler;
        let helper = &mut self.helper;

        renderer.draw_frame(|graphics| window_handler.on_draw(helper, graphics));
        self.helper.get_event_loop_action()
    }

    #[inline]
    fn on_mouse_move(&mut self, position: Vector2<f32>) -> WindowEventLoopAction
    {
        self.window_handler
            .on_mouse_move(&mut self.helper, position);
        self.helper.get_event_loop_action()
    }

    #[inline]
    fn on_mouse_button_down(&mut self, button: MouseButton) -> WindowEventLoopAction
    {
        self.window_handler
            .on_mouse_button_down(&mut self.helper, button);
        self.helper.get_event_loop_action()
    }

    #[inline]
    fn on_mouse_button_up(&mut self, button: MouseButton) -> WindowEventLoopAction
    {
        self.window_handler
            .on_mouse_button_up(&mut self.helper, button);
        self.helper.get_event_loop_action()
    }

    #[inline]
    fn on_key_down(
        &mut self,
        virtual_key_code: Option<VirtualKeyCode>,
        scancode: KeyScancode
    ) -> WindowEventLoopAction
    {
        self.window_handler
            .on_key_down(&mut self.helper, virtual_key_code, scancode);
        self.helper.get_event_loop_action()
    }

    #[inline]
    fn on_key_up(
        &mut self,
        virtual_key_code: Option<VirtualKeyCode>,
        scancode: KeyScancode
    ) -> WindowEventLoopAction
    {
        self.window_handler
            .on_key_up(&mut self.helper, virtual_key_code, scancode);
        self.helper.get_event_loop_action()
    }

    #[inline]
    fn on_keyboard_char(&mut self, unicode_codepoint: char) -> WindowEventLoopAction
    {
        self.window_handler
            .on_keyboard_char(&mut self.helper, unicode_codepoint);
        self.helper.get_event_loop_action()
    }

    #[inline]
    fn on_keyboard_modifiers_changed(
        &mut self,
        state: ModifiersState
    ) -> WindowEventLoopAction
    {
        self.window_handler
            .on_keyboard_modifiers_changed(&mut self.helper, state);
        self.helper.get_event_loop_action()
    }
}

/// A set of helper methods to perform actions on a [crate::Window].
pub struct WindowHelper<UserEventType = ()>
where
    UserEventType: 'static
{
    helper: WindowImplHelper<UserEventType>,
    event_loop_action: WindowEventLoopAction
}

impl<UserEventType> WindowHelper<UserEventType>
{
    #[inline]
    fn get_event_loop_action(&self) -> WindowEventLoopAction
    {
        self.event_loop_action
    }

    #[inline]
    pub(crate) fn new(helper: WindowImplHelper<UserEventType>) -> Self
    {
        WindowHelper {
            helper,
            event_loop_action: WindowEventLoopAction::Continue
        }
    }

    /// Causes the event loop to stop processing events, and terminate the
    /// application.
    ///
    /// Note: The event loop will stop only once the current callback has
    /// returned, rather than terminating immediately.
    ///
    /// Once the event loop has stopped, the entire process will end with error
    /// code 0, even if other threads are running.
    ///
    /// If your `WindowHandler` struct implements `Drop`, it will be safely
    /// destructed before exiting.
    ///
    /// No further callbacks will be given once this function has been called.
    pub fn terminate_loop(&mut self)
    {
        self.event_loop_action = WindowEventLoopAction::Exit;
    }

    /// Sets the window icon from the provided RGBA pixels.
    ///
    /// On Windows, the base icon size is 16x16, however a multiple of this
    /// (e.g. 32x32) should be provided for high-resolution displays.
    pub fn set_icon_from_rgba_pixels<S>(
        &self,
        data: Vec<u8>,
        size: S
    ) -> Result<(), BacktraceError<ErrorMessage>>
    where
        S: Into<Vector2<u32>>
    {
        self.helper.set_icon_from_rgba_pixels(data, size.into())
    }

    /// Sets the visibility of the mouse cursor.
    pub fn set_cursor_visible(&self, visible: bool)
    {
        self.helper.set_cursor_visible(visible)
    }

    /// Grabs the cursor, preventing it from leaving the window.
    pub fn set_cursor_grab(
        &self,
        grabbed: bool
    ) -> Result<(), BacktraceError<ErrorMessage>>
    {
        self.helper.set_cursor_grab(grabbed)
    }

    /// Set to false to prevent the user from resizing the window.
    pub fn set_resizable(&self, resizable: bool)
    {
        self.helper.set_resizable(resizable);
    }

    /// Request that the window is redrawn.
    ///
    /// This will cause the [WindowHandler::on_draw] callback to be invoked on
    /// the next frame.
    #[inline]
    pub fn request_redraw(&self)
    {
        self.helper.request_redraw()
    }

    /// Sets the window title.
    pub fn set_title(&self, title: &str)
    {
        self.helper.set_title(title);
    }

    /// Sets the window fullscreen mode.
    pub fn set_fullscreen_mode(&self, mode: WindowFullscreenMode)
    {
        self.helper.set_fullscreen_mode(mode)
    }

    /// Sets the window size in pixels. This is the window's inner size,
    /// excluding the border.
    pub fn set_size_pixels<S: Into<Vector2<u32>>>(&self, size: S)
    {
        self.helper.set_size_pixels(size)
    }

    /// Sets the position of the window in pixels. If multiple monitors are in
    /// use, this will be the distance from the top left of the display
    /// area, spanning all the monitors.
    pub fn set_position_pixels<P: Into<Vector2<i32>>>(&self, position: P)
    {
        self.helper.set_position_pixels(position)
    }

    /// Sets the window size in scaled device-independent pixels. This is the
    /// window's inner size, excluding the border.
    pub fn set_size_scaled_pixels<S: Into<Vector2<f32>>>(&self, size: S)
    {
        self.helper.set_size_scaled_pixels(size)
    }

    /// Sets the position of the window in scaled device-independent pixels. If
    /// multiple monitors are in use, this will be the distance from the top
    /// left of the display area, spanning all the monitors.
    pub fn set_position_scaled_pixels<P: Into<Vector2<f32>>>(&self, position: P)
    {
        self.helper.set_position_scaled_pixels(position)
    }

    /// Gets the window's scale factor.
    #[inline]
    pub fn get_scale_factor(&self) -> f64
    {
        self.helper.get_scale_factor()
    }

    /// Creates a [UserEventSender], which can be used to post custom events to
    /// this event loop from another thread.
    ///
    /// See [UserEventSender::send_event], [WindowHandler::on_user_event].
    pub fn create_user_event_sender(&self) -> UserEventSender<UserEventType>
    {
        self.helper.create_user_event_sender()
    }
}

impl From<PhysicalSize<u32>> for Vector2<u32>
{
    fn from(value: PhysicalSize<u32>) -> Self
    {
        Self::new(value.width, value.height)
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
#[must_use]
pub(crate) enum WindowEventLoopAction
{
    /// Continue running the event loop.
    Continue,

    /// Stops the event loop. This will cause the entire process to end with
    /// error code 0, even if other threads are running.
    ///
    /// No further callbacks will be given once a handler has returned this
    /// value. The handler itself will be dropped before exiting.
    Exit
}

/// Information about the starting state of the window.
#[derive(Debug, PartialEq, Clone)]
pub struct WindowStartupInfo
{
    viewport_size_pixels: Vector2<u32>,
    scale_factor: f64
}

impl WindowStartupInfo
{
    pub(crate) fn new(viewport_size_pixels: Vector2<u32>, scale_factor: f64) -> Self
    {
        WindowStartupInfo {
            viewport_size_pixels,
            scale_factor
        }
    }

    /// The scale factor of the window. When a high-dpi display is in use,
    /// this will be greater than `1.0`.
    pub fn scale_factor(&self) -> f64
    {
        self.scale_factor
    }

    /// The size of the viewport in pixels.
    pub fn viewport_size_pixels(&self) -> &Vector2<u32>
    {
        &self.viewport_size_pixels
    }
}

/// Identifies a mouse button.
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum MouseButton
{
    /// The left mouse button.
    Left,
    /// The middle mouse button.
    Middle,
    /// The right mouse button.
    Right,
    /// Another mouse button, identified by a number.
    Other(u16)
}

/// A virtual key code.
#[allow(missing_docs)]
#[derive(Debug, Hash, Ord, PartialOrd, PartialEq, Eq, Clone, Copy)]
pub enum VirtualKeyCode
{
    Key1,
    Key2,
    Key3,
    Key4,
    Key5,
    Key6,
    Key7,
    Key8,
    Key9,
    Key0,

    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,

    Escape,

    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    F13,
    F14,
    F15,
    F16,
    F17,
    F18,
    F19,
    F20,
    F21,
    F22,
    F23,
    F24,

    PrintScreen,
    ScrollLock,
    PauseBreak,

    Insert,
    Home,
    Delete,
    End,
    PageDown,
    PageUp,

    Left,
    Up,
    Right,
    Down,

    Backspace,
    Return,
    Space,

    Compose,

    Caret,

    Numlock,
    Numpad0,
    Numpad1,
    Numpad2,
    Numpad3,
    Numpad4,
    Numpad5,
    Numpad6,
    Numpad7,
    Numpad8,
    Numpad9,
    NumpadAdd,
    NumpadDivide,
    NumpadDecimal,
    NumpadComma,
    NumpadEnter,
    NumpadEquals,
    NumpadMultiply,
    NumpadSubtract,

    AbntC1,
    AbntC2,
    Apostrophe,
    Apps,
    Asterisk,
    At,
    Ax,
    Backslash,
    Calculator,
    Capital,
    Colon,
    Comma,
    Convert,
    Equals,
    Grave,
    Kana,
    Kanji,
    LAlt,
    LBracket,
    LControl,
    LShift,
    LWin,
    Mail,
    MediaSelect,
    MediaStop,
    Minus,
    Mute,
    MyComputer,
    NavigateForward,
    NavigateBackward,
    NextTrack,
    NoConvert,
    OEM102,
    Period,
    PlayPause,
    Plus,
    Power,
    PrevTrack,
    RAlt,
    RBracket,
    RControl,
    RShift,
    RWin,
    Semicolon,
    Slash,
    Sleep,
    Stop,
    Sysrq,
    Tab,
    Underline,
    Unlabeled,
    VolumeDown,
    VolumeUp,
    Wake,
    WebBack,
    WebFavorites,
    WebForward,
    WebHome,
    WebRefresh,
    WebSearch,
    WebStop,
    Yen,
    Copy,
    Paste,
    Cut
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

/// The state of the modifier keys.
#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct ModifiersState
{
    ctrl: bool,
    alt: bool,
    shift: bool,
    logo: bool
}

impl ModifiersState
{
    /// This is true if the CTRL key is pressed.
    #[inline]
    #[must_use]
    pub fn ctrl(&self) -> bool
    {
        self.ctrl
    }

    /// This is true if the ALT key is pressed.
    #[inline]
    #[must_use]
    pub fn alt(&self) -> bool
    {
        self.alt
    }

    /// This is true if the SHIFT key is pressed.
    #[inline]
    #[must_use]
    pub fn shift(&self) -> bool
    {
        self.shift
    }

    /// This is true if the logo key is pressed (normally the Windows key).
    #[inline]
    #[must_use]
    pub fn logo(&self) -> bool
    {
        self.logo
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

/// Configuration options about the mode in which the window should be created,
/// for example fullscreen or windowed.
#[derive(Debug, PartialEq, Clone)]
enum WindowCreationMode
{
    /// Create the window in non-fullscreen mode.
    Windowed
    {
        /// The size of the window.
        size: WindowSize,

        /// The position of the window.
        position: Option<WindowPosition>
    },

    /// Create the window in fullscreen borderless mode.
    FullscreenBorderless
}

/// The size of the window to create.
#[derive(Debug, PartialEq, Clone)]
pub enum WindowSize
{
    /// Define the window size in pixels.
    PhysicalPixels(Vector2<u32>),
    /// Define the window size in device-independent scaled pixels.
    ScaledPixels(Vector2<f32>),
    /// Make the window fill the screen, except for a margin around the outer
    /// edges.
    MarginPhysicalPixels(u32),
    /// Make the window fill the screen, except for a margin around the outer
    /// edges.
    MarginScaledPixels(f32)
}

/// The position of the window to create.
#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub enum WindowPosition
{
    /// Place the window in the center of the primary monitor.
    Center,
    /// Place the window at the specified pixel location from the top left of
    /// the primary monitor.
    PrimaryMonitorPixelsFromTopLeft(Vector2<i32>)
}

/// Whether or not the window is in fullscreen mode.
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum WindowFullscreenMode
{
    /// Non-fullscreen mode.
    Windowed,
    /// Fullscreen borderless mode.
    FullscreenBorderless
}

/// Options used during the creation of a window.
#[derive(Debug, Clone, PartialEq)]
pub struct WindowCreationOptions
{
    mode: WindowCreationMode,
    multisampling: u16,
    vsync: bool
}

impl WindowCreationOptions
{
    /// Instantiates a new `WindowCreationOptions` structure with the default
    /// options, in non-fullscreen mode.
    pub fn new_windowed(size: WindowSize, position: Option<WindowPosition>) -> Self
    {
        Self::new(WindowCreationMode::Windowed { size, position })
    }

    /// Instantiates a new `WindowCreationOptions` structure with the default
    /// options, in borderless fullscreen mode.
    #[inline]
    #[must_use]
    pub fn new_fullscreen_borderless() -> Self
    {
        Self::new(WindowCreationMode::FullscreenBorderless)
    }

    #[inline]
    #[must_use]
    fn new(mode: WindowCreationMode) -> Self
    {
        WindowCreationOptions {
            mode,
            multisampling: 8,
            vsync: true
        }
    }

    /// Sets the maximum level of multisampling which will be applied. By
    /// default this is set to `8`.
    ///
    /// Note that this depends on platform support, and setting this may have no
    /// effect.
    #[inline]
    #[must_use]
    pub fn with_multisampling(mut self, multisampling: u16) -> Self
    {
        self.multisampling = multisampling;
        self
    }

    /// Sets whether or not vsync should be enabled. This can increase latency,
    /// but should eliminate tearing. By default this is set to `true`.
    ///
    /// Note that this depends on platform support, and setting this may have no
    /// effect.
    #[inline]
    #[must_use]
    pub fn with_vsync(mut self, vsync: bool) -> Self
    {
        self.vsync = vsync;
        self
    }

    #[inline]
    #[must_use]
    fn mode(&self) -> &WindowCreationMode
    {
        &self.mode
    }

    #[inline]
    #[must_use]
    fn multisampling(&self) -> u16
    {
        self.multisampling
    }

    #[inline]
    #[must_use]
    fn vsync(&self) -> bool
    {
        self.vsync
    }
}

/// Type representing a keyboard scancode.
pub type KeyScancode = u32;
