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

#[cfg(feature = "windowing")]
use std::cell::RefCell;
use std::fmt::Debug;
use std::rc::Rc;

#[cfg(feature = "windowing")]
use wasm_bindgen::closure::{Closure, WasmClosure};
use wasm_bindgen::JsCast;
#[cfg(feature = "windowing")]
use web_sys::{
    AddEventListenerOptions,
    EventTarget,
    KeyboardEvent,
    MediaQueryListEvent,
    MouseEvent
};
use web_sys::{Document, Element, HtmlCanvasElement, HtmlElement, Performance, Window};

use crate::dimen::UVec2;
#[cfg(feature = "windowing")]
use crate::dimen::Vector2;
use crate::error::{BacktraceError, ErrorMessage};
use crate::glbackend::GLBackendGlow;
use crate::glwrapper::GLVersion;
#[cfg(feature = "windowing")]
use crate::web::WebPendingStatus::{Active, AlreadyTriggered};
use crate::{GLRenderer, GLRendererCreationError};

#[allow(dead_code)]
#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub enum WebCursorType
{
    Auto,
    Default,
    None,
    Pointer,
    Progress,
    Wait,
    Cell,
    Crosshair,
    Text,
    VerticalText,
    Alias,
    Copy,
    Move,
    NoDrop,
    NotAllowed,
    Grab,
    Grabbing,
    ColResize,
    RowResize,
    EWResize,
    NSResize,
    NESWResize,
    NWSEResize,
    ZoomIn,
    ZoomOut
}

impl WebCursorType
{
    #[cfg(feature = "windowing")]
    fn css_text(&self) -> &'static str
    {
        match self {
            WebCursorType::Auto => "auto",
            WebCursorType::Default => "default",
            WebCursorType::None => "none",
            WebCursorType::Pointer => "pointer",
            WebCursorType::Progress => "progress",
            WebCursorType::Wait => "wait",
            WebCursorType::Cell => "cell",
            WebCursorType::Crosshair => "crosshair",
            WebCursorType::Text => "text",
            WebCursorType::VerticalText => "vertical-text",
            WebCursorType::Alias => "alias",
            WebCursorType::Copy => "copy",
            WebCursorType::Move => "move",
            WebCursorType::NoDrop => "no-drop",
            WebCursorType::NotAllowed => "not-allowed",
            WebCursorType::Grab => "grab",
            WebCursorType::Grabbing => "grabbing",
            WebCursorType::ColResize => "col-resize",
            WebCursorType::RowResize => "row-resize",
            WebCursorType::EWResize => "ew-resize",
            WebCursorType::NSResize => "ns-resize",
            WebCursorType::NESWResize => "nesw-resize",
            WebCursorType::NWSEResize => "nwse-resize",
            WebCursorType::ZoomIn => "zoom-in",
            WebCursorType::ZoomOut => "zoom-out"
        }
    }
}

#[derive(Clone)]
pub struct WebWindow
{
    window: Window
}

impl WebWindow
{
    pub fn new() -> Result<Self, BacktraceError<ErrorMessage>>
    {
        Ok(Self {
            window: web_sys::window()
                .ok_or_else(|| ErrorMessage::msg("Failed to get window"))?
        })
    }

    pub fn document(&self) -> Result<WebDocument, BacktraceError<ErrorMessage>>
    {
        Ok(WebDocument {
            document: self
                .window
                .document()
                .ok_or_else(|| ErrorMessage::msg("Failed to get document object"))?
        })
    }

    pub fn performance(&self) -> Result<WebPerformance, BacktraceError<ErrorMessage>>
    {
        Ok(WebPerformance {
            performance: self
                .window
                .performance()
                .ok_or_else(|| ErrorMessage::msg("Failed to get performance object"))?
        })
    }

    #[cfg(feature = "windowing")]
    pub fn match_media(
        &self,
        query: &str
    ) -> Result<WebEventTarget, BacktraceError<ErrorMessage>>
    {
        WebEventTarget::dyn_from(
            self.window
                .match_media(query)
                .map_err(|original| {
                    ErrorMessage::msg(format!("matchMedia() failed: {original:?}"))
                })?
                .ok_or_else(|| ErrorMessage::msg("matchMedia() returned null"))?
        )
    }

