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

//! Hardware-accelerated drawing of shapes, images, and text, with an easy to
//! use API.
//!
//! Speedy2D aims to be:
//!
//!  - The simplest Rust API for creating a window, rendering graphics/text, and
//!    handling input
//!  - Compatible with any device supporting OpenGL 2.0+ or WebGL 2.0. Support
//!    for OpenGL ES 2.0+ is planned.
//!  - Very fast
//!
//! Supports Windows, Mac, Linux, and WebGL. Support for Android and iOS is in
//! development.
//!
//! By default, Speedy2D contains support for setting up a window with an OpenGL
//! context. If you'd like to handle this yourself, and use Speedy2D only for
//! rendering, you can disable the `windowing` feature.
//!
//! # Useful Links
//!
//! * [Source repository](https://github.com/QuantumBadger/Speedy2D)
//! * [Crate](https://crates.io/crates/speedy2d)
//!
//! # Getting Started (Windows/Mac/Linux)
//!
//! ## Create a window
//!
//! After adding Speedy2D to your Cargo.toml dependencies, create a window as
//! follows:
//!
//! ```rust,no_run
//! use speedy2d::Window;
//!
//! let window = Window::new_centered("Title", (640, 480)).unwrap();
//! ```
//!
//! You may also use [Window::new_fullscreen_borderless()],
//! [Window::new_with_options()], or [Window::new_with_user_events()].
//!
//! ## Implement the callbacks
//!
//! Create a struct implementing the `WindowHandler` trait. Override
//! whichever callbacks you're interested in, for example `on_draw()`,
//! `on_mouse_move()`, or `on_key_down()`.
//!
//! ```
//! use speedy2d::window::{WindowHandler, WindowHelper};
//! use speedy2d::Graphics2D;
//!
//! struct MyWindowHandler {}
//!
//! impl WindowHandler for MyWindowHandler
//! {
//!     fn on_draw(&mut self, helper: &mut WindowHelper, graphics: &mut Graphics2D)
//!     {
//!         // Draw things here using `graphics`
//!     }
//! }
//! ```
//!
//! The full list of possible callbacks is currently as follows. See
//! [WindowHandler] for full documentation.
//!
//! It's only necessary to implement the callbacks you actually want to use. The
//! default implementation will do nothing and continue the event loop.
//!
//! ```text
//! fn on_start()
//! fn on_user_event()
//! fn on_resize()
//! fn on_scale_factor_changed()
//! fn on_draw()
//! fn on_mouse_move()
//! fn on_mouse_button_down()
//! fn on_mouse_button_up()
//! fn on_key_down()
//! fn on_key_up()
//! fn on_keyboard_char()
//! fn on_keyboard_modifiers_changed()
//! ```
//!
//! Each callback gives you a [window::WindowHelper] instance, which
//! lets you perform window-related actions, like requesting that a new frame is
//! drawn using [window::WindowHelper::request_redraw()].
//!
//! Note: Unless you call [window::WindowHelper::request_redraw()], frames will
//! only be drawn when necessary, for example when resizing the window.
//!
//! ## Render some graphics
//!
//! The [WindowHandler::on_draw()] callback gives you a [Graphics2D]
//! instance, which lets you draw shapes, text, and images.
//!
//! ```
//! # use speedy2d::window::{WindowHandler, WindowHelper};
//! # use speedy2d::Graphics2D;
//! # use speedy2d::color::Color;
//! #
//! # struct MyWindowHandler {}
//! #
//! # impl WindowHandler for MyWindowHandler
//! # {
//!     fn on_draw(&mut self, helper: &mut WindowHelper, graphics: &mut Graphics2D)
//!     {
//!         graphics.clear_screen(Color::from_rgb(0.8, 0.9, 1.0));
//!         graphics.draw_circle((100.0, 100.0), 75.0, Color::BLUE);
//!
//!         // Request that we draw another frame once this one has finished
//!         helper.request_redraw();
//!     }
//! # }
//! ```
//!
//! ## Start it running!
//!
//! Once you've implemented the callbacks you're interested in, start the event
//! loop running with [Window::run_loop()]:
//!
//! ```rust,no_run
//! # use speedy2d::Window;
//! # struct MyWindowHandler {}
//! # impl speedy2d::window::WindowHandler for MyWindowHandler {}
//! let window = Window::<()>::new_centered("Title", (640, 480)).unwrap();
//!
//! window.run_loop(MyWindowHandler{});
//! ```
//!
//! ## Alternative: Managing the GL context yourself
//!
//! If you'd rather handle the window creation and OpenGL context management
//! yourself, simply disable Speedy2D's `windowing` feature in your `Cargo.toml`
//! file, and create a context as follows. You will need to specify a loader
//! function to allow Speedy2D to obtain the OpenGL function pointers.
//!
//! ```rust,no_run
//! use speedy2d::GLRenderer;
//! # struct WindowContext {}
//! # impl WindowContext {
//! #     fn get_proc_address(&self, fn_name: &str) -> *const std::ffi::c_void
//! #     {
//! #         std::ptr::null()
//! #     }
//! # }
//! # let window_context = WindowContext {};
//!
//! let mut renderer = unsafe {
//!     GLRenderer::new_for_gl_context((640, 480), |fn_name| {
//!         window_context.get_proc_address(fn_name) as *const _
//!     })
//! }.unwrap();
//! ```
//!
//! Then, draw a frame using [GLRenderer::draw_frame()]:
//!
//! ```rust,no_run
//! # use speedy2d::GLRenderer;
//! # use speedy2d::color::Color;
//! # let mut renderer = unsafe {
//! #     GLRenderer::new_for_gl_context((640, 480), |fn_name| {
//! #         std::ptr::null() as *const _
//! #     })
//! # }.unwrap();
//! renderer.draw_frame(|graphics| {
//!     graphics.clear_screen(Color::WHITE);
//!     graphics.draw_circle((100.0, 100.0), 75.0, Color::BLUE);
//! });
//! ```
//!
//! # Laying out text
//!
//! To render text, a font must be created. Call [font::Font::new()] with the
//! bytes from the TTF or OTF font file.
//!
//! (note: OTF support may be limited)
//!
//! ```rust,no_run
//! use speedy2d::font::Font;
//!
//! let bytes = include_bytes!("../assets/fonts/NotoSans-Regular.ttf");
//! let font = Font::new(bytes).unwrap();
//! ```
//!
//! Then, invoke `font.layout_text()` (part of the [font::TextLayout] trait) to
//! calculate the necessary line breaks and spacing. This will give you
//! a [font::FormattedTextBlock].
//!
//! ```rust,no_run
//! # use speedy2d::font::{Font, TextOptions};
//! # let font = Font::new(&[]).unwrap();
//! use speedy2d::font::TextLayout;
//!
//! let block = font.layout_text("Hello World", 32.0, TextOptions::new());
//! ```
//!
//! Finally, call [Graphics2D::draw_text()] to draw the text block!
//!
//! ```rust,no_run
//! # use speedy2d::GLRenderer;
//! # use speedy2d::color::Color;
//! # use speedy2d::font::{Font, TextOptions, TextLayout};
//! # let font = Font::new(&[]).unwrap();
//! # let block = font.layout_text("Hello World", 32.0, TextOptions::new());
//! # let mut renderer = unsafe {
//! #     GLRenderer::new_for_gl_context((640, 480), |fn_name| {
//! #         std::ptr::null() as *const _
//! #     })
//! # }.unwrap();
//! # renderer.draw_frame(|graphics| {
//! graphics.draw_text((100.0, 100.0), Color::BLUE, &block);
//! # });
//! ```
//!
//! ## Word wrap
//!
//! To wrap lines of text to a certain width, use
//! [font::TextOptions::with_wrap_to_width()]:
//!
//! ```rust,no_run
//! # use speedy2d::font::{Font, TextOptions};
//! # let font = Font::new(&[]).unwrap();
//! use speedy2d::font::{TextLayout, TextAlignment};
//!
//! let block = font.layout_text(
//!     "The quick brown fox jumps over the lazy dog.",
//!     32.0,
//!     TextOptions::new().with_wrap_to_width(300.0, TextAlignment::Left));
//! ```
//!
//! # Loading images
//!
//! Image files (in formats such as PNG, JPG, and BMP) can be loaded using the
//! following APIs, available in both `Graphics2D` and `GLRenderer`.
//!
//! * [Graphics2D::create_image_from_file_path()]
//! * [Graphics2D::create_image_from_file_bytes()]
//! * [GLRenderer::create_image_from_file_path()]
//! * [GLRenderer::create_image_from_file_bytes()]
//!
//! Alternatively, you can create an image from raw pixel data, using:
//!
//! * [Graphics2D::create_image_from_raw_pixels()]
//! * [GLRenderer::create_image_from_raw_pixels()]
//!
//! # Getting Started (WebGL)
//!
//! To use Speedy2D with WebGL, your app must be compiled for WebAssembly.
//! Speedy2D can attach itself to a `canvas` on the page using an ID you
//! specify.
//!
//! As with Windows/Mac/Linux targets, it's possible to use Speedy2D either in a
//! full rendering and event handling configuation, or for rendering only.
//!
//! For rendering only, use the following API:
//!
//! * [GLRenderer::new_for_web_canvas_by_id()]
//!
//! For full keyboard/mouse/etc event handling in addition to rendering, use:
//!
//! * [WebCanvas::new_for_id()]
//! * [WebCanvas::new_for_id_with_user_events()]
//!
//! After initialization, the usual [WindowHandler] callbacks and
//! [window::WindowHelper]/[Graphics2D] APIs should operate as on other
//! platforms.
//!
//! For an example, see the `examples/webgl` directory. To build this, first
//! install the prerequisites:
//!
//! ```shell
//! cargo install wasm-bindgen-cli just
//! ```
//!
//! Then use the following command to run the build:
//!
//! ```shell
//! just build-example-webgl
//! ```

