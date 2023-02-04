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

use std::borrow::Borrow;
use std::cell::{Cell, RefCell};
use std::convert::TryInto;
use std::ops::{Deref, DerefMut, Mul};
use std::rc::Rc;

use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::{KeyboardEvent, MouseEvent, WheelEvent};

use crate::dimen::{IVec2, UVec2, Vec2};
use crate::error::{BacktraceError, ErrorMessage};
use crate::numeric::RoundFloat;
use crate::web::{WebCanvasElement, WebCursorType, WebDocument, WebPending, WebWindow};
use crate::window::{
    DrawingWindowHandler,
    EventLoopSendError,
    KeyScancode,
    ModifiersState,
    MouseButton,
    MouseScrollDistance,
    UserEventSender,
    VirtualKeyCode,
    WindowFullscreenMode,
    WindowHandler,
    WindowHelper,
    WindowStartupInfo
};
use crate::GLRenderer;

fn key_code_from_web(code: &str) -> Option<VirtualKeyCode>
{
    match code {
        "Escape" => Some(VirtualKeyCode::Escape),
        "Digit1" => Some(VirtualKeyCode::Key1),
        "Digit2" => Some(VirtualKeyCode::Key2),
        "Digit3" => Some(VirtualKeyCode::Key3),
        "Digit4" => Some(VirtualKeyCode::Key4),
        "Digit5" => Some(VirtualKeyCode::Key5),
        "Digit6" => Some(VirtualKeyCode::Key6),
        "Digit7" => Some(VirtualKeyCode::Key7),
        "Digit8" => Some(VirtualKeyCode::Key8),
        "Digit9" => Some(VirtualKeyCode::Key9),
        "Digit0" => Some(VirtualKeyCode::Key0),
        "Minus" => Some(VirtualKeyCode::Minus),
        "Equal" => Some(VirtualKeyCode::Equals),
        "Backspace" => Some(VirtualKeyCode::Backspace),
        "Tab" => Some(VirtualKeyCode::Tab),
        "KeyQ" => Some(VirtualKeyCode::Q),
        "KeyW" => Some(VirtualKeyCode::W),
        "KeyE" => Some(VirtualKeyCode::E),
        "KeyR" => Some(VirtualKeyCode::R),
        "KeyT" => Some(VirtualKeyCode::T),
        "KeyY" => Some(VirtualKeyCode::Y),
        "KeyU" => Some(VirtualKeyCode::U),
        "KeyI" => Some(VirtualKeyCode::I),
        "KeyO" => Some(VirtualKeyCode::O),
        "KeyP" => Some(VirtualKeyCode::P),
        "BracketLeft" => Some(VirtualKeyCode::LBracket),
        "BracketRight" => Some(VirtualKeyCode::RBracket),
        "Enter" => Some(VirtualKeyCode::Return),
        "ControlLeft" => Some(VirtualKeyCode::LControl),
        "KeyA" => Some(VirtualKeyCode::A),
        "KeyS" => Some(VirtualKeyCode::S),
        "KeyD" => Some(VirtualKeyCode::D),
        "KeyF" => Some(VirtualKeyCode::F),
        "KeyG" => Some(VirtualKeyCode::G),
        "KeyH" => Some(VirtualKeyCode::H),
        "KeyJ" => Some(VirtualKeyCode::J),
        "KeyK" => Some(VirtualKeyCode::K),
        "KeyL" => Some(VirtualKeyCode::L),
        "Semicolon" => Some(VirtualKeyCode::Semicolon),
        "Quote" => Some(VirtualKeyCode::Apostrophe),
        "Backquote" => Some(VirtualKeyCode::Grave),
        "ShiftLeft" => Some(VirtualKeyCode::LShift),
        "Backslash" => Some(VirtualKeyCode::Backslash),
        "KeyZ" => Some(VirtualKeyCode::Z),
        "KeyX" => Some(VirtualKeyCode::X),
        "KeyC" => Some(VirtualKeyCode::C),
        "KeyV" => Some(VirtualKeyCode::V),
        "KeyB" => Some(VirtualKeyCode::B),
        "KeyN" => Some(VirtualKeyCode::N),
        "KeyM" => Some(VirtualKeyCode::M),
        "Comma" => Some(VirtualKeyCode::Comma),
        "Period" => Some(VirtualKeyCode::Period),
        "Slash" => Some(VirtualKeyCode::Slash),
        "ShiftRight" => Some(VirtualKeyCode::RShift),
        "NumpadMultiply" => Some(VirtualKeyCode::NumpadMultiply),
        "AltLeft" => Some(VirtualKeyCode::LAlt),
        "Space" => Some(VirtualKeyCode::Space),
        "CapsLock" => Some(VirtualKeyCode::Capital),
        "F1" => Some(VirtualKeyCode::F1),
        "F2" => Some(VirtualKeyCode::F2),
        "F3" => Some(VirtualKeyCode::F3),
        "F4" => Some(VirtualKeyCode::F4),
        "F5" => Some(VirtualKeyCode::F5),
        "F6" => Some(VirtualKeyCode::F6),
        "F7" => Some(VirtualKeyCode::F7),
        "F8" => Some(VirtualKeyCode::F8),
        "F9" => Some(VirtualKeyCode::F9),
        "F10" => Some(VirtualKeyCode::F10),
        "Pause" => Some(VirtualKeyCode::PauseBreak),
        "ScrollLock" => Some(VirtualKeyCode::ScrollLock),
        "Numpad7" => Some(VirtualKeyCode::Numpad7),
        "Numpad8" => Some(VirtualKeyCode::Numpad8),
        "Numpad9" => Some(VirtualKeyCode::Numpad9),
        "NumpadSubtract" => Some(VirtualKeyCode::NumpadSubtract),
        "Numpad4" => Some(VirtualKeyCode::Numpad4),
        "Numpad5" => Some(VirtualKeyCode::Numpad5),
        "Numpad6" => Some(VirtualKeyCode::Numpad6),
        "NumpadAdd" => Some(VirtualKeyCode::NumpadAdd),
        "Numpad1" => Some(VirtualKeyCode::Numpad1),
        "Numpad2" => Some(VirtualKeyCode::Numpad2),
        "Numpad3" => Some(VirtualKeyCode::Numpad3),
        "Numpad0" => Some(VirtualKeyCode::Numpad0),
        "NumpadDecimal" => Some(VirtualKeyCode::NumpadDecimal),
        "PrintScreen" => Some(VirtualKeyCode::PrintScreen),
        "IntlBackslash" => Some(VirtualKeyCode::Backslash),
        "F11" => Some(VirtualKeyCode::F11),
        "F12" => Some(VirtualKeyCode::F12),
        "NumpadEqual" => Some(VirtualKeyCode::NumpadEquals),
        "F13" => Some(VirtualKeyCode::F13),
        "F14" => Some(VirtualKeyCode::F14),
        "F15" => Some(VirtualKeyCode::F15),
        "F16" => Some(VirtualKeyCode::F16),
        "F17" => Some(VirtualKeyCode::F17),
        "F18" => Some(VirtualKeyCode::F18),
        "F19" => Some(VirtualKeyCode::F19),
        "F20" => Some(VirtualKeyCode::F20),
        "F21" => Some(VirtualKeyCode::F21),
        "F22" => Some(VirtualKeyCode::F22),
        "F23" => Some(VirtualKeyCode::F23),
        "KanaMode" => Some(VirtualKeyCode::Kana),
        "Lang2" => None,
        "Lang1" => None,
        "IntlRo" => None,
        "F24" => Some(VirtualKeyCode::F24),
        "Convert" => Some(VirtualKeyCode::Convert),
        "NonConvert" => Some(VirtualKeyCode::NoConvert),
        "IntlYen" => Some(VirtualKeyCode::Yen),
        "NumpadComma" => Some(VirtualKeyCode::NumpadComma),
        "Paste" => Some(VirtualKeyCode::Paste),
        "MediaTrackPrevious" => Some(VirtualKeyCode::PrevTrack),
        "Cut" => Some(VirtualKeyCode::Cut),
        "Copy" => Some(VirtualKeyCode::Copy),
        "MediaTrackNext" => Some(VirtualKeyCode::NextTrack),
        "NumpadEnter" => Some(VirtualKeyCode::NumpadEnter),
        "ControlRight" => Some(VirtualKeyCode::RControl),
        "AudioVolumeMute" => Some(VirtualKeyCode::Mute),
        "MediaPlayPause" => Some(VirtualKeyCode::PlayPause),
        "MediaStop" => Some(VirtualKeyCode::MediaStop),
        "VolumeDown" => Some(VirtualKeyCode::VolumeDown),
        "AudioVolumeDown" => Some(VirtualKeyCode::VolumeDown),
        "VolumeUp" => Some(VirtualKeyCode::VolumeUp),
        "AudioVolumeUp" => Some(VirtualKeyCode::VolumeUp),
        "BrowserHome" => Some(VirtualKeyCode::WebHome),
        "NumpadDivide" => Some(VirtualKeyCode::NumpadDivide),
        "AltRight" => Some(VirtualKeyCode::RAlt),
        "NumLock" => Some(VirtualKeyCode::Numlock),
        "Home" => Some(VirtualKeyCode::Home),
        "ArrowUp" => Some(VirtualKeyCode::Up),
        "PageUp" => Some(VirtualKeyCode::PageUp),
        "ArrowLeft" => Some(VirtualKeyCode::Left),
        "ArrowRight" => Some(VirtualKeyCode::Right),
        "End" => Some(VirtualKeyCode::End),
        "ArrowDown" => Some(VirtualKeyCode::Down),
        "PageDown" => Some(VirtualKeyCode::PageDown),
        "Insert" => Some(VirtualKeyCode::Insert),
        "Delete" => Some(VirtualKeyCode::Delete),
        "OSLeft" => Some(VirtualKeyCode::LWin),
        "MetaLeft" => Some(VirtualKeyCode::LWin),
        "OSRight" => Some(VirtualKeyCode::RWin),
        "MetaRight" => Some(VirtualKeyCode::RWin),
        "ContextMenu" => None,
        "Power" => Some(VirtualKeyCode::Power),
        "BrowserSearch" => Some(VirtualKeyCode::WebSearch),
        "BrowserFavorites" => Some(VirtualKeyCode::WebFavorites),
        "BrowserRefresh" => Some(VirtualKeyCode::WebRefresh),
        "BrowserStop" => Some(VirtualKeyCode::Stop),
        "BrowserForward" => Some(VirtualKeyCode::WebForward),
        "BrowserBack" => Some(VirtualKeyCode::WebBack),
        "LaunchMail" => Some(VirtualKeyCode::Mail),
        "MediaSelect" => Some(VirtualKeyCode::MediaSelect),
        _ => None
    }
}