    #[cfg(feature = "windowing")]
    pub fn request_animation_frame<T: ?Sized + 'static>(
        &self,
        callback: &RefCell<Closure<T>>
    ) -> Result<WebPending, BacktraceError<ErrorMessage>>
    {
        let frame_id: i32 = self
            .window
            .request_animation_frame(callback.borrow_mut().as_ref().unchecked_ref())
            .map_err(|err| {
                ErrorMessage::msg(format!("Failed to request animation frame: {err:?}"))
            })?;

        let window = self.window.clone();

        Ok(WebPending::new_with_status(move |status| {
            if status == Active {
                if let Err(err) = window.cancel_animation_frame(frame_id) {
                    log::error!("Failed to cancel animation frame: {err:?}")
                } else {
                    log::info!("Cancelled animation frame {frame_id}")
                }
            }
        }))
    }

    #[cfg(feature = "windowing")]
    pub fn set_timeout_immediate<T: ?Sized + 'static>(
        &self,
        callback: &RefCell<Closure<T>>
    ) -> Result<WebPending, BacktraceError<ErrorMessage>>
    {
        let timeout_id: i32 = self
            .window
            .set_timeout_with_callback(callback.borrow_mut().as_ref().unchecked_ref())
            .map_err(|err| {
                ErrorMessage::msg(format!("Failed to request animation frame: {err:?}"))
            })?;

        let window = self.window.clone();

        Ok(WebPending::new_with_status(move |status| {
            if status == Active {
                window.clear_timeout_with_handle(timeout_id);
                log::info!("Cancelled timeout {}", timeout_id);
            }
        }))
    }

    #[cfg(feature = "windowing")]
    pub fn device_pixel_ratio(&self) -> f64
    {
        self.window.device_pixel_ratio()
    }

    #[cfg(feature = "windowing")]
    pub fn dyn_into_event_target(
        self
    ) -> Result<WebEventTarget, BacktraceError<ErrorMessage>>
    {
        WebEventTarget::dyn_from(self.window)
    }
}

#[derive(Clone)]
pub struct WebDocument
{
    document: Document
}

impl WebDocument
{
    pub fn get_element_by_id<S: AsRef<str>>(
        &self,
        element_id: S
    ) -> Result<WebElement, BacktraceError<ErrorMessage>>
    {
        Ok(WebElement {
            document: self.clone(),
            element: self
                .document
                .get_element_by_id(element_id.as_ref())
                .ok_or_else(|| {
                    ErrorMessage::msg(format!(
                        "Failed to find element. The id ('{}') may be incorrect.",
                        element_id.as_ref()
                    ))
                })?
        })
    }

    #[cfg(feature = "windowing")]
    pub fn pointer_lock_element(&self) -> Option<WebElement>
    {
        self.document
            .pointer_lock_element()
            .map(|element| WebElement {
                document: self.clone(),
                element
            })
    }

    #[cfg(feature = "windowing")]
    pub fn fullscreen_element(&self) -> Option<WebElement>
    {
        self.document
            .fullscreen_element()
            .map(|element| WebElement {
                document: self.clone(),
                element
            })
    }

    #[cfg(feature = "windowing")]
    pub fn dyn_into_event_target(
        self
    ) -> Result<WebEventTarget, BacktraceError<ErrorMessage>>
    {
        WebEventTarget::dyn_from(self.document)
    }

    #[cfg(feature = "windowing")]
    pub fn set_title(&self, title: &str)
    {
        self.document.set_title(title);
    }

    #[cfg(feature = "windowing")]
    pub fn exit_pointer_lock(&self)
    {
        self.document.exit_pointer_lock()
    }

    #[cfg(feature = "windowing")]
    pub fn exit_fullscreen(&self)
    {
        self.document.exit_fullscreen();
    }
}

#[derive(Clone)]
pub struct WebPerformance
{
    performance: Performance
}

impl WebPerformance
{
    #[inline]
    pub fn now(&self) -> f64
    {
        self.performance.now()
    }
}

#[derive(Clone)]
pub struct WebElement
{
    #[allow(dead_code)]
    document: WebDocument,
    element: Element
}

impl WebElement
{
    pub fn dyn_into_html_element(
        self
    ) -> Result<WebHtmlElement, BacktraceError<ErrorMessage>>
    {
        let element = self.clone();

        Ok(WebHtmlElement {
            element,
            html_element: self.element.dyn_into::<HtmlElement>().map_err(|err| {
                ErrorMessage::msg(format!(
                    "Failed to convert Element to HtmlElement: '{err:?}'"
                ))
            })?
        })
    }

    #[cfg(feature = "windowing")]
    pub fn dyn_into_event_target(
        self
    ) -> Result<WebEventTarget, BacktraceError<ErrorMessage>>
    {
        WebEventTarget::dyn_from(self.element)
    }