#![deny(warnings)]
#![deny(missing_docs)]

use std::fmt::{Display, Formatter};
#[cfg(any(doc, doctest, all(target_arch = "wasm32", feature = "windowing")))]
use std::marker::PhantomData;
use std::rc::Rc;

#[cfg(any(feature = "image-loading", doc, doctest))]
use {
    crate::image::ImageFileFormat,
    std::io::{BufRead, Seek},
    std::path::Path
};

use crate::color::Color;
use crate::dimen::{UVec2, Vec2};
use crate::error::{BacktraceError, ErrorMessage};
use crate::font::FormattedTextBlock;
use crate::glbackend::GLBackend;
#[cfg(not(target_arch = "wasm32"))]
use crate::glbackend::GLBackendGlow;
use crate::glwrapper::{GLContextManager, GLVersion};
use crate::image::{ImageDataType, ImageHandle, ImageSmoothingMode, RawBitmapData};
use crate::renderer2d::Renderer2D;
use crate::shape::{Polygon, Rect, Rectangle};
#[cfg(target_arch = "wasm32")]
use crate::web::WebCanvasElement;
#[cfg(any(doc, doctest, feature = "windowing"))]
use crate::window::WindowHandler;
#[cfg(any(doc, doctest, all(feature = "windowing", not(target_arch = "wasm32"))))]
use crate::window::{
    UserEventSender,
    WindowCreationError,
    WindowCreationOptions,
    WindowPosition,
    WindowSize
};
#[cfg(any(doc, doctest))]
use crate::window_internal_doctest::{WebCanvasImpl, WindowGlutin};
#[cfg(all(
    feature = "windowing",
    not(target_arch = "wasm32"),
    not(any(doc, doctest))
))]
use crate::window_internal_glutin::WindowGlutin;
#[cfg(all(feature = "windowing", target_arch = "wasm32", not(any(doc, doctest))))]
use crate::window_internal_web::WebCanvasImpl;

