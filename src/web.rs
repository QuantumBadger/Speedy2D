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

use std::rc::Rc;

use wasm_bindgen::closure::{Closure, WasmClosure};
use wasm_bindgen::JsCast;
use web_sys::{Document, HtmlCanvasElement, HtmlElement, MouseEvent, Window};

use crate::dimen::Vector2;
use crate::error::{BacktraceError, ErrorMessage};
use crate::glbackend::GLBackendGlow;
use crate::glwrapper::GLVersion;
use crate::{GLRenderer, GLRendererCreationError};

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
                .ok_or_else(|| ErrorMessage::msg("Failed to get document"))?
        })
    }
}

pub struct WebDocument
{
    document: Document
}

impl WebDocument
{
    pub fn get_html_element_by_id<S: AsRef<str>>(
        &self,
        element_id: S
    ) -> Result<WebHtmlElement, BacktraceError<ErrorMessage>>
    {
        Ok(WebHtmlElement {
            element: self
                .document
                .get_element_by_id(element_id.as_ref())
                .ok_or_else(|| {
                    ErrorMessage::msg(format!(
                        "Failed to find element. The id ('{}') may be incorrect.",
                        element_id.as_ref()
                    ))
                })?
                .dyn_into::<HtmlElement>()
                .map_err(|err| {
                    ErrorMessage::msg(format!(
                        "Failed to convert Element to HtmlElement: '{:?}'",
                        err
                    ))
                })?
        })
    }
}

#[derive(Clone)]
pub struct WebHtmlElement
{
    element: HtmlElement
}

impl WebHtmlElement
{
    pub fn dyn_into_canvas(self)
        -> Result<WebCanvasElement, BacktraceError<ErrorMessage>>
    {
        let canvas_element = self.element.clone();

        Ok(WebCanvasElement {
            element: self,
            canvas: canvas_element
                .dyn_into::<HtmlCanvasElement>()
                .map_err(|err| {
                    ErrorMessage::msg(format!(
                        "Failed to convert element to canvas: '{:?}'",
                        err
                    ))
                })?
        })
    }

    pub fn register_event_listener_mouse<F: FnMut(MouseEvent) + 'static>(
        &self,
        listener_type: &str,
        callback: F
    ) -> Result<EventListener, BacktraceError<ErrorMessage>>
    {
        self.register_event_listener(
            listener_type,
            Box::new(callback) as Box<dyn FnMut(_)>
        )
    }

    fn register_event_listener<F: ?Sized + WasmClosure + 'static>(
        &self,
        listener_type: &str,
        callback: Box<F>
    ) -> Result<EventListener, BacktraceError<ErrorMessage>>
    {
        let closure = Closure::wrap(callback);

        self.element
            .add_event_listener_with_callback(
                listener_type,
                closure.as_ref().unchecked_ref()
            )
            .map_err(|err| {
                ErrorMessage::msg(format!(
                    "Failed to register {} callback: '{:?}'",
                    listener_type, err
                ))
            })?;

        let element = self.element.clone();
        let listener_type = listener_type.to_string();

        Ok(EventListener {
            unregister_action: Some(Box::new(move || {
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
        })
    }
}

#[derive(Clone)]
pub struct WebCanvasElement
{
    element: WebHtmlElement,
    canvas: HtmlCanvasElement
}

impl WebCanvasElement
{
    pub fn new_by_id<S: AsRef<str>>(
        canvas_id: S
    ) -> Result<WebCanvasElement, BacktraceError<ErrorMessage>>
    {
        Ok(WebWindow::new()?
            .document()?
            .get_html_element_by_id(canvas_id)?
            .dyn_into_canvas()?)
    }

    pub fn element(&self) -> &WebHtmlElement
    {
        &self.element
    }

    pub fn get_webgl2_context<V>(
        &self,
        viewport_size_pixels: V
    ) -> Result<GLRenderer, BacktraceError<GLRendererCreationError>>
    where
        V: Into<Vector2<u32>>
    {
        let context = self
            .canvas
            .get_context("webgl2")
            .map_err(|err| {
                GLRendererCreationError::msg(format!(
                    "Failed to get WebGL2 context: '{:?}'",
                    err
                ))
            })?
            .ok_or_else(|| GLRendererCreationError::msg("WebGL2 context not available"))?
            .dyn_into::<web_sys::WebGl2RenderingContext>()
            .map_err(|err| {
                GLRendererCreationError::msg(format!(
                    "Failed to convert object to rendering context: '{:?}'",
                    err
                ))
            })?;

        let gl_context = glow::Context::from_webgl2_context(context);

        GLRenderer::new_with_gl_backend(
            viewport_size_pixels,
            Rc::new(GLBackendGlow::new(gl_context)),
            GLVersion::WebGL2_0
        )
    }
}

#[must_use]
pub struct EventListener
{
    unregister_action: Option<Box<dyn FnOnce()>>
}

impl Drop for EventListener
{
    fn drop(&mut self)
    {
        self.unregister_action.take().unwrap()()
    }
}
