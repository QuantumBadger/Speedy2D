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
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::rc::Rc;

use wasm_bindgen::closure::Closure;
use web_sys::MouseEvent;

use crate::dimen::Vector2;
use crate::error::{BacktraceError, ErrorMessage};
use crate::numeric::RoundFloat;
use crate::web::{WebCanvasElement, WebCursorType, WebPending, WebWindow};
use crate::window::{
    DrawingWindowHandler,
    EventLoopSendError,
    MouseButton,
    UserEventSender,
    WindowFullscreenMode,
    WindowHandler,
    WindowHelper,
    WindowStartupInfo
};
use crate::{GLRenderer, WebCanvasAttachOptions};

pub struct WindowHelperWeb<UserEventType>
where
    UserEventType: 'static
{
    phantom: PhantomData<UserEventType>,
    redraw_pending: RefCell<Option<WebPending>>,
    redraw_request_action: Option<Box<RefCell<dyn FnMut() -> WebPending>>>,
    terminate_loop_action: Option<Box<dyn FnOnce()>>,
    canvas: WebCanvasElement,
    window: WebWindow
}

impl<UserEventType: 'static> WindowHelperWeb<UserEventType>
{
    fn new(canvas: WebCanvasElement, window: WebWindow) -> Self
    {
        Self {
            phantom: PhantomData::default(),
            redraw_pending: RefCell::new(None),
            redraw_request_action: None,
            terminate_loop_action: None,
            canvas,
            window
        }
    }

    pub fn set_redraw_request_action<F>(&mut self, redraw_request_action: F)
    where
        F: FnMut() -> WebPending + 'static
    {
        self.redraw_request_action = Some(Box::new(RefCell::new(redraw_request_action)));
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
        S: Into<Vector2<u32>>
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

    pub fn set_fullscreen_mode(&self, _mode: WindowFullscreenMode)
    {
        // TODO
    }

    pub fn set_size_pixels<S: Into<Vector2<u32>>>(&self, _size: S)
    {
        // Do nothing
    }

    pub fn set_position_pixels<P: Into<Vector2<i32>>>(&self, _position: P)
    {
        // Do nothing
    }

    pub fn set_size_scaled_pixels<S: Into<Vector2<f32>>>(&self, _size: S)
    {
        // Do nothing
    }

    pub fn set_position_scaled_pixels<P: Into<Vector2<f32>>>(&self, _position: P)
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
        UserEventSender::new(UserEventSenderWeb::new())
    }
}

#[derive(Clone)]
pub struct UserEventSenderWeb<UserEventType>
where
    UserEventType: 'static
{
    phantom: PhantomData<UserEventType>
}

impl<UserEventType: 'static> UserEventSenderWeb<UserEventType>
{
    // TODO
    fn new() -> Self
    {
        Self {
            phantom: PhantomData::default()
        }
    }

    #[inline]
    pub fn send_event(&self, _event: UserEventType) -> Result<(), EventLoopSendError>
    {
        // TODO
        Ok(())
    }
}

pub struct WebCanvasImpl<UserEventType>
where
    UserEventType: 'static
{
    user_event_queue: Vec<UserEventType>,
    event_listeners_to_clean_up: Rc<RefCell<Vec<WebPending>>>
}