/// Types representing colors.
pub mod color;

/// Types representing shapes.
pub mod shape;

/// Components for loading fonts and laying out text.
pub mod font;

/// Types representing sizes and positions.
pub mod dimen;

/// Utilities and traits for numeric values.
pub mod numeric;

/// Error types.
pub mod error;

/// Types relating to images.
pub mod image;

/// Utilities for accessing the system clock on all platforms.
pub mod time;

/// Allows for the creation and management of windows.
#[cfg(any(doc, doctest, feature = "windowing"))]
pub mod window;

#[cfg(all(
    feature = "windowing",
    not(target_arch = "wasm32"),
    not(any(doc, doctest))
))]
mod window_internal_glutin;

#[cfg(all(feature = "windowing", target_arch = "wasm32", not(any(doc, doctest))))]
mod window_internal_web;

#[cfg(any(doc, doctest))]
mod window_internal_doctest;

#[cfg(target_arch = "wasm32")]
mod web;

mod font_cache;
mod glbackend;
mod glwrapper;
mod renderer2d;
mod texture_packer;
mod utils;

/// An error encountered during the creation of a [GLRenderer].
#[derive(Clone, Debug)]
pub struct GLRendererCreationError
{
    description: String
}

impl GLRendererCreationError
{
    fn msg_with_cause<S, Cause>(description: S, cause: Cause) -> BacktraceError<Self>
    where
        S: AsRef<str>,
        Cause: std::error::Error + 'static
    {
        BacktraceError::new_with_cause(
            Self {
                description: description.as_ref().to_string()
            },
            cause
        )
    }

    #[allow(dead_code)]
    fn msg<S>(description: S) -> BacktraceError<Self>
    where
        S: AsRef<str>
    {
        BacktraceError::new(Self {
            description: description.as_ref().to_string()
        })
    }
}

impl Display for GLRendererCreationError
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result
    {
        Display::fmt("GL renderer creation error: ", f)?;
        Display::fmt(&self.description, f)
    }
}

/// A graphics renderer using an OpenGL backend.
///
/// Note: There is no need to use this struct if you are letting Speedy2D create
/// a window for you.
pub struct GLRenderer
{
    context: GLContextManager,
    renderer: Graphics2D
}

impl GLRenderer
{
    /// Creates a `GLRenderer` with the specified OpenGL loader function. The
    /// loader function takes the name of an OpenGL function, and returns the
    /// associated function pointer. `viewport_size_pixels` should be set to
    /// the initial viewport size, however this can be changed later using
    /// [GLRenderer:: set_viewport_size_pixels()].
    ///
    /// Note: This function must not be called if you are letting Speedy2D
    /// create a window for you.
    ///
    /// # Safety
    ///
    /// While a `GLRenderer` object is active, you must not make any changes to
    /// the active GL context. Doing so may lead to undefined behavior,
    /// which is why this function is marked `unsafe`. It is strongly
    /// advised not to use any other OpenGL libraries in the same thread
    /// as `GLRenderer`.
    #[cfg(not(target_arch = "wasm32"))]
    pub unsafe fn new_for_gl_context<V, F>(
        viewport_size_pixels: V,
        loader_function: F
    ) -> Result<Self, BacktraceError<GLRendererCreationError>>
    where
        V: Into<UVec2>,
        F: FnMut(&str) -> *const std::os::raw::c_void
    {
        let backend =
            GLBackendGlow::new(glow::Context::from_loader_function(loader_function));

        Self::new_with_gl_backend(
            viewport_size_pixels,
            Rc::new(backend),
            GLVersion::OpenGL2_0
        )
    }

    /// Creates a `GLRenderer` for the specified HTML canvas. The canvas
    /// will be found based on the specified ID.
    ///
    /// The parameter `viewport_size_pixels` should be set to
    /// the initial canvas size, however this can be changed later using
    /// [GLRenderer:: set_viewport_size_pixels()].
    #[cfg(any(doc, doctest, target_arch = "wasm32"))]
    pub fn new_for_web_canvas_by_id<V, S>(
        viewport_size_pixels: V,
        element_id: S
    ) -> Result<Self, BacktraceError<GLRendererCreationError>>
    where
        V: Into<UVec2>,
        S: AsRef<str>
    {
        WebCanvasElement::new_by_id(element_id.as_ref())
            .map_err(|err| {
                GLRendererCreationError::msg_with_cause("Failed to get canvas", err)
            })?
            .get_webgl2_context(viewport_size_pixels)
    }

    fn new_with_gl_backend<V: Into<UVec2>>(
        viewport_size_pixels: V,
        gl_backend: Rc<dyn GLBackend>,
        gl_version: GLVersion
    ) -> Result<Self, BacktraceError<GLRendererCreationError>>
    {
        let viewport_size_pixels = viewport_size_pixels.into();

        let context =
            GLContextManager::create(gl_backend, gl_version).map_err(|err| {
                GLRendererCreationError::msg_with_cause(
                    "GL context manager creation failed",
                    err
                )
            })?;

        let renderer = Graphics2D {
            renderer: Renderer2D::new(&context, viewport_size_pixels).map_err(|err| {
                GLRendererCreationError::msg_with_cause("Renderer2D creation failed", err)
            })?
        };

        Ok(GLRenderer { context, renderer })
    }

    /// Sets the renderer viewport to the specified pixel size, in response to a
    /// change in the window size.
    pub fn set_viewport_size_pixels(&mut self, viewport_size_pixels: UVec2)
    {
        self.renderer
            .renderer
            .set_viewport_size_pixels(viewport_size_pixels)
    }

