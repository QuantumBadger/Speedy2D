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

use std::fmt::{Display, Formatter};
use std::marker::PhantomData;

use crate::dimen::{IVec2, UVec2, Vec2};
use crate::error::{BacktraceError, ErrorMessage};
use crate::{GLRenderer, Graphics2D};

#[cfg(all(not(target_arch = "wasm32"), not(any(doc, doctest))))]
type WindowHelperInnerType<UserEventType> =
    crate::window_internal_glutin::WindowHelperGlutin<UserEventType>;

#[cfg(all(not(target_arch = "wasm32"), not(any(doc, doctest))))]
type UserEventSenderInnerType<UserEventType> =
    crate::window_internal_glutin::UserEventSenderGlutin<UserEventType>;

#[cfg(all(target_arch = "wasm32", not(any(doc, doctest))))]
type WindowHelperInnerType<UserEventType> =
    crate::window_internal_web::WindowHelperWeb<UserEventType>;

#[cfg(all(target_arch = "wasm32", not(any(doc, doctest))))]
type UserEventSenderInnerType<UserEventType> =
    crate::window_internal_web::UserEventSenderWeb<UserEventType>;

#[cfg(any(doc, doctest))]
type WindowHelperInnerType<UserEventType> = PhantomData<UserEventType>;

#[cfg(any(doc, doctest))]
type UserEventSenderInnerType<UserEventType> = PhantomData<UserEventType>;

/// Error occurring when sending a user event.
#[derive(Clone, Debug, Hash, Eq, PartialEq, Copy)]
pub enum EventLoopSendError
{
    /// Send failed as the event loop no longer exists.
    EventLoopNoLongerExists
}

/// Allows user events to be sent to the event loop from other threads.
pub struct UserEventSender<UserEventType: 'static>
{
    inner: UserEventSenderInnerType<UserEventType>
}

impl<UserEventType> Clone for UserEventSender<UserEventType>
{
    fn clone(&self) -> Self
    {
        UserEventSender {
            inner: self.inner.clone()
        }
    }
}

impl<UserEventType> UserEventSender<UserEventType>
{
    pub(crate) fn new(inner: UserEventSenderInnerType<UserEventType>) -> Self
    {
        Self { inner }
    }

