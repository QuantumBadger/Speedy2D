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

use std::cell::RefCell;
use std::marker::PhantomData;
use std::ops::DerefMut;
use std::rc::Rc;

use crate::dimen::Vector2;
use crate::error::{BacktraceError, ErrorMessage};
use crate::web::{EventListener, WebCanvasElement};
use crate::window::{
    EventLoopSendError,
    UserEventSender,
    WindowFullscreenMode,
    WindowHandler,
    WindowHelper
};

pub struct WindowHelperWeb<UserEventType>
where
    UserEventType: 'static
{
    phantom: PhantomData<UserEventType>
}

// TODO remove allow
#[allow(unused_variables)]
impl<UserEventType: 'static> WindowHelperWeb<UserEventType>
{
    fn new() -> Self
    {
        Self {
            phantom: PhantomData::default()
        }
    }

    pub fn terminate_loop(&mut self)
    {
        // TODO
    }

    pub fn set_icon_from_rgba_pixels<S>(
        &self,
        data: Vec<u8>,
        size: S
    ) -> Result<(), BacktraceError<ErrorMessage>>
    where
        S: Into<Vector2<u32>>
    {
        // Do nothing
        Err(ErrorMessage::msg("Cannot set icon for WebCanvas"))
    }

    pub fn set_cursor_visible(&self, visible: bool)
    {
        // TODO
    }

    pub fn set_cursor_grab(
        &self,
        grabbed: bool
    ) -> Result<(), BacktraceError<ErrorMessage>>
    {
        // TODO
        Ok(())
    }

    pub fn set_resizable(&self, resizable: bool)
    {
        // Do nothing
    }

    #[inline]
    pub fn request_redraw(&self)
    {
        // TODO
    }

    pub fn set_title(&self, title: &str)
    {
        // TODO
    }

    pub fn set_fullscreen_mode(&self, mode: WindowFullscreenMode)
    {
        // TODO
    }

    pub fn set_size_pixels<S: Into<Vector2<u32>>>(&self, size: S)
    {
        // Do nothing
    }

    pub fn set_position_pixels<P: Into<Vector2<i32>>>(&self, position: P)
    {
        // Do nothing
    }

    /// Sets the window size in scaled device-independent pixels. This is the
    /// window's inner size, excluding the border.
    pub fn set_size_scaled_pixels<S: Into<Vector2<f32>>>(&self, size: S)
    {
        // Do nothing
    }

    pub fn set_position_scaled_pixels<P: Into<Vector2<f32>>>(&self, position: P)
    {
        // Do nothing
    }

    #[inline]
    #[must_use]
    pub fn get_scale_factor(&self) -> f64
    {
        // TODO
        0.0
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

// TODO remove allow
#[allow(unused_variables)]
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
    pub fn send_event(&self, event: UserEventType) -> Result<(), EventLoopSendError>
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
    event_listeners_to_clean_up: Vec<EventListener>
}

impl<UserEventType: 'static> WebCanvasImpl<UserEventType>
{
    pub fn new<S, H>(
        element_id: S,
        handler: H
    ) -> Result<Self, BacktraceError<ErrorMessage>>
    where
        S: AsRef<str>,
        H: WindowHandler<UserEventType> + 'static
    {
        let canvas = WebCanvasElement::new_by_id(element_id)?;

        let mut event_listeners_to_clean_up = Vec::new();

        let handler = Rc::new(RefCell::new(handler));

        // TODO invoke on_start
        // TODO handle event loop ending

        let helper = Rc::new(RefCell::new(WindowHelper::new(WindowHelperWeb::new())));

        {
            let handler = handler.clone();
            let helper = helper.clone();

            event_listeners_to_clean_up.push(
                canvas.element().register_event_listener_mouse(
                    "mousemove",
                    move |event| {
                        handler.borrow_mut().on_mouse_move(
                            helper.borrow_mut().deref_mut(),
                            Vector2::new(
                                event.offset_x() as f32,
                                event.offset_y() as f32
                            )
                        )
                    }
                )?
            );
        }

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