    /// Creates a new [ImageHandle] from the specified raw pixel data.
    ///
    /// The data provided in the `data` parameter must be in the format
    /// specified by `data_type`.
    ///
    /// The returned [ImageHandle] is valid only for the current graphics
    /// context.
    pub fn create_image_from_raw_pixels(
        &mut self,
        data_type: ImageDataType,
        smoothing_mode: ImageSmoothingMode,
        size: UVec2,
        data: &[u8]
    ) -> Result<ImageHandle, BacktraceError<ErrorMessage>>
    {
        self.renderer
            .create_image_from_raw_pixels(data_type, smoothing_mode, size, data)
    }

    /// Loads an image from the specified file path.
    ///
    /// If no `data_type` is provided, an attempt will be made to guess the file
    /// format.
    ///
    /// For a list of supported image types, see [image::ImageFileFormat].
    ///
    /// The returned [ImageHandle] is valid only for the current graphics
    /// context.
    #[cfg(any(feature = "image-loading", doc, doctest))]
    pub fn create_image_from_file_path<S: AsRef<Path>>(
        &mut self,
        data_type: Option<ImageFileFormat>,
        smoothing_mode: ImageSmoothingMode,
        path: S
    ) -> Result<ImageHandle, BacktraceError<ErrorMessage>>
    {
        self.renderer
            .create_image_from_file_path(data_type, smoothing_mode, path)
    }

    /// Loads an image from the provided encoded image file data.
    ///
    /// If no `data_type` is provided, an attempt will be made to guess the file
    /// format.
    ///
    /// The data source must implement `std::io::BufRead` and `std::io::Seek`.
    /// For example, if you have a `&[u8]`, you may wrap it in a
    /// `std::io::Cursor` as follows:
    ///
    /// ```rust,no_run
    /// # use speedy2d::GLRenderer;
    /// # use speedy2d::color::Color;
    /// # use speedy2d::image::ImageSmoothingMode;
    /// use std::io::Cursor;
    /// # let mut renderer = unsafe {
    /// #     GLRenderer::new_for_gl_context((640, 480), |fn_name| {
    /// #         std::ptr::null() as *const _
    /// #     })
    /// # }.unwrap();
    ///
    /// let image_bytes : &[u8] = include_bytes!("../assets/screenshots/hello_world.png");
    ///
    /// let image_result = renderer.create_image_from_file_bytes(
    ///     None,
    ///     ImageSmoothingMode::Linear,
    ///     Cursor::new(image_bytes));
    /// ```
    ///
    /// For a list of supported image types, see [image::ImageFileFormat].
    ///
    /// The returned [ImageHandle] is valid only for the current graphics
    /// context.
    #[cfg(any(feature = "image-loading", doc, doctest))]
    pub fn create_image_from_file_bytes<R: Seek + BufRead>(
        &mut self,
        data_type: Option<ImageFileFormat>,
        smoothing_mode: ImageSmoothingMode,
        file_bytes: R
    ) -> Result<ImageHandle, BacktraceError<ErrorMessage>>
    {
        self.renderer
            .create_image_from_file_bytes(data_type, smoothing_mode, file_bytes)
    }

    /// Starts the process of drawing a frame. A `Graphics2D` object will be
    /// provided to the callback. When the callback returns, the internal
    /// render queue will be flushed.
    ///
    /// Note: if calling this method, you are responsible for swapping the
    /// window context buffers if necessary.
    #[inline]
    pub fn draw_frame<F: FnOnce(&mut Graphics2D) -> R, R>(&mut self, callback: F) -> R
    {
        self.renderer.set_clip(None);
        let result = callback(&mut self.renderer);
        self.renderer.renderer.finish_frame();
        result
    }
}

impl Drop for GLRenderer
{
    fn drop(&mut self)
    {
        self.context.mark_invalid();
    }
}

/// A `Graphics2D` object allows you to draw shapes, images, and text to the
/// screen.
///
/// An instance is provided in the [window::WindowHandler::on_draw] callback.
///
/// If you are managing the GL context yourself, you must invoke
/// [GLRenderer::draw_frame] to obtain an instance.
pub struct Graphics2D
{
    renderer: Renderer2D
}

impl Graphics2D
{
    /// Creates a new [ImageHandle] from the specified raw pixel data.
    ///
    /// The data provided in the `data` parameter must be in the format
    /// specified by `data_type`.
    ///
    /// The returned [ImageHandle] is valid only for the current graphics
    /// context.
    pub fn create_image_from_raw_pixels<S: Into<UVec2>>(
        &mut self,
        data_type: ImageDataType,
        smoothing_mode: ImageSmoothingMode,
        size: S,
        data: &[u8]
    ) -> Result<ImageHandle, BacktraceError<ErrorMessage>>
    {
        self.renderer.create_image_from_raw_pixels(
            data_type,
            smoothing_mode,
            size.into(),
            data
        )
    }

    /// Loads an image from the specified file path.
    ///
    /// If no `data_type` is provided, an attempt will be made to guess the file
    /// format.
    ///
    /// For a list of supported image types, see [image::ImageFileFormat].
    ///
    /// The returned [ImageHandle] is valid only for the current graphics
    /// context.
    #[cfg(any(feature = "image-loading", doc, doctest))]
    pub fn create_image_from_file_path<S: AsRef<Path>>(
        &mut self,
        data_type: Option<ImageFileFormat>,
        smoothing_mode: ImageSmoothingMode,
        path: S
    ) -> Result<ImageHandle, BacktraceError<ErrorMessage>>
    {
        self.renderer
            .create_image_from_file_path(data_type, smoothing_mode, path)
    }