fn get_scan_code_from_key_code(code: VirtualKeyCode) -> Option<KeyScancode>
{
    Some(match code {
        VirtualKeyCode::Escape => 0x0001,
        VirtualKeyCode::Key1 => 0x0002,
        VirtualKeyCode::Key2 => 0x0003,
        VirtualKeyCode::Key3 => 0x0004,
        VirtualKeyCode::Key4 => 0x0005,
        VirtualKeyCode::Key5 => 0x0006,
        VirtualKeyCode::Key6 => 0x0007,
        VirtualKeyCode::Key7 => 0x0008,
        VirtualKeyCode::Key8 => 0x0009,
        VirtualKeyCode::Key9 => 0x000A,
        VirtualKeyCode::Key0 => 0x000B,
        VirtualKeyCode::Minus => 0x000C,
        VirtualKeyCode::Equals => 0x000D,
        VirtualKeyCode::Backspace => 0x000E,
        VirtualKeyCode::Tab => 0x000F,
        VirtualKeyCode::Q => 0x0010,
        VirtualKeyCode::W => 0x0011,
        VirtualKeyCode::E => 0x0012,
        VirtualKeyCode::R => 0x0013,
        VirtualKeyCode::T => 0x0014,
        VirtualKeyCode::Y => 0x0015,
        VirtualKeyCode::U => 0x0016,
        VirtualKeyCode::I => 0x0017,
        VirtualKeyCode::O => 0x0018,
        VirtualKeyCode::P => 0x0019,
        VirtualKeyCode::LBracket => 0x001A,
        VirtualKeyCode::RBracket => 0x001B,
        VirtualKeyCode::Return => 0x001C,
        VirtualKeyCode::LControl => 0x001D,
        VirtualKeyCode::A => 0x001E,
        VirtualKeyCode::S => 0x001F,
        VirtualKeyCode::D => 0x0020,
        VirtualKeyCode::F => 0x0021,
        VirtualKeyCode::G => 0x0022,
        VirtualKeyCode::H => 0x0023,
        VirtualKeyCode::J => 0x0024,
        VirtualKeyCode::K => 0x0025,
        VirtualKeyCode::L => 0x0026,
        VirtualKeyCode::Semicolon => 0x0027,
        VirtualKeyCode::Apostrophe => 0x0028,
        VirtualKeyCode::Grave => 0x0029,
        VirtualKeyCode::LShift => 0x002A,
        VirtualKeyCode::Backslash => 0x002B,
        VirtualKeyCode::Z => 0x002C,
        VirtualKeyCode::X => 0x002D,
        VirtualKeyCode::C => 0x002E,
        VirtualKeyCode::V => 0x002F,
        VirtualKeyCode::B => 0x0030,
        VirtualKeyCode::N => 0x0031,
        VirtualKeyCode::M => 0x0032,
        VirtualKeyCode::Comma => 0x0033,
        VirtualKeyCode::Period => 0x0034,
        VirtualKeyCode::Slash => 0x0035,
        VirtualKeyCode::RShift => 0x0036,
        VirtualKeyCode::NumpadMultiply => 0x0037,
        VirtualKeyCode::LAlt => 0x0038,
        VirtualKeyCode::Space => 0x0039,
        VirtualKeyCode::F1 => 0x003B,
        VirtualKeyCode::F2 => 0x003C,
        VirtualKeyCode::F3 => 0x003D,
        VirtualKeyCode::F4 => 0x003E,
        VirtualKeyCode::F5 => 0x003F,
        VirtualKeyCode::F6 => 0x0040,
        VirtualKeyCode::F7 => 0x0041,
        VirtualKeyCode::F8 => 0x0042,
        VirtualKeyCode::F9 => 0x0043,
        VirtualKeyCode::F10 => 0x0044,
        VirtualKeyCode::PauseBreak => 0x0045,
        VirtualKeyCode::ScrollLock => 0x0046,
        VirtualKeyCode::Numpad7 => 0x0047,
        VirtualKeyCode::Numpad8 => 0x0048,
        VirtualKeyCode::Numpad9 => 0x0049,
        VirtualKeyCode::NumpadSubtract => 0x004A,
        VirtualKeyCode::Numpad4 => 0x004B,
        VirtualKeyCode::Numpad5 => 0x004C,
        VirtualKeyCode::Numpad6 => 0x004D,
        VirtualKeyCode::NumpadAdd => 0x004E,
        VirtualKeyCode::Numpad1 => 0x004F,
        VirtualKeyCode::Numpad2 => 0x0050,
        VirtualKeyCode::Numpad3 => 0x0051,
        VirtualKeyCode::Numpad0 => 0x0052,
        VirtualKeyCode::NumpadDecimal => 0x0053,
        VirtualKeyCode::PrintScreen => 0x0054,
        VirtualKeyCode::F11 => 0x0057,
        VirtualKeyCode::F12 => 0x0058,
        VirtualKeyCode::NumpadEquals => 0x0059,
        VirtualKeyCode::F13 => 0x0064,
        VirtualKeyCode::F14 => 0x0065,
        VirtualKeyCode::F15 => 0x0066,
        VirtualKeyCode::F16 => 0x0067,
        VirtualKeyCode::F17 => 0x0068,
        VirtualKeyCode::F18 => 0x0069,
        VirtualKeyCode::F19 => 0x006A,
        VirtualKeyCode::F20 => 0x006B,
        VirtualKeyCode::F21 => 0x006C,
        VirtualKeyCode::F22 => 0x006D,
        VirtualKeyCode::F23 => 0x006E,
        VirtualKeyCode::Kana => 0x0070,
        VirtualKeyCode::F24 => 0x0076,
        VirtualKeyCode::Convert => 0x0079,
        VirtualKeyCode::NoConvert => 0x007B,
        VirtualKeyCode::Yen => 0x007D,
        VirtualKeyCode::NumpadComma => 0x007E,
        VirtualKeyCode::Paste => 0xE00A,
        VirtualKeyCode::PrevTrack => 0xE010,
        VirtualKeyCode::Cut => 0xE017,
        VirtualKeyCode::Copy => 0xE018,
        VirtualKeyCode::NextTrack => 0xE019,
        VirtualKeyCode::NumpadEnter => 0xE01C,
        VirtualKeyCode::RControl => 0xE01D,
        VirtualKeyCode::Mute => 0xE020,
        VirtualKeyCode::PlayPause => 0xE022,
        VirtualKeyCode::MediaStop => 0xE024,
        VirtualKeyCode::VolumeDown => 0xE02E,
        VirtualKeyCode::VolumeUp => 0xE030,
        VirtualKeyCode::WebHome => 0xE032,
        VirtualKeyCode::NumpadDivide => 0xE035,
        VirtualKeyCode::RAlt => 0xE038,
        VirtualKeyCode::Numlock => 0xE045,
        VirtualKeyCode::Home => 0xE047,
        VirtualKeyCode::Up => 0xE048,
        VirtualKeyCode::PageUp => 0xE049,
        VirtualKeyCode::Left => 0xE04B,
        VirtualKeyCode::Right => 0xE04D,
        VirtualKeyCode::End => 0xE04F,
        VirtualKeyCode::Down => 0xE050,
        VirtualKeyCode::PageDown => 0xE051,
        VirtualKeyCode::Insert => 0xE052,
        VirtualKeyCode::Delete => 0xE053,
        VirtualKeyCode::LWin => 0xE05B,
        VirtualKeyCode::RWin => 0xE05C,
        VirtualKeyCode::Power => 0xE05E,
        VirtualKeyCode::WebSearch => 0xE065,
        VirtualKeyCode::WebFavorites => 0xE066,
        VirtualKeyCode::WebRefresh => 0xE067,
        VirtualKeyCode::WebStop => 0xE068,
        VirtualKeyCode::WebForward => 0xE069,
        VirtualKeyCode::WebBack => 0xE06A,
        VirtualKeyCode::Mail => 0xE06C,
        VirtualKeyCode::MediaSelect => 0xE06D,
        VirtualKeyCode::Compose => return None,
        VirtualKeyCode::Caret => return None,
        VirtualKeyCode::AbntC1 => return None,
        VirtualKeyCode::AbntC2 => return None,
        VirtualKeyCode::Apps => return None,
        VirtualKeyCode::Asterisk => return None,
        VirtualKeyCode::At => return None,
        VirtualKeyCode::Ax => return None,
        VirtualKeyCode::Calculator => return None,
        VirtualKeyCode::Capital => 0x003A,
        VirtualKeyCode::Colon => return None,
        VirtualKeyCode::Kanji => return None,
        VirtualKeyCode::MyComputer => return None,
        VirtualKeyCode::NavigateForward => return None,
        VirtualKeyCode::NavigateBackward => return None,
        VirtualKeyCode::OEM102 => 0xE056,
        VirtualKeyCode::Plus => 0xE00D,
        VirtualKeyCode::Sleep => 0xE05F,
        VirtualKeyCode::Stop => return None,
        VirtualKeyCode::Sysrq => return None,
        VirtualKeyCode::Underline => return None,
        VirtualKeyCode::Unlabeled => return None,
        VirtualKeyCode::Wake => return None
    })
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum KeyEventType
{
    Down,
    Up
}

pub struct WindowHelperWeb<UserEventType>
where
    UserEventType: 'static
{
    redraw_pending: RefCell<Option<WebPending>>,
    redraw_request_action: Option<Box<RefCell<dyn FnMut() -> WebPending>>>,
    post_user_event_action: Option<Rc<RefCell<UserEventSenderActionType<UserEventType>>>>,
    terminate_loop_action: Option<Box<dyn FnOnce()>>,
    canvas: WebCanvasElement,
    document: WebDocument,
    window: WebWindow
}

impl<UserEventType: 'static> WindowHelperWeb<UserEventType>
{
    fn new(canvas: WebCanvasElement, document: WebDocument, window: WebWindow) -> Self
    {
        Self {
            redraw_pending: RefCell::new(None),
            redraw_request_action: None,
            post_user_event_action: None,
            terminate_loop_action: None,
            canvas,
            document,
            window
        }
    }

    pub fn set_redraw_request_action<F>(&mut self, redraw_request_action: F)
    where
        F: FnMut() -> WebPending + 'static
    {
        self.redraw_request_action = Some(Box::new(RefCell::new(redraw_request_action)));
    }

    pub fn set_post_user_event_action<F>(&mut self, post_user_event_action: F)
    where
        F: FnMut(UserEventType) -> Result<(), BacktraceError<ErrorMessage>> + 'static
    {
        self.post_user_event_action = Some(Rc::new(RefCell::new(post_user_event_action)));
    }

    pub fn set_terminate_loop_action<F>(&mut self, terminate_loop_action: F)
    where
        F: FnOnce() + 'static
    {
        self.terminate_loop_action = Some(Box::new(terminate_loop_action));
    }

    pub fn clear_redraw_pending_flag(&self)
    {
        if let Some(pending) = self.redraw_pending.borrow_mut().deref_mut() {
            pending.mark_as_triggered()
        }
        self.redraw_pending.replace(None);
    }

    pub fn terminate_loop(&mut self)
    {
        self.redraw_pending.replace(None);
        self.redraw_request_action = None;
        if let Some(action) = self.terminate_loop_action.take() {
            action();
        }
    }

    pub fn set_icon_from_rgba_pixels<S>(
        &self,
        _data: Vec<u8>,
        _size: S
    ) -> Result<(), BacktraceError<ErrorMessage>>
    where
        S: Into<UVec2>
    {
        // Do nothing
        Err(ErrorMessage::msg("Cannot set icon for WebCanvas"))
    }

    pub fn set_cursor_visible(&self, visible: bool)
    {
        if visible {
            self.canvas.set_cursor(WebCursorType::Auto);
        } else {
            self.canvas.set_cursor(WebCursorType::None);
        }
    }

    pub fn set_cursor_grab(
        &self,
        grabbed: bool
    ) -> Result<(), BacktraceError<ErrorMessage>>
    {
        if grabbed {
            self.canvas.request_pointer_lock();
        } else {
            self.window.document().unwrap().exit_pointer_lock();
        }

        Ok(())
    }

    pub fn set_resizable(&self, _resizable: bool)
    {
        // Do nothing
    }

    #[inline]
    pub fn request_redraw(&self)
    {
        if self.redraw_request_action.borrow().is_none() {
            log::warn!("Ignoring call to request_redraw() in invalid state");
            return;
        }

        if self.redraw_pending.borrow().is_none() {
            self.redraw_pending.replace(Some(self
                .redraw_request_action
                .as_ref()
                .unwrap()
                .deref()
                .borrow_mut()()));
        }
    }

    pub fn set_title(&self, title: &str)
    {
        self.window.document().unwrap().set_title(title);
    }

    pub fn set_fullscreen_mode(&self, mode: WindowFullscreenMode)
    {
        match mode {
            WindowFullscreenMode::Windowed => {
                self.document.exit_fullscreen();
            }
            WindowFullscreenMode::FullscreenBorderless => {
                self.canvas.request_fullscreen();
            }
        }
    }

    pub fn set_size_pixels<S: Into<UVec2>>(&self, _size: S)
    {
        // Do nothing
    }

    pub fn set_position_pixels<P: Into<IVec2>>(&self, _position: P)
    {
        // Do nothing
    }

    pub fn set_size_scaled_pixels<S: Into<Vec2>>(&self, _size: S)
    {
        // Do nothing
    }

    pub fn set_position_scaled_pixels<P: Into<Vec2>>(&self, _position: P)
    {
        // Do nothing
    }

    #[inline]
    #[must_use]
    pub fn get_scale_factor(&self) -> f64
    {
        self.window.device_pixel_ratio()
    }

    pub fn create_user_event_sender(&self) -> UserEventSender<UserEventType>
    {
        UserEventSender::new(UserEventSenderWeb::new(
            self.post_user_event_action.as_ref().unwrap().clone()
        ))
    }
}

type UserEventSenderActionType<UserEventType> =
    dyn FnMut(UserEventType) -> Result<(), BacktraceError<ErrorMessage>>;

#[derive(Clone)]
pub struct UserEventSenderWeb<UserEventType>
where
    UserEventType: 'static
{
    action: Rc<RefCell<UserEventSenderActionType<UserEventType>>>
}

impl<UserEventType: 'static> UserEventSenderWeb<UserEventType>
{
    fn new(action: Rc<RefCell<UserEventSenderActionType<UserEventType>>>) -> Self
    {
        Self { action }
    }

    #[inline]
    pub fn send_event(&self, event: UserEventType) -> Result<(), EventLoopSendError>
    {
        RefCell::borrow_mut(Rc::borrow(&self.action))(event).unwrap();
        Ok(())
    }
}

pub struct WebCanvasImpl
{
    event_listeners_to_clean_up: Rc<RefCell<Vec<WebPending>>>
}

impl WebCanvasImpl
{
    fn handle_key_event<H, UserEventType>(
        event_type: KeyEventType,
        event: KeyboardEvent,
        handler: &Rc<RefCell<DrawingWindowHandler<UserEventType, H>>>,
        helper: &Rc<RefCell<WindowHelper<UserEventType>>>,
        modifiers: &Rc<RefCell<ModifiersState>>
    ) where
        H: WindowHandler<UserEventType> + 'static,
        UserEventType: 'static
    {
        let code: String = event.code();

        let mut handler = RefCell::borrow_mut(Rc::borrow(handler));
        let mut helper = RefCell::borrow_mut(Rc::borrow(helper));
        let mut modifiers = RefCell::borrow_mut(Rc::borrow(modifiers));

        if let Some(virtual_key_code) = key_code_from_web(code.as_str()) {
            let scancode = get_scan_code_from_key_code(virtual_key_code);

            if let Some(scancode) = scancode {
                match event_type {
                    KeyEventType::Down => handler.on_key_down(
                        helper.deref_mut(),
                        Some(virtual_key_code),
                        scancode
                    ),
                    KeyEventType::Up => handler.on_key_up(
                        helper.deref_mut(),
                        Some(virtual_key_code),
                        scancode
                    )
                }
            } else {
                log::warn!(
                    "Ignoring key {:?} due to unknown scancode",
                    virtual_key_code
                );
            }
        } else {
            log::warn!("Ignoring unknown key code {}", code);
        }

        if event_type == KeyEventType::Down {
            let key: String = event.key();

            if key.chars().count() == 1 {
                handler.on_keyboard_char(helper.deref_mut(), key.chars().next().unwrap());
            }
        }

        let new_modifiers = ModifiersState {
            ctrl: event.get_modifier_state("Control"),
            alt: event.get_modifier_state("Alt"),
            shift: event.get_modifier_state("Shift"),
            logo: event.get_modifier_state("OS")
        };

        if new_modifiers != *modifiers {
            *modifiers = new_modifiers.clone();
            handler.on_keyboard_modifiers_changed(helper.deref_mut(), new_modifiers);
        }
    }

    pub fn new<S, H, UserEventType>(
        element_id: S,
        handler: H
    ) -> Result<Self, BacktraceError<ErrorMessage>>
    where
        S: AsRef<str>,
        H: WindowHandler<UserEventType> + 'static,
        UserEventType: 'static
    {
        let window = WebWindow::new()?;
        let document = window.document()?;

        let canvas = WebCanvasElement::new_by_id(&element_id)?;

        let initial_size_scaled = canvas.html_element().element().dimensions();
        let initial_dpr = window.device_pixel_ratio();

        let current_dpr = Rc::new(Cell::new(initial_dpr));

        let initial_size_unscaled =
            (initial_size_scaled * initial_dpr).round().into_u32();

        canvas.set_buffer_dimensions(&initial_size_unscaled);

        // Needed to ensure we can get keyboard focus
        canvas.set_tab_index(0);

        let mut event_listeners_to_clean_up = Vec::new();
        let is_pointer_locked = Rc::new(Cell::new(false));

        let renderer =
            GLRenderer::new_for_web_canvas_by_id(initial_size_unscaled, &element_id)
                .map_err(|err| {
                    ErrorMessage::msg_with_cause("Failed to create renderer", err)
                })?;

        let handler = Rc::new(RefCell::new(DrawingWindowHandler::new(handler, renderer)));

        let helper = {
            Rc::new(RefCell::new(WindowHelper::new(WindowHelperWeb::new(
                canvas.clone(),
                document.clone(),
                window.clone()
            ))))
        };

        {
            let helper_inner = helper.clone();
            let window = window.clone();
            let handler = handler.clone();

            let frame_callback = RefCell::new(Closure::wrap(Box::new(move || {
                RefCell::borrow_mut(Rc::borrow(&helper_inner))
                    .inner()
                    .clear_redraw_pending_flag();
                RefCell::borrow_mut(Rc::borrow(&handler))
                    .on_draw(RefCell::borrow_mut(Rc::borrow(&helper_inner)).deref_mut());
            })
                as Box<dyn FnMut()>));

            let redraw_request_action =
                move || window.request_animation_frame(&frame_callback).unwrap();

            RefCell::borrow_mut(Rc::borrow(&helper))
                .inner()
                .set_redraw_request_action(redraw_request_action);
        }

        {
            let user_event_queue = Rc::new(RefCell::new(Vec::new()));
            let user_event_callback_pending = Rc::new(RefCell::new(None));
            let window = window.clone();

            let callback = {
                let handler = handler.clone();
                let helper = helper.clone();
                let user_event_queue = user_event_queue.clone();
                let user_event_callback_pending = user_event_callback_pending.clone();

                RefCell::new(Closure::wrap(Box::new(move || {
                    let user_event_callback_pending: Option<WebPending> =
                        user_event_callback_pending.take();
                    user_event_callback_pending.unwrap().mark_as_triggered();

                    let mut pending_events = Vec::new();
                    std::mem::swap(
                        &mut pending_events,
                        RefCell::borrow_mut(Rc::borrow(&user_event_queue)).deref_mut()
                    );
                    pending_events.drain(..).for_each(|event| {
                        RefCell::borrow_mut(Rc::borrow(&handler)).on_user_event(
                            RefCell::borrow_mut(Rc::borrow(&helper)).deref_mut(),
                            event
                        )
                    });
                }) as Box<dyn FnMut()>))
            };

            RefCell::borrow_mut(Rc::borrow(&helper))
                .inner()
                .set_post_user_event_action(move |event| {
                    RefCell::borrow_mut(Rc::borrow(&user_event_queue)).push(event);

                    if user_event_callback_pending.deref().borrow().is_none() {
                        user_event_callback_pending
                            .replace(Some(window.set_timeout_immediate(&callback)?));
                    }

                    Ok(())
                })
        }

        let canvas_event_target = canvas
            .html_element()
            .element()
            .clone()
            .dyn_into_event_target()?;

        match canvas_event_target
            .register_event_listener_mouse("contextmenu", move |event| {
                event.prevent_default()
            }) {
            Ok(listener) => event_listeners_to_clean_up.push(listener),
            Err(err) => {
                log::error!("Failed to register context menu event listener: {:?}", err)
            }
        }

        {
            let handler = handler.clone();
            let helper = helper.clone();
            let window_inner = window.clone();
            let canvas = canvas.clone();
            let current_dpr = current_dpr.clone();

            event_listeners_to_clean_up.push(
                window
                    .clone()
                    .dyn_into_event_target()?
                    .register_event_listener_void("resize", move || {
                        let size_scaled = canvas.html_element().element().dimensions();
                        let dpr = window_inner.device_pixel_ratio();

                        Cell::replace(Rc::borrow(&current_dpr), dpr);

                        let size_unscaled = (size_scaled * dpr).round().into_u32();

                        canvas.set_buffer_dimensions(&size_unscaled);

                        RefCell::borrow_mut(Rc::borrow(&handler)).on_resize(
                            RefCell::borrow_mut(Rc::borrow(&helper)).deref_mut(),
                            size_unscaled
                        );

                        RefCell::borrow_mut(Rc::borrow(&handler)).on_draw(
                            RefCell::borrow_mut(Rc::borrow(&helper)).deref_mut()
                        );
                    })?
            );
        }

        {
            let handler = handler.clone();
            let helper = helper.clone();
            let canvas = canvas.clone();
            let is_pointer_locked = is_pointer_locked.clone();

            event_listeners_to_clean_up.push(
                document
                    .clone()
                    .dyn_into_event_target()?
                    .register_event_listener_void("pointerlockchange", move || {
                        let mouse_grabbed = canvas.is_pointer_lock_active();

                        is_pointer_locked.set(mouse_grabbed);

                        RefCell::borrow_mut(Rc::borrow(&handler))
                            .on_mouse_grab_status_changed(
                                RefCell::borrow_mut(Rc::borrow(&helper)).deref_mut(),
                                mouse_grabbed
                            );
                    })?
            );
        }

        {
            let handler = handler.clone();
            let helper = helper.clone();

            event_listeners_to_clean_up.push(
                document
                    .dyn_into_event_target()?
                    .register_event_listener_void("fullscreenchange", move || {
                        let fullscreen = canvas.is_fullscreen_active();

                        RefCell::borrow_mut(Rc::borrow(&handler))
                            .on_fullscreen_status_changed(
                                RefCell::borrow_mut(Rc::borrow(&helper)).deref_mut(),
                                fullscreen
                            );
                    })?
            );
        }

        {
            let handler = handler.clone();
            let helper = helper.clone();
            let current_dpr = current_dpr.clone();

            event_listeners_to_clean_up.push(
                canvas_event_target.register_event_listener_mouse(
                    "mousemove",
                    move |event| {
                        let current_dpr = Cell::get(Rc::borrow(&current_dpr)) as f32;

                        let position = if is_pointer_locked.get() {
                            IVec2::new(event.movement_x(), event.movement_y())
                                .into_f32()
                                .mul(current_dpr)
                        } else {
                            IVec2::new(event.offset_x(), event.offset_y())
                                .into_f32()
                                .mul(current_dpr)
                        };

                        RefCell::borrow_mut(Rc::borrow(&handler)).on_mouse_move(
                            RefCell::borrow_mut(Rc::borrow(&helper)).deref_mut(),
                            position
                        );
                    }
                )?
            );
        }

        {
            let handler = handler.clone();
            let helper = helper.clone();

            event_listeners_to_clean_up.push(
                canvas_event_target.register_event_listener_mouse(
                    "mousedown",
                    move |event| match mouse_button_from_event(&event) {
                        None => {
                            log::error!(
                                "Mouse down: Unknown mouse button {}",
                                event.button()
                            )
                        }
                        Some(button) => RefCell::borrow_mut(Rc::borrow(&handler))
                            .on_mouse_button_down(
                                RefCell::borrow_mut(Rc::borrow(&helper)).deref_mut(),
                                button
                            )
                    }
                )?
            );
        }

        {
            let handler = handler.clone();
            let helper = helper.clone();

            event_listeners_to_clean_up.push(
                canvas_event_target.register_event_listener_mouse(
                    "mouseup",
                    move |event| match mouse_button_from_event(&event) {
                        None => {
                            log::error!(
                                "Mouse up: Unknown mouse button {}",
                                event.button()
                            )
                        }
                        Some(button) => RefCell::borrow_mut(Rc::borrow(&handler))
                            .on_mouse_button_up(
                                RefCell::borrow_mut(Rc::borrow(&helper)).deref_mut(),
                                button
                            )
                    }
                )?
            );
        }

        {
            let handler = handler.clone();
            let helper = helper.clone();

            event_listeners_to_clean_up.push(
                canvas_event_target.register_event_listener_mouse(
                    "wheel",
                    move |event| {
                        let event: WheelEvent = event.dyn_into().unwrap();

                        let delta = match event.delta_mode() {
                            0x00 => MouseScrollDistance::Pixels {
                                x: event.delta_x(),
                                y: -event.delta_y(),
                                z: event.delta_z()
                            },

                            0x01 => MouseScrollDistance::Lines {
                                x: event.delta_x(),
                                y: -event.delta_y(),
                                z: event.delta_z()
                            },

                            0x02 => MouseScrollDistance::Pages {
                                x: event.delta_x(),
                                y: -event.delta_y(),
                                z: event.delta_z()
                            },

                            mode => {
                                log::error!("Mouse wheel: Unknown delta mode {}", mode);
                                return;
                            }
                        };

                        handler.borrow_mut().on_mouse_wheel_scroll(
                            helper.borrow_mut().deref_mut(),
                            delta
                        );
                    }
                )?
            );
        }

        let modifier_state = Rc::new(RefCell::new(ModifiersState::default()));

        {
            let handler = handler.clone();
            let helper = helper.clone();
            let modifier_state = modifier_state.clone();

            event_listeners_to_clean_up.push(
                canvas_event_target.register_event_listener_keyboard(
                    "keydown",
                    move |event| {
                        Self::handle_key_event(
                            KeyEventType::Down,
                            event,
                            &handler,
                            &helper,
                            &modifier_state
                        );
                    }
                )?
            );
        }

        {
            let handler = handler.clone();
            let helper = helper.clone();

            event_listeners_to_clean_up.push(
                canvas_event_target.register_event_listener_keyboard(
                    "keyup",
                    move |event| {
                        Self::handle_key_event(
                            KeyEventType::Up,
                            event,
                            &handler,
                            &helper,
                            &modifier_state
                        );
                    }
                )?
            );
        }

        {
            let handler = handler.clone();
            let helper = helper.clone();

            let device_pixel_ratio_event_listener = Rc::new(Cell::new(None));

            {
                let device_pixel_ratio_event_listener =
                    device_pixel_ratio_event_listener.clone();

                event_listeners_to_clean_up.push(WebPending::new(move || {
                    Cell::replace(Rc::borrow(&device_pixel_ratio_event_listener), None);
                }));
            }

            let callback: Rc<RefCell<Box<dyn FnMut()>>> =
                Rc::new(RefCell::new(Box::new(|| {
                    panic!("Device pixel ratio callback not present")
                })));

            let callback_inner = callback.clone();

            drop(RefCell::replace(
                Rc::borrow(&callback),
                Box::new(move || {
                    let new_dpr = window.device_pixel_ratio();
                    log::info!("DPI changed to {}", new_dpr);

                    Cell::replace(Rc::borrow(&current_dpr), new_dpr);

                    handler.borrow_mut().on_scale_factor_changed(
                        RefCell::borrow_mut(Rc::borrow(&helper)).deref_mut(),
                        new_dpr
                    );

                    let callback_inner = callback_inner.clone();

                    Cell::replace(
                        Rc::borrow(&device_pixel_ratio_event_listener),
                        Some(
                            window
                                .clone()
                                .match_media(&format!("(resolution: {new_dpr}dppx"))
                                .unwrap()
                                .register_event_listener_media_event_list_once(
                                    "change",
                                    move |_event| {
                                        RefCell::borrow_mut(Rc::borrow(&callback_inner))(
                                        );
                                    }
                                )
                                .unwrap()
                        )
                    );
                })
            ));

            RefCell::borrow_mut(Rc::borrow(&callback))();
        }

        let terminated = Rc::new(Cell::new(false));
        let event_listeners_to_clean_up =
            Rc::new(RefCell::new(event_listeners_to_clean_up));

        {
            let terminated = terminated.clone();
            let event_listeners_to_clean_up = event_listeners_to_clean_up.clone();

            RefCell::borrow_mut(Rc::borrow(&helper))
                .inner()
                .set_terminate_loop_action(move || {
                    log::info!("Terminating event loop");
                    terminated.set(true);
                    RefCell::borrow_mut(Rc::borrow(&event_listeners_to_clean_up)).clear();
                });
        }

        log::info!(
            "Initial scaled canvas size: {:?}, dpr {}, unscaled: {:?}",
            initial_size_scaled,
            initial_dpr,
            initial_size_unscaled
        );

        RefCell::borrow_mut(Rc::borrow(&handler)).on_start(
            RefCell::borrow_mut(Rc::borrow(&helper)).deref_mut(),
            WindowStartupInfo::new(initial_size_unscaled, initial_dpr)
        );

        if !terminated.get() {
            RefCell::borrow_mut(Rc::borrow(&handler))
                .on_draw(RefCell::borrow_mut(Rc::borrow(&helper)).deref_mut());
        }

        Ok(WebCanvasImpl {
            event_listeners_to_clean_up
        })
    }
}

impl Drop for WebCanvasImpl
{
    fn drop(&mut self)
    {
        log::info!("Unregistering WebCanvasImpl");
        RefCell::borrow_mut(Rc::borrow(&self.event_listeners_to_clean_up)).clear();
    }
}

fn mouse_button_from_event(event: &MouseEvent) -> Option<MouseButton>
{
    let button: i16 = event.button();
    match button {
        0 => Some(MouseButton::Left),
        1 => Some(MouseButton::Middle),
        2 => Some(MouseButton::Right),
        _ => Some(MouseButton::Other(button.try_into().unwrap()))
    }
}