    #[cfg(feature = "windowing")]
    pub fn dimensions(&self) -> Vector2<f64>
    {
        let bounding_rect = self.element.get_bounding_client_rect();

        Vector2::new(
            bounding_rect.right() - bounding_rect.left(),
            bounding_rect.bottom() - bounding_rect.top()
        )
    }

    #[cfg(feature = "windowing")]
    #[inline]
    pub fn document(&self) -> &WebDocument
    {
        &self.document
    }
}

impl PartialEq for WebElement
{
    fn eq(&self, other: &Self) -> bool
    {
        self.element == other.element
    }
}

impl Eq for WebElement {}

#[derive(Clone)]
pub struct WebHtmlElement
{
    #[allow(dead_code)]
    element: WebElement,
    html_element: HtmlElement
}

impl WebHtmlElement
{
    #[cfg(feature = "windowing")]
    #[inline]
    pub fn element(&self) -> &WebElement
    {
        &self.element
    }

    pub fn dyn_into_canvas(self)
        -> Result<WebCanvasElement, BacktraceError<ErrorMessage>>
    {
        let canvas = self
            .html_element
            .clone()
            .dyn_into::<HtmlCanvasElement>()
            .map_err(|err| {
                ErrorMessage::msg(format!(
                    "Failed to convert element to canvas: '{err:?}'"
                ))
            })?;

        Ok(WebCanvasElement {
            html_element: self,
            canvas
        })
    }

    #[cfg(feature = "windowing")]
    #[inline]
    pub fn document(&self) -> &WebDocument
    {
        self.element.document()
    }
}

#[derive(Clone)]
pub struct WebCanvasElement
{
    #[allow(dead_code)]
    html_element: WebHtmlElement,
    canvas: HtmlCanvasElement
}

impl WebCanvasElement
{
    pub fn new_by_id<S: AsRef<str>>(
        canvas_id: S
    ) -> Result<WebCanvasElement, BacktraceError<ErrorMessage>>
    {
        WebWindow::new()?
            .document()?
            .get_element_by_id(canvas_id)?
            .dyn_into_html_element()?
            .dyn_into_canvas()
    }

    #[cfg(feature = "windowing")]
    pub fn html_element(&self) -> &WebHtmlElement
    {
        &self.html_element
    }

    pub fn get_webgl2_context<V>(
        &self,
        viewport_size_pixels: V
    ) -> Result<GLRenderer, BacktraceError<GLRendererCreationError>>
    where
        V: Into<UVec2>
    {
        let viewport_size_pixels = viewport_size_pixels.into();

        log::info!(
            "Getting WebGL2 context for viewport size {:?}",
            viewport_size_pixels
        );

        let context = self
            .canvas
            .get_context("webgl2")
            .map_err(|err| {
                GLRendererCreationError::msg(format!(
                    "Failed to get WebGL2 context: '{err:?}'"
                ))
            })?
            .ok_or_else(|| GLRendererCreationError::msg("WebGL2 context not available"))?
            .dyn_into::<web_sys::WebGl2RenderingContext>()
            .map_err(|err| {
                GLRendererCreationError::msg(format!(
                    "Failed to convert object to rendering context: '{err:?}'"
                ))
            })?;

        let gl_context = glow::Context::from_webgl2_context(context);

        GLRenderer::new_with_gl_backend(
            viewport_size_pixels,
            Rc::new(GLBackendGlow::new(gl_context)),
            GLVersion::WebGL2_0
        )
    }

    #[cfg(feature = "windowing")]
    pub fn set_buffer_dimensions(&self, size: &UVec2)
    {
        self.canvas.set_width(size.x);
        self.canvas.set_height(size.y);
    }

    #[cfg(feature = "windowing")]
    pub fn set_tab_index(&self, index: i32)
    {
        self.canvas.set_tab_index(index);
    }

    #[cfg(feature = "windowing")]
    pub fn set_cursor(&self, cursor: WebCursorType)
    {
        if let Err(err) = self
            .canvas
            .style()
            .set_property("cursor", cursor.css_text())
        {
            log::info!("Failed to set cursor: {:?}", err);
        }
    }

    #[cfg(feature = "windowing")]
    pub fn request_pointer_lock(&self)
    {
        self.canvas.request_pointer_lock();
    }

    #[cfg(feature = "windowing")]
    pub fn is_pointer_lock_active(&self) -> bool
    {
        match self.html_element.document().pointer_lock_element() {
            None => false,
            Some(lock_elem) => lock_elem == *self.html_element().element()
        }
    }