    /// Loads an image from the provided encoded image file data.
    ///
    /// If no `data_type` is provided, an attempt will be made to guess the file
    /// format.
    ///
    /// The data source must implement `std::io::BufRead` and `std::io::Seek`.
    /// For example, if you have a `&[u8]`, you may wrap it in a
    /// `std::io::Cursor` as follows:
    ///
    /// ```rust,no_run
    /// # use speedy2d::GLRenderer;
    /// # use speedy2d::color::Color;
    /// # use speedy2d::image::ImageSmoothingMode;
    /// use std::io::Cursor;
    /// # let mut renderer = unsafe {
    /// #     GLRenderer::new_for_gl_context((640, 480), |fn_name| {
    /// #         std::ptr::null() as *const _
    /// #     })
    /// # }.unwrap();
    /// # renderer.draw_frame(|graphics| {
    ///
    /// let image_bytes : &[u8] = include_bytes!("../assets/screenshots/hello_world.png");
    ///
    /// let image_result = graphics.create_image_from_file_bytes(
    ///     None,
    ///     ImageSmoothingMode::Linear,
    ///     Cursor::new(image_bytes));
    /// # });
    /// ```
    ///
    /// For a list of supported image types, see [image::ImageFileFormat].
    ///
    /// The returned [ImageHandle] is valid only for the current graphics
    /// context.
    #[cfg(any(feature = "image-loading", doc, doctest))]
    pub fn create_image_from_file_bytes<R: Seek + BufRead>(
        &mut self,
        data_type: Option<ImageFileFormat>,
        smoothing_mode: ImageSmoothingMode,
        file_bytes: R
    ) -> Result<ImageHandle, BacktraceError<ErrorMessage>>
    {
        self.renderer
            .create_image_from_file_bytes(data_type, smoothing_mode, file_bytes)
    }

    /// Fills the screen with the specified color.
    pub fn clear_screen(&mut self, color: Color)
    {
        self.renderer.clear_screen(color);
    }

    /// Draws the provided block of text at the specified position.
    ///
    /// Lines of text can be prepared by loading a font (using
    /// [crate::font::Font::new]), and calling `layout_text_line()` on that
    /// font with your desired text.
    ///
    /// To fall back to another font if a glyph isn't found, see
    /// [crate::font::FontFamily].
    ///
    /// To achieve good performance, it's possible to layout a line of text
    /// once, and then re-use the same [crate::font::FormattedTextLine]
    /// object whenever you need to draw that text to the screen.
    ///
    /// Note: Text will be rendered with subpixel precision. If the subpixel
    /// position changes between frames, performance may be degraded, as the
    /// text will need to be re-rendered and re-uploaded. To avoid this,
    /// call `round()` on the position coordinates, to ensure that
    /// the text is always located at an integer pixel position.
    pub fn draw_text<V: Into<Vec2>>(
        &mut self,
        position: V,
        color: Color,
        text: &FormattedTextBlock
    )
    {
        self.renderer.draw_text(position, color, text);
    }

    /// Draws the provided block of text at the specified position, cropped to
    /// the specified window. Characters outside this window will not be
    /// rendered. Characters partially inside the window will be cropped.
    ///
    /// Both `position` and `crop_window` are relative to the overall render
    /// window.
    ///
    /// See the documentation for [Graphics2D::draw_text] for more details.
    pub fn draw_text_cropped<V: Into<Vec2>>(
        &mut self,
        position: V,
        crop_window: Rect,
        color: Color,
        text: &FormattedTextBlock
    )
    {
        self.renderer
            .draw_text_cropped(position, crop_window, color, text);
    }

    /// Draws a polygon with a single color, with the specified offset in
    /// pixels.
    pub fn draw_polygon<V: Into<Vec2>>(
        &mut self,
        polygon: &Polygon,
        offset: V,
        color: Color
    )
    {
        self.renderer.draw_polygon(polygon, offset, color)
    }

    /// Draws a triangle with the specified colors (one color for each corner).
    ///
    /// The vertex positions (and associated colors) must be provided in
    /// clockwise order.
    pub fn draw_triangle_three_color(
        &mut self,
        vertex_positions_clockwise: [Vec2; 3],
        vertex_colors_clockwise: [Color; 3]
    )
    {
        self.renderer.draw_triangle_three_color(
            vertex_positions_clockwise,
            vertex_colors_clockwise
        );
    }

    /// Draws part of an image, tinted with the provided colors, at the
    /// specified location. The sub-image will be scaled to fill the
    /// triangle described by the vertices in `vertex_positions_clockwise`.
    ///
    /// The coordinates in `image_coords_normalized` should be in the range
    /// `0.0` to `1.0`, and define the portion of the source image which
    /// should be drawn.
    ///
    /// The tinting is performed by for each pixel by multiplying each color
    /// component in the image pixel by the corresponding color component in
    /// the `color` parameter.
    ///
    /// The vertex positions (and associated colors and image coordinates) must
    /// be provided in clockwise order.
    pub fn draw_triangle_image_tinted_three_color(
        &mut self,
        vertex_positions_clockwise: [Vec2; 3],
        vertex_colors: [Color; 3],
        image_coords_normalized: [Vec2; 3],
        image: &ImageHandle
    )
    {
        self.renderer.draw_triangle_image_tinted(
            vertex_positions_clockwise,
            vertex_colors,
            image_coords_normalized,
            image
        );
    }

    /// Draws a triangle with the specified color.
    ///
    /// The vertex positions must be provided in clockwise order.
    #[inline]
    pub fn draw_triangle(&mut self, vertex_positions_clockwise: [Vec2; 3], color: Color)
    {
        self.draw_triangle_three_color(vertex_positions_clockwise, [color, color, color]);
    }

    /// Draws a quadrilateral with the specified colors (one color for each
    /// corner).
    ///
    /// The vertex positions (and associated colors) must be provided in
    /// clockwise order.
    #[inline]
    pub fn draw_quad_four_color(
        &mut self,
        vertex_positions_clockwise: [Vec2; 4],
        vertex_colors: [Color; 4]
    )
    {
        let vp = vertex_positions_clockwise;
        let vc = vertex_colors;

        self.draw_triangle_three_color([vp[0], vp[1], vp[2]], [vc[0], vc[1], vc[2]]);

        self.draw_triangle_three_color([vp[2], vp[3], vp[0]], [vc[2], vc[3], vc[0]]);
    }

