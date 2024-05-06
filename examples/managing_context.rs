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

// Minimal example on how to use Speedy2D while managing the GL context
// yourself. Uses glutin for cross-platform window creation and does not do any
// error handling.

use std::convert::TryInto;
use std::ffi::CString;

use glutin::config::ConfigTemplateBuilder;
use glutin::context::{
    ContextApi,
    ContextAttributesBuilder,
    NotCurrentGlContext,
    PossiblyCurrentContext,
    Version
};
use glutin::display::{GetGlDisplay, GlDisplay};
use glutin::prelude::GlSurface;
use glutin::surface::{Surface, SurfaceAttributesBuilder, WindowSurface};
use glutin_winit::{DisplayBuilder, GlWindow};
use raw_window_handle::HasRawWindowHandle;
use speedy2d::color::Color;
use speedy2d::font::{Font, FormattedTextBlock, TextLayout, TextOptions};
use speedy2d::{GLRenderer, Graphics2D};
use winit::dpi::PhysicalSize;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{Window, WindowBuilder};

fn main()
{
    let event_loop = EventLoop::new().unwrap();

    let builder = WindowBuilder::new()
        .with_title("Speedy2D: Hello World")
        .with_visible(true)
        .with_inner_size(PhysicalSize::new(640, 240));

    let (context, _window, surface) = create_best_context(&builder, &event_loop).unwrap();

    let mut renderer = unsafe {
        GLRenderer::new_for_gl_context((640, 240), |fn_name| {
            context
                .display()
                .get_proc_address(CString::new(fn_name).unwrap().as_c_str())
                as *const _
        })
        .unwrap()
    };

    let font = Font::new(include_bytes!("../assets/fonts/NotoSans-Regular.ttf")).unwrap();
    let text = font.layout_text("Hello world!", 64.0, TextOptions::new());

    event_loop
        .run(move |event, target| {
            target.set_control_flow(ControlFlow::Poll);

            match event {
                Event::WindowEvent { event, .. } => match event {
                    WindowEvent::CloseRequested => target.exit(),
                    _ => {}
                },

                Event::AboutToWait => {
                    renderer.draw_frame(|graphics| {
                        render_frame(graphics, &text);
                    });
                    surface.swap_buffers(&context).unwrap();
                }
                _ => {}
            }
        })
        .unwrap();
}

fn render_frame(graphics: &mut Graphics2D, text: &FormattedTextBlock)
{
    graphics.clear_screen(Color::WHITE);
    graphics.draw_circle((150.0, 120.0), 75.0, Color::from_rgb(0.8, 0.9, 1.0));
    graphics.draw_text((290.0, 90.0), Color::BLACK, text);
}

fn create_best_context<UserEventType>(
    window_builder: &WindowBuilder,
    event_loop: &EventLoop<UserEventType>
) -> Option<(PossiblyCurrentContext, Window, Surface<WindowSurface>)>
{
    for multisampling in &[16, 8, 4, 2, 1, 0] {
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
            .build(event_loop, template, |mut configs| configs.next().unwrap());

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

        return Some((context, window, surface));
    }

    log::error!("Failed to create any context.");
    None
}