impl<UserEventType: 'static> WebCanvasImpl<UserEventType>
{
    pub fn new<S, H>(
        element_id: S,
        handler: H,
        _options: Option<WebCanvasAttachOptions>
    ) -> Result<Self, BacktraceError<ErrorMessage>>
    where
        S: AsRef<str>,
        H: WindowHandler<UserEventType> + 'static
    {
        let window = WebWindow::new()?;
        let document = window.document()?;

        let canvas = WebCanvasElement::new_by_id(&element_id)?;

        let initial_size_scaled = canvas.html_element().element().dimensions();
        let initial_dpr = window.device_pixel_ratio();

        let initial_size_unscaled =
            (initial_size_scaled * initial_dpr).round().into_u32();

        canvas.set_buffer_dimensions(&initial_size_unscaled);

        let mut event_listeners_to_clean_up = Vec::new();
        let is_pointer_locked = Rc::new(Cell::new(false));

        let renderer = GLRenderer::new_for_web_canvas_by_id(
            initial_size_unscaled.clone(),
            &element_id
        )
        .map_err(|err| ErrorMessage::msg_with_cause("Failed to create renderer", err))?;

        let handler = Rc::new(RefCell::new(DrawingWindowHandler::new(handler, renderer)));

        let helper = {
            Rc::new(RefCell::new(WindowHelper::new(WindowHelperWeb::new(
                canvas.clone(),
                window.clone()
            ))))
        };

        {
            let helper_inner = helper.clone();
            let window = window.clone();
            let handler = handler.clone();

            let frame_callback = RefCell::new(Closure::wrap(Box::new(move || {
                helper_inner
                    .borrow_mut()
                    .inner()
                    .clear_redraw_pending_flag();
                handler
                    .borrow_mut()
                    .on_draw(helper_inner.borrow_mut().deref_mut());
            })
                as Box<dyn FnMut()>));

            let redraw_request_action =
                move || window.request_animation_frame(&frame_callback).unwrap();

            helper
                .borrow_mut()
                .inner()
                .set_redraw_request_action(redraw_request_action);
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

            event_listeners_to_clean_up.push(
                window
                    .dyn_into_event_target()?
                    .register_event_listener_void("resize", move || {
                        let size_scaled = canvas.html_element().element().dimensions();
                        let dpr = window_inner.device_pixel_ratio();

                        let size_unscaled = (size_scaled * dpr).round().into_u32();

                        canvas.set_buffer_dimensions(&size_unscaled);

                        handler
                            .borrow_mut()
                            .on_resize(helper.borrow_mut().deref_mut(), size_unscaled);

                        handler
                            .borrow_mut()
                            .on_draw(helper.borrow_mut().deref_mut());
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
                    .dyn_into_event_target()?
                    .register_event_listener_void("pointerlockchange", move || {
                        let mouse_grabbed = canvas.is_pointer_lock_active();

                        is_pointer_locked.set(mouse_grabbed);

                        handler.borrow_mut().on_mouse_grab_status_changed(
                            helper.borrow_mut().deref_mut(),
                            mouse_grabbed
                        );
                    })?
            );
        }

        {
            let handler = handler.clone();
            let helper = helper.clone();

            event_listeners_to_clean_up.push(
                canvas_event_target.register_event_listener_mouse(
                    "mousemove",
                    move |event| {
                        let position = if is_pointer_locked.get() {
                            Vector2::new(event.movement_x(), event.movement_y())
                                .into_f32()
                        } else {
                            Vector2::new(event.offset_x(), event.offset_y()).into_f32()
                        };

                        handler
                            .borrow_mut()
                            .on_mouse_move(helper.borrow_mut().deref_mut(), position);
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
                        Some(button) => handler
                            .borrow_mut()
                            .on_mouse_button_down(helper.borrow_mut().deref_mut(), button)
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
                        Some(button) => handler
                            .borrow_mut()
                            .on_mouse_button_up(helper.borrow_mut().deref_mut(), button)
                    }
                )?
            );
        }

        let terminated = Rc::new(Cell::new(false));
        let event_listeners_to_clean_up =
            Rc::new(RefCell::new(event_listeners_to_clean_up));

        {
            let terminated = terminated.clone();
            let event_listeners_to_clean_up = event_listeners_to_clean_up.clone();

            helper
                .borrow_mut()
                .inner()
                .set_terminate_loop_action(move || {
                    log::info!("Terminating event loop");
                    terminated.set(true);
                    event_listeners_to_clean_up.borrow_mut().clear();
                });
        }

        log::info!(
            "Initial scaled canvas size: {:?}, dpr {}, unscaled: {:?}",
            initial_size_scaled,
            initial_dpr,
            initial_size_unscaled
        );

        handler.borrow_mut().on_start(
            helper.borrow_mut().deref_mut(),
            WindowStartupInfo::new(initial_size_unscaled, initial_dpr)
        );

        if !terminated.get() {
            handler
                .borrow_mut()
                .on_draw(helper.borrow_mut().deref_mut());
        }

        // TODO key events
        // TODO user events
        // TODO all remaining events

        Ok(WebCanvasImpl {
            user_event_queue: Vec::new(),
            event_listeners_to_clean_up
        })
    }
}

impl<UserEventType: 'static> Drop for WebCanvasImpl<UserEventType>
{
    fn drop(&mut self)
    {
        log::info!("Unregistering WebCanvasImpl")
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