    /// Draws a quadrilateral with the specified color.
    ///
    /// The vertex positions must be provided in clockwise order.
    #[inline]
    pub fn draw_quad(&mut self, vertex_positions_clockwise: [Vec2; 4], color: Color)
    {
        self.draw_quad_four_color(
            vertex_positions_clockwise,
            [color, color, color, color]
        );
    }

    /// Draws part of an image, tinted with the provided colors, at the
    /// specified location. The sub-image will be scaled to fill the
    /// quadrilateral described by the vertices in
    /// `vertex_positions_clockwise`.
    ///
    /// The coordinates in `image_coords_normalized` should be in the range
    /// `0.0` to `1.0`, and define the portion of the source image which
    /// should be drawn.
    ///
    /// The tinting is performed by for each pixel by multiplying each color
    /// component in the image pixel by the corresponding color component in
    /// the `color` parameter.
    ///
    /// The vertex positions (and associated colors and image coordinates) must
    /// be provided in clockwise order.
    #[inline]
    pub fn draw_quad_image_tinted_four_color(
        &mut self,
        vertex_positions_clockwise: [Vec2; 4],
        vertex_colors: [Color; 4],
        image_coords_normalized: [Vec2; 4],
        image: &ImageHandle
    )
    {
        let vp = vertex_positions_clockwise;
        let vc = vertex_colors;
        let ic = image_coords_normalized;

        self.draw_triangle_image_tinted_three_color(
            [vp[0], vp[1], vp[2]],
            [vc[0], vc[1], vc[2]],
            [ic[0], ic[1], ic[2]],
            image
        );

        self.draw_triangle_image_tinted_three_color(
            [vp[2], vp[3], vp[0]],
            [vc[2], vc[3], vc[0]],
            [ic[2], ic[3], ic[0]],
            image
        );
    }

    /// Draws part of an image, tinted with the provided color, at the specified
    /// location. The sub-image will be scaled to fill the pixel coordinates
    /// in the provided rectangle.
    ///
    /// The coordinates in `image_coords_normalized` should be in the range
    /// `0.0` to `1.0`, and define the portion of the source image which
    /// should be drawn.
    ///
    /// The tinting is performed by for each pixel by multiplying each color
    /// component in the image pixel by the corresponding color component in
    /// the `color` parameter.
    #[inline]
    pub fn draw_rectangle_image_subset_tinted(
        &mut self,
        rect: impl AsRef<Rectangle>,
        color: Color,
        image_coords_normalized: impl AsRef<Rectangle>,
        image: &ImageHandle
    )
    {
        let rect = rect.as_ref();
        let image_coords_normalized = image_coords_normalized.as_ref();

        self.draw_quad_image_tinted_four_color(
            [
                *rect.top_left(),
                rect.top_right(),
                *rect.bottom_right(),
                rect.bottom_left()
            ],
            [color, color, color, color],
            [
                *image_coords_normalized.top_left(),
                image_coords_normalized.top_right(),
                *image_coords_normalized.bottom_right(),
                image_coords_normalized.bottom_left()
            ],
            image
        );
    }

    /// Draws an image, tinted with the provided color, at the specified
    /// location. The image will be scaled to fill the pixel coordinates in
    /// the provided rectangle.
    ///
    /// The tinting is performed by for each pixel by multiplying each color
    /// component in the image pixel by the corresponding color component in
    /// the `color` parameter.
    #[inline]
    pub fn draw_rectangle_image_tinted(
        &mut self,
        rect: impl AsRef<Rectangle>,
        color: Color,
        image: &ImageHandle
    )
    {
        self.draw_rectangle_image_subset_tinted(
            rect,
            color,
            Rectangle::new(Vec2::ZERO, Vec2::new(1.0, 1.0)),
            image
        );
    }

    /// Draws an image at the specified location. The image will be
    /// scaled to fill the pixel coordinates in the provided rectangle.
    #[inline]
    pub fn draw_rectangle_image(
        &mut self,
        rect: impl AsRef<Rectangle>,
        image: &ImageHandle
    )
    {
        self.draw_rectangle_image_tinted(rect, Color::WHITE, image);
    }

    /// Draws an image at the specified pixel location. The image will be
    /// drawn at its original size with no scaling.
    #[inline]
    pub fn draw_image<P: Into<Vec2>>(&mut self, position: P, image: &ImageHandle)
    {
        let position = position.into();

        self.draw_rectangle_image(
            Rectangle::new(position, position + image.size().into_f32()),
            image
        );
    }

    /// Draws a single-color rectangle at the specified location. The
    /// coordinates of the rectangle are specified in pixels.
    #[inline]
    pub fn draw_rectangle(&mut self, rect: impl AsRef<Rectangle>, color: Color)
    {
        let rect = rect.as_ref();

        self.draw_quad(
            [
                *rect.top_left(),
                rect.top_right(),
                *rect.bottom_right(),
                rect.bottom_left()
            ],
            color
        );
    }