    /// Sends a user-defined event to the event loop. This will cause
    /// [WindowHandler::on_user_event] to be invoked on the event loop
    /// thread.
    ///
    /// This may be invoked from a different thread to the one running the event
    /// loop.
    #[inline]
    pub fn send_event(&self, event: UserEventType) -> Result<(), EventLoopSendError>
    {
        self.inner.send_event(event)
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
    fn on_resize(&mut self, helper: &mut WindowHelper<UserEventType>, size_pixels: UVec2)
    {
    }

    /// Invoked if the mouse cursor becomes grabbed or un-grabbed. See
    /// [WindowHelper::set_cursor_grab].
    ///
    /// Note: mouse movement events will behave differently depending on the
    /// current cursor grabbing status.
    #[allow(unused_variables)]
    #[inline]
    fn on_mouse_grab_status_changed(
        &mut self,
        helper: &mut WindowHelper<UserEventType>,
        mouse_grabbed: bool
    )
    {
    }

    /// Invoked if the window enters or exits fullscreen mode. See
    /// [WindowHelper::set_fullscreen_mode].
    #[allow(unused_variables)]
    #[inline]
    fn on_fullscreen_status_changed(
        &mut self,
        helper: &mut WindowHelper<UserEventType>,
        fullscreen: bool
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
    ///
    /// Normally, this provides the absolute  position of the mouse in the
    /// window/canvas. However, if the mouse cursor is grabbed, this will
    /// instead provide the amount of relative movement since the last move
    /// event.
    ///
    /// See [WindowHandler::on_mouse_grab_status_changed].
    #[allow(unused_variables)]
    #[inline]
    fn on_mouse_move(&mut self, helper: &mut WindowHelper<UserEventType>, position: Vec2)
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

    /// Invoked when the mouse wheel moves.
    #[allow(unused_variables)]
    #[inline]
    fn on_mouse_wheel_scroll(
        &mut self,
        helper: &mut WindowHelper<UserEventType>,
        distance: MouseScrollDistance
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

pub(crate) struct DrawingWindowHandler<UserEventType, H>
where
    UserEventType: 'static,
    H: WindowHandler<UserEventType>
{
    window_handler: H,
    renderer: GLRenderer,
    phantom: PhantomData<UserEventType>
}

impl<UserEventType, H> DrawingWindowHandler<UserEventType, H>
where
    H: WindowHandler<UserEventType>,
    UserEventType: 'static
{
    pub fn new(window_handler: H, renderer: GLRenderer) -> Self
    {
        DrawingWindowHandler {
            window_handler,
            renderer,
            phantom: PhantomData
        }
    }

    #[inline]
    pub fn on_start(
        &mut self,
        helper: &mut WindowHelper<UserEventType>,
        info: WindowStartupInfo
    )
    {
        self.window_handler.on_start(helper, info);
    }

    #[inline]
    pub fn on_user_event(
        &mut self,
        helper: &mut WindowHelper<UserEventType>,
        user_event: UserEventType
    )
    {
        self.window_handler.on_user_event(helper, user_event)
    }

    #[inline]
    pub fn on_resize(
        &mut self,
        helper: &mut WindowHelper<UserEventType>,
        size_pixels: UVec2
    )
    {
        self.renderer.set_viewport_size_pixels(size_pixels);
        self.window_handler.on_resize(helper, size_pixels)
    }

    #[inline]
    pub fn on_mouse_grab_status_changed(
        &mut self,
        helper: &mut WindowHelper<UserEventType>,
        mouse_grabbed: bool
    )
    {
        self.window_handler
            .on_mouse_grab_status_changed(helper, mouse_grabbed)
    }

    #[inline]
    pub fn on_fullscreen_status_changed(
        &mut self,
        helper: &mut WindowHelper<UserEventType>,
        fullscreen: bool
    )
    {
        self.window_handler
            .on_fullscreen_status_changed(helper, fullscreen)
    }

    #[inline]
    pub fn on_scale_factor_changed(
        &mut self,
        helper: &mut WindowHelper<UserEventType>,
        scale_factor: f64
    )
    {
        self.window_handler
            .on_scale_factor_changed(helper, scale_factor)
    }

    #[inline]
    pub fn on_draw(&mut self, helper: &mut WindowHelper<UserEventType>)
    {
        let renderer = &mut self.renderer;
        let window_handler = &mut self.window_handler;

        renderer.draw_frame(|graphics| window_handler.on_draw(helper, graphics))
    }

    #[inline]
    pub fn on_mouse_move(
        &mut self,
        helper: &mut WindowHelper<UserEventType>,
        position: Vec2
    )
    {
        self.window_handler.on_mouse_move(helper, position)
    }

    #[inline]
    pub fn on_mouse_button_down(
        &mut self,
        helper: &mut WindowHelper<UserEventType>,
        button: MouseButton
    )
    {
        self.window_handler.on_mouse_button_down(helper, button)
    }

    #[inline]
    pub fn on_mouse_button_up(
        &mut self,
        helper: &mut WindowHelper<UserEventType>,
        button: MouseButton
    )
    {
        self.window_handler.on_mouse_button_up(helper, button)
    }

    #[inline]
    pub fn on_mouse_wheel_scroll(
        &mut self,
        helper: &mut WindowHelper<UserEventType>,
        distance: MouseScrollDistance
    )
    {
        self.window_handler.on_mouse_wheel_scroll(helper, distance)
    }

    #[inline]
    pub fn on_key_down(
        &mut self,
        helper: &mut WindowHelper<UserEventType>,
        virtual_key_code: Option<VirtualKeyCode>,
        scancode: KeyScancode
    )
    {
        self.window_handler
            .on_key_down(helper, virtual_key_code, scancode)
    }

    #[inline]
    pub fn on_key_up(
        &mut self,
        helper: &mut WindowHelper<UserEventType>,
        virtual_key_code: Option<VirtualKeyCode>,
        scancode: KeyScancode
    )
    {
        self.window_handler
            .on_key_up(helper, virtual_key_code, scancode)
    }

    #[inline]
    pub fn on_keyboard_char(
        &mut self,
        helper: &mut WindowHelper<UserEventType>,
        unicode_codepoint: char
    )
    {
        self.window_handler
            .on_keyboard_char(helper, unicode_codepoint)
    }

    #[inline]
    pub fn on_keyboard_modifiers_changed(
        &mut self,
        helper: &mut WindowHelper<UserEventType>,
        state: ModifiersState
    )
    {
        self.window_handler
            .on_keyboard_modifiers_changed(helper, state)
    }
}

/// A set of helper methods to perform actions on a [crate::Window].
pub struct WindowHelper<UserEventType = ()>
where
    UserEventType: 'static
{
    inner: WindowHelperInnerType<UserEventType>
}

impl<UserEventType> WindowHelper<UserEventType>
{
    pub(crate) fn new(inner: WindowHelperInnerType<UserEventType>) -> Self
    {
        WindowHelper { inner }
    }

    #[inline]
    #[must_use]
    pub(crate) fn inner(&mut self) -> &mut WindowHelperInnerType<UserEventType>
    {
        &mut self.inner
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
        self.inner.terminate_loop()
    }

    /// Sets the window icon from the provided RGBA pixels.
    ///
    /// On Windows, the base icon size is 16x16, however a multiple of this
    /// (e.g. 32x32) should be provided for high-resolution displays.
    ///
    /// For `WebCanvas`, this function has no effect.
    pub fn set_icon_from_rgba_pixels<S>(
        &self,
        data: Vec<u8>,
        size: S
    ) -> Result<(), BacktraceError<ErrorMessage>>
    where
        S: Into<UVec2>
    {
        self.inner.set_icon_from_rgba_pixels(data, size.into())
    }

    /// Sets the visibility of the mouse cursor.
    pub fn set_cursor_visible(&self, visible: bool)
    {
        self.inner.set_cursor_visible(visible)
    }

    /// Grabs the cursor, preventing it from leaving the window.
    pub fn set_cursor_grab(
        &self,
        grabbed: bool
    ) -> Result<(), BacktraceError<ErrorMessage>>
    {
        self.inner.set_cursor_grab(grabbed)
    }

    /// Set to false to prevent the user from resizing the window.
    ///
    /// For `WebCanvas`, this function has no effect.
    pub fn set_resizable(&self, resizable: bool)
    {
        self.inner.set_resizable(resizable)
    }

    /// Request that the window is redrawn.
    ///
    /// This will cause the [WindowHandler::on_draw] callback to be invoked on
    /// the next frame.
    #[inline]
    pub fn request_redraw(&self)
    {
        self.inner.request_redraw()
    }

    /// Sets the window title.
    pub fn set_title<S: AsRef<str>>(&self, title: S)
    {
        self.inner.set_title(title.as_ref())
    }

    /// Sets the window fullscreen mode.
    ///
    /// When using a web canvas, permission for this operation may be denied,
    /// depending on where this is called from, and the user's browser settings.
    /// If the operation is successful, the
    /// [WindowHandler::on_fullscreen_status_changed] callback will be invoked.
    pub fn set_fullscreen_mode(&self, mode: WindowFullscreenMode)
    {
        self.inner.set_fullscreen_mode(mode)
    }

    /// Sets the window size in pixels. This is the window's inner size,
    /// excluding the border.
    ///
    /// For `WebCanvas`, this function has no effect.
    pub fn set_size_pixels<S: Into<UVec2>>(&self, size: S)
    {
        self.inner.set_size_pixels(size)
    }

    /// Gets the window size in pixels.
    pub fn get_size_pixels(&self) -> UVec2
    {
        self.inner.get_size_pixels()
    }

    /// Sets the position of the window in pixels. If multiple monitors are in
    /// use, this will be the distance from the top left of the display
    /// area, spanning all the monitors.
    ///
    /// For `WebCanvas`, this function has no effect.
    pub fn set_position_pixels<P: Into<IVec2>>(&self, position: P)
    {
        self.inner.set_position_pixels(position)
    }

    /// Sets the window size in scaled device-independent pixels. This is the
    /// window's inner size, excluding the border.
    ///
    /// For `WebCanvas`, this function has no effect.
    pub fn set_size_scaled_pixels<S: Into<Vec2>>(&self, size: S)
    {
        self.inner.set_size_scaled_pixels(size)
    }

    /// Sets the position of the window in scaled device-independent pixels. If
    /// multiple monitors are in use, this will be the distance from the top
    /// left of the display area, spanning all the monitors.
    ///
    /// For `WebCanvas`, this function has no effect.
    pub fn set_position_scaled_pixels<P: Into<Vec2>>(&self, position: P)
    {
        self.inner.set_position_scaled_pixels(position)
    }

    /// Gets the window's scale factor.
    #[inline]
    #[must_use]
    pub fn get_scale_factor(&self) -> f64
    {
        self.inner.get_scale_factor()
    }

    /// Creates a [UserEventSender], which can be used to post custom events to
    /// this event loop from another thread.
    ///
    /// See [UserEventSender::send_event], [WindowHandler::on_user_event].
    pub fn create_user_event_sender(&self) -> UserEventSender<UserEventType>
    {
        self.inner.create_user_event_sender()
    }
}

#[cfg(any(doc, doctest, not(target_arch = "wasm32")))]
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
    viewport_size_pixels: UVec2,
    scale_factor: f64
}

impl WindowStartupInfo
{
    pub(crate) fn new(viewport_size_pixels: UVec2, scale_factor: f64) -> Self
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
    pub fn viewport_size_pixels(&self) -> &UVec2
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

/// Describes a difference in the mouse scroll wheel position.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum MouseScrollDistance
{
    /// Number of lines or rows to scroll in each direction. The `y` field
    /// represents the vertical scroll direction on a typical mouse wheel.
    Lines
    {
        /// The horizontal scroll distance. Negative values indicate scrolling
        /// left, and positive values indicate scrolling right.
        x: f64,
        /// The vertical scroll distance. Negative values indicate scrolling
        /// down, and positive values indicate scrolling up.
        y: f64,
        /// The forward/backward scroll distance, supported on some 3D mice.
        z: f64
    },
    /// Number of pixels to scroll in each direction. Scroll events are
    /// expressed in pixels if supported by the device (eg. a touchpad) and
    /// platform. The `y` field represents the vertical scroll direction on a
    /// typical mouse wheel.
    Pixels
    {
        /// The horizontal scroll distance. Negative values indicate scrolling
        /// left, and positive values indicate scrolling right.
        x: f64,
        /// The vertical scroll distance. Negative values indicate scrolling
        /// down, and positive values indicate scrolling up.
        y: f64,
        /// The forward/backward scroll distance, supported on some 3D mice.
        z: f64
    },
    /// Number of pages to scroll in each direction (only supported for
    /// WebCanvas). The `y` field represents the vertical scroll direction on a
    /// typical mouse wheel.
    Pages
    {
        /// The horizontal scroll distance. Negative values indicate scrolling
        /// left, and positive values indicate scrolling right.
        x: f64,
        /// The vertical scroll distance. Negative values indicate scrolling
        /// down, and positive values indicate scrolling up.
        y: f64,
        /// The forward/backward scroll distance, supported on some 3D mice.
        z: f64
    }
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

/// The state of the modifier keys.
#[derive(Debug, Hash, PartialEq, Eq, Clone, Default)]
pub struct ModifiersState
{
    pub(crate) ctrl: bool,
    pub(crate) alt: bool,
    pub(crate) shift: bool,
    pub(crate) logo: bool
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

/// Configuration options about the mode in which the window should be created,
/// for example fullscreen or windowed.
#[derive(Debug, PartialEq, Clone)]
pub(crate) enum WindowCreationMode
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
    PhysicalPixels(UVec2),
    /// Define the window size in device-independent scaled pixels.
    ScaledPixels(Vec2),
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
    PrimaryMonitorPixelsFromTopLeft(IVec2)
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
    pub(crate) mode: WindowCreationMode,
    pub(crate) multisampling: u16,
    pub(crate) vsync: bool,
    pub(crate) always_on_top: bool,
    pub(crate) resizable: bool,
    pub(crate) maximized: bool,
    pub(crate) transparent: bool,
    pub(crate) decorations: bool
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
            multisampling: 16,
            vsync: true,
            always_on_top: false,
            resizable: true,
            maximized: false,
            decorations: true,
            transparent: false
        }
    }

    /// Sets the maximum level of multisampling which will be applied. By
    /// default this is set to `16`.
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

    /// Sets whether or not the window can be resized by the user. The default
    /// is `true`.
    #[inline]
    #[must_use]
    pub fn with_resizable(mut self, resizable: bool) -> Self
    {
        self.resizable = resizable;
        self
    }

    /// If set to `true`, the window will be placed above other windows. The
    /// default is `false`.
    #[inline]
    #[must_use]
    pub fn with_always_on_top(mut self, always_on_top: bool) -> Self
    {
        self.always_on_top = always_on_top;
        self
    }

    /// If set to `true`, the window will be initially maximized. The default is
    /// `false`.
    #[inline]
    #[must_use]
    pub fn with_maximized(mut self, maximized: bool) -> Self
    {
        self.maximized = maximized;
        self
    }

    /// If set to `false`, the window will have no border.  The default is
    /// `true`.
    #[inline]
    #[must_use]
    pub fn with_decorations(mut self, decorations: bool) -> Self
    {
        self.decorations = decorations;
        self
    }

    /// Sets whether the background of the window should be transparent. The
    /// default is `false`.
    ///
    /// Note that this depends on platform support, and setting this may have no
    /// effect.
    #[inline]
    #[must_use]
    pub fn with_transparent(mut self, transparent: bool) -> Self
    {
        self.transparent = transparent;
        self
    }
}

/// Type representing a keyboard scancode.
pub type KeyScancode = u32;