    #[cfg(feature = "windowing")]
    pub fn is_fullscreen_active(&self) -> bool
    {
        match self.html_element.document().fullscreen_element() {
            None => false,
            Some(lock_elem) => lock_elem == *self.html_element().element()
        }
    }

    #[cfg(feature = "windowing")]
    pub fn request_fullscreen(&self)
    {
        if let Err(err) = self.canvas.request_fullscreen() {
            log::error!("Failed to request fullscreen mode: {:?}", err);
        }
    }
}

#[cfg(feature = "windowing")]
#[must_use]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
enum WebPendingStatus
{
    Active,
    AlreadyTriggered
}

#[cfg(feature = "windowing")]
#[must_use]
pub struct WebPending
{
    unregister_action: Option<Box<dyn FnOnce(WebPendingStatus)>>,
    status: WebPendingStatus
}

#[cfg(feature = "windowing")]
impl WebPending
{
    pub fn new<F: FnOnce() + 'static>(unregister_action: F) -> Self
    {
        Self::new_with_status(move |_status| unregister_action())
    }

    fn new_with_status<F: FnOnce(WebPendingStatus) + 'static>(
        unregister_action: F
    ) -> Self
    {
        Self {
            unregister_action: Some(Box::new(unregister_action)),
            status: Active
        }
    }

    pub fn mark_as_triggered(&mut self)
    {
        self.status = AlreadyTriggered
    }
}

#[cfg(feature = "windowing")]
impl Drop for WebPending
{
    fn drop(&mut self)
    {
        self.unregister_action.take().unwrap()(self.status)
    }
}

#[cfg(feature = "windowing")]
#[derive(Clone)]
pub struct WebEventTarget
{
    target: EventTarget
}

#[cfg(feature = "windowing")]
impl WebEventTarget
{
    fn dyn_from<E: Debug + JsCast>(
        element: E
    ) -> Result<Self, BacktraceError<ErrorMessage>>
    {
        Ok(WebEventTarget {
            target: element.dyn_into().map_err(|original| {
                ErrorMessage::msg(format!("Failed to cast to EventTarget: {original:?}"))
            })?
        })
    }

    pub fn register_event_listener_void<F: FnMut() + 'static>(
        &self,
        listener_type: &str,
        callback: F
    ) -> Result<WebPending, BacktraceError<ErrorMessage>>
    {
        self.register_event_listener(
            listener_type,
            Box::new(callback) as Box<dyn FnMut()>,
            false
        )
    }

    pub fn register_event_listener_mouse<F: FnMut(MouseEvent) + 'static>(
        &self,
        listener_type: &str,
        callback: F
    ) -> Result<WebPending, BacktraceError<ErrorMessage>>
    {
        self.register_event_listener(
            listener_type,
            Box::new(callback) as Box<dyn FnMut(_)>,
            false
        )
    }

    pub fn register_event_listener_keyboard<F: FnMut(KeyboardEvent) + 'static>(
        &self,
        listener_type: &str,
        callback: F
    ) -> Result<WebPending, BacktraceError<ErrorMessage>>
    {
        self.register_event_listener(
            listener_type,
            Box::new(callback) as Box<dyn FnMut(_)>,
            false
        )
    }

    pub fn register_event_listener_media_event_list_once<
        F: FnMut(MediaQueryListEvent) + 'static
    >(
        &self,
        listener_type: &str,
        callback: F
    ) -> Result<WebPending, BacktraceError<ErrorMessage>>
    {
        self.register_event_listener(
            listener_type,
            Box::new(callback) as Box<dyn FnMut(_)>,
            true
        )
    }

    fn register_event_listener<F: ?Sized + WasmClosure + 'static>(
        &self,
        listener_type: &str,
        callback: Box<F>,
        once: bool
    ) -> Result<WebPending, BacktraceError<ErrorMessage>>
    {
        let closure = Closure::wrap(callback);

        self.target
            .add_event_listener_with_callback_and_add_event_listener_options(
                listener_type,
                closure.as_ref().unchecked_ref(),
                AddEventListenerOptions::new().once(once)
            )
            .map_err(|err| {
                ErrorMessage::msg(format!(
                    "Failed to register {listener_type} callback: '{err:?}'"
                ))
            })?;

        let element = self.target.clone();
        let listener_type = listener_type.to_string();

        Ok(WebPending::new_with_status(move |_status| {
            element
                .remove_event_listener_with_callback(
                    listener_type.as_ref(),
                    closure.as_ref().unchecked_ref()
                )
                .unwrap_or_else(|err| {
                    log::error!(
                        "Failed to remove '{}' event handler: {:?}",
                        listener_type,
                        err
                    )
                })
        }))
    }
}