    /// Draws a single-color line between the given points, specified in pixels.
    ///
    /// # Pixel alignment
    ///
    /// On a display with square pixels, an integer-valued coordinate is located
    /// at the boundary between two pixels, rather than the center of the
    /// pixel. For example:
    ///
    ///  * `(0.0, 0.0)` = Top left of pixel
    ///  * `(0.5, 0.5)` = Center of pixel
    ///  * `(1.0, 1.0)` = Bottom right of pixel
    ///
    /// If drawing a line of odd-numbered thickness, it is advisable to locate
    /// the start and end of the line at the centers of pixels, rather than
    /// the edges.
    ///
    /// For example, a one-pixel-thick line between `(0.0, 10.0)` and `(100.0,
    /// 10.0)` will be drawn as a rectangle with corners `(0.0, 9.5)` and
    /// `(100.0, 10.5)`, meaning that the line's thickness will actually
    /// span two half-pixels. Drawing the same line between `(0.0, 10.5)`
    /// and `(100.0, 10.5)` will result in a pixel-aligned rectangle between
    /// `(0.0, 10.0)` and `(100.0, 11.0)`.
    pub fn draw_line<VStart: Into<Vec2>, VEnd: Into<Vec2>>(
        &mut self,
        start_position: VStart,
        end_position: VEnd,
        thickness: f32,
        color: Color
    )
    {
        let start_position = start_position.into();
        let end_position = end_position.into();

        let gradient_normalized = match (end_position - start_position).normalize() {
            None => return,
            Some(gradient) => gradient
        };

        let gradient_thickness = gradient_normalized * (thickness / 2.0);

        let offset_anticlockwise = gradient_thickness.rotate_90_degrees_anticlockwise();
        let offset_clockwise = gradient_thickness.rotate_90_degrees_clockwise();

        let start_anticlockwise = start_position + offset_anticlockwise;
        let start_clockwise = start_position + offset_clockwise;

        let end_anticlockwise = end_position + offset_anticlockwise;
        let end_clockwise = end_position + offset_clockwise;

        self.draw_quad(
            [
                start_anticlockwise,
                end_anticlockwise,
                end_clockwise,
                start_clockwise
            ],
            color
        );
    }

    /// Draws a circle, filled with a single color, at the specified pixel
    /// location.
    pub fn draw_circle<V: Into<Vec2>>(
        &mut self,
        center_position: V,
        radius: f32,
        color: Color
    )
    {
        let center_position = center_position.into();

        let top_left = center_position + Vec2::new(-radius, -radius);
        let top_right = center_position + Vec2::new(radius, -radius);
        let bottom_right = center_position + Vec2::new(radius, radius);
        let bottom_left = center_position + Vec2::new(-radius, radius);

        self.renderer.draw_circle_section(
            [top_left, top_right, bottom_right],
            [color, color, color],
            [
                Vec2::new(-1.0, -1.0),
                Vec2::new(1.0, -1.0),
                Vec2::new(1.0, 1.0)
            ]
        );

        self.renderer.draw_circle_section(
            [bottom_right, bottom_left, top_left],
            [color, color, color],
            [
                Vec2::new(1.0, 1.0),
                Vec2::new(-1.0, 1.0),
                Vec2::new(-1.0, -1.0)
            ]
        );
    }

    /// Draws a triangular subset of a circle.
    ///
    /// Put simply, this function will draw a triangle on the screen, textured
    /// with a region of a circle.
    ///
    /// The circle region is specified using `vertex_circle_coords_normalized`,
    /// which denotes UV coordinates relative to an infinitely-detailed
    /// circle of radius `1.0`, and center `(0.0, 0.0)`.
    ///
    /// For example, to draw the top-right half of a circle with radius 100px:
    ///
    /// ```rust,no_run
    /// # use speedy2d::GLRenderer;
    /// # use speedy2d::dimen::Vec2;
    /// # use speedy2d::color::Color;
    /// # let mut renderer = unsafe {
    /// #     GLRenderer::new_for_gl_context((640, 480), |fn_name| {
    /// #         std::ptr::null() as *const _
    /// #     })
    /// # }.unwrap();
    /// # renderer.draw_frame(|graphics| {
    /// graphics.draw_circle_section_triangular_three_color(
    ///         [
    ///                 Vec2::new(200.0, 200.0),
    ///                 Vec2::new(300.0, 200.0),
    ///                 Vec2::new(300.0, 300.0)],
    ///         [Color::MAGENTA; 3],
    ///         [
    ///                 Vec2::new(-1.0, -1.0),
    ///                 Vec2::new(1.0, -1.0),
    ///                 Vec2::new(1.0, 1.0)]);
    /// # });
    /// ```
    #[inline]
    pub fn draw_circle_section_triangular_three_color(
        &mut self,
        vertex_positions_clockwise: [Vec2; 3],
        vertex_colors: [Color; 3],
        vertex_circle_coords_normalized: [Vec2; 3]
    )
    {
        self.renderer.draw_circle_section(
            vertex_positions_clockwise,
            vertex_colors,
            vertex_circle_coords_normalized
        );
    }

    /// Sets the current clip to the rectangle specified by the given
    /// coordinates. Rendering operations have no effect outside of the
    /// clipping area.
    pub fn set_clip(&mut self, rect: Option<Rectangle<i32>>)
    {
        self.renderer.set_clip(rect);
    }

    /// Captures a screenshot of the render window. The returned data contains
    /// the color of each pixel. Pixels are represented using a `u8` for each
    /// component (red, green, blue, and alpha). Use the `format` parameter to
    /// specify the byte layout (and size) of each pixel.
    pub fn capture(&mut self, format: ImageDataType) -> RawBitmapData
    {
        self.renderer.capture(format)
    }
}

/// Struct representing a window.
#[cfg(any(doc, doctest, all(feature = "windowing", not(target_arch = "wasm32"))))]
pub struct Window<UserEventType = ()>
where
    UserEventType: 'static
{
    window_impl: WindowGlutin<UserEventType>,
    renderer: GLRenderer
}

#[cfg(any(doc, doctest, all(feature = "windowing", not(target_arch = "wasm32"))))]
impl Window<()>
{
    /// Create a new window, centered in the middle of the primary monitor.
    pub fn new_centered<Str, Size>(
        title: Str,
        size: Size
    ) -> Result<Window<()>, BacktraceError<WindowCreationError>>
    where
        Str: AsRef<str>,
        Size: Into<UVec2>
    {
        let size = size.into();

        Self::new_with_options(
            title.as_ref(),
            WindowCreationOptions::new_windowed(
                WindowSize::PhysicalPixels(size),
                Some(WindowPosition::Center)
            )
        )
    }

    /// Create a new window, in fullscreen borderless mode on the primary
    /// monitor.
    pub fn new_fullscreen_borderless<Str>(
        title: Str
    ) -> Result<Window<()>, BacktraceError<WindowCreationError>>
    where
        Str: AsRef<str>
    {
        Self::new_with_options(
            title.as_ref(),
            WindowCreationOptions::new_fullscreen_borderless()
        )
    }

    /// Create a new window with the specified options.
    pub fn new_with_options(
        title: &str,
        options: WindowCreationOptions
    ) -> Result<Window<()>, BacktraceError<WindowCreationError>>
    {
        Self::new_with_user_events(title, options)
    }
}

#[cfg(any(doc, doctest, all(feature = "windowing", not(target_arch = "wasm32"))))]
impl<UserEventType: 'static> Window<UserEventType>
{
    /// Create a new window with the specified options, with support for user
    /// events. See [window::UserEventSender].
    pub fn new_with_user_events(
        title: &str,
        options: WindowCreationOptions
    ) -> Result<Self, BacktraceError<WindowCreationError>>
    {
        let window_impl = WindowGlutin::new(title, options)?;

        let renderer = GLRenderer::new_with_gl_backend(
            window_impl.get_inner_size_pixels(),
            window_impl.gl_backend().clone(),
            GLVersion::OpenGL2_0
        )
        .map_err(|err| {
            BacktraceError::new_with_cause(
                WindowCreationError::RendererCreationFailed,
                err
            )
        })?;

        Ok(Window {
            window_impl,
            renderer
        })
    }

    /// Creates a [window::UserEventSender], which can be used to post custom
    /// events to this event loop from another thread.
    ///
    /// If calling this, specify the type of the event data using
    /// `Window::<YourTypeHere>::new_with_user_events()`.
    ///
    /// See [UserEventSender::send_event], [WindowHandler::on_user_event].
    pub fn create_user_event_sender(&self) -> UserEventSender<UserEventType>
    {
        self.window_impl.create_user_event_sender()
    }

    /// Run the window event loop, with the specified callback handler.
    ///
    /// Once the event loop finishes running, the entire app will terminate,
    /// even if other threads are still running. See
    /// [window::WindowHelper::terminate_loop()].
    pub fn run_loop<H>(self, handler: H) -> !
    where
        H: WindowHandler<UserEventType> + 'static
    {
        self.window_impl.run_loop(handler, self.renderer);
    }
}

/// Struct representing an HTML canvas.
#[cfg(any(doc, doctest, all(target_arch = "wasm32", feature = "windowing")))]
pub struct WebCanvas<UserEventType = ()>
where
    UserEventType: 'static
{
    inner: Option<WebCanvasImpl>,
    should_cleanup: bool,
    user_event_type: PhantomData<UserEventType>
}

#[cfg(any(doc, doctest, all(target_arch = "wasm32", feature = "windowing")))]
impl WebCanvas<()>
{
    /// Creates (and starts running) a new WebCanvas instance, attached to the
    /// HTML canvas with the specified ID. Event handlers will be registered for
    /// keyboard, mouse, and other events.
    ///
    /// The event loop/handlers will continue to exist after the WebCanvas is
    /// dropped. This behaviour can be avoided using
    /// [WebCanvas::unregister_when_dropped].
    ///
    /// The provided [WindowHandler] will start to receive callbacks as soon as
    /// this function returns. Note that the main thread must not be blocked.
    pub fn new_for_id<S, H>(
        element_id: S,
        handler: H
    ) -> Result<WebCanvas<()>, BacktraceError<ErrorMessage>>
    where
        S: AsRef<str>,
        H: WindowHandler<()> + 'static
    {
        WebCanvas::<()>::new_for_id_with_user_events(element_id, handler)
    }
}

#[cfg(any(doc, doctest, all(target_arch = "wasm32", feature = "windowing")))]
impl<UserEventType: 'static> WebCanvas<UserEventType>
{
    /// Creates (and starts running) a new WebCanvas instance, attached to the
    /// HTML canvas with the specified ID. Event handlers will be registered for
    /// keyboard, mouse, and other events.
    ///
    /// This variant has support for user-generated events. See
    /// [window::UserEventSender] for more details.
    ///
    /// The event loop/handlers will continue to exist after the WebCanvas is
    /// dropped. This behaviour can be avoided using
    /// [WebCanvas::unregister_when_dropped].
    ///
    /// The provided [WindowHandler] will start to receive callbacks as soon as
    /// this function returns. Note that the main thread must not be blocked.
    pub fn new_for_id_with_user_events<S, H>(
        element_id: S,
        handler: H
    ) -> Result<Self, BacktraceError<ErrorMessage>>
    where
        S: AsRef<str>,
        H: WindowHandler<UserEventType> + 'static
    {
        Ok(WebCanvas {
            inner: Some(WebCanvasImpl::new(element_id, handler)?),
            should_cleanup: false,
            user_event_type: PhantomData
        })
    }

    /// Causes the WebCanvas event loop to terminate when the WebCanvas is
    /// dropped. If this function is not called, then the event loop (and
    /// associated event handlers) will continue to run after the WebCanvas
    /// struct is dropped.
    pub fn unregister_when_dropped(&mut self)
    {
        self.should_cleanup = true;
    }
}

#[cfg(any(doc, doctest, all(target_arch = "wasm32", feature = "windowing")))]
impl<UserEventType: 'static> Drop for WebCanvas<UserEventType>
{
    fn drop(&mut self)
    {
        if !self.should_cleanup {
            std::mem::forget(self.inner.take());
            log::info!(
                "Deliberately leaking speedy2d::WebCanvas object. This is normally \
                 fine, but if you want to clean up before the page closes, call \
                 WebCanvas::unregister_when_dropped(), and retain ownership of the \
                 WebCanvas until you want to delete it."
            )
        }
    }
}
