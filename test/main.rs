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

#[cfg(not(all(target_arch = "x86_64", target_os = "linux")))]
compile_error!("The automated tests currently support Linux x86_64 only");

use std::convert::TryInto;
use std::os::raw::c_void;

use glutin::dpi::PhysicalSize;
use glutin::event_loop::EventLoop;
use image::{ColorType, GenericImageView, ImageFormat};
use speedy2d::color::Color;
use speedy2d::dimen::Vector2;
use speedy2d::font::{Font, TextAlignment, TextLayout, TextOptions};
use speedy2d::image::{ImageDataType, ImageSmoothingMode};
use speedy2d::shape::Rectangle;
use speedy2d::GLRenderer;

const NOTO_SANS_REGULAR_BYTES: &[u8] =
    include_bytes!("../assets/fonts/NotoSans-Regular.ttf");

fn get_expected_image_path<S: AsRef<str>>(name: S) -> String
{
    format!("test/assets/expected_images/test_{}.png", name.as_ref())
}

fn write_rgba_to_png<S: AsRef<str>>(name: S, width: u32, height: u32, buf: &[u8])
{
    image::save_buffer_with_format(
        get_expected_image_path(name),
        buf,
        width,
        height,
        ColorType::Rgba8,
        ImageFormat::Png
    )
    .unwrap();
}

fn read_png_argb8<S: AsRef<str>>(name: S) -> Option<Vec<u8>>
{
    image::io::Reader::open(get_expected_image_path(name))
        .ok()
        .and_then(|reader| reader.decode().ok())
        .map(|image| image.into_rgba8().into_raw())
}

fn read_framebuffer_argb(width: u32, height: u32) -> Vec<u8>
{
    let mut buf: Vec<u8> = vec![0; (width * height * 4).try_into().unwrap()];

    unsafe {
        gl::ReadPixels(
            0,
            0,
            width.try_into().unwrap(),
            height.try_into().unwrap(),
            gl::RGBA,
            gl::UNSIGNED_BYTE,
            buf.as_mut_ptr() as *mut c_void
        );
    }

    let mut flipped_buf: Vec<u8> = Vec::new();

    for y in 0..height {
        let in_start = ((height - y - 1) * width * 4).try_into().unwrap();
        let in_slice = &buf[in_start..(in_start + (width * 4) as usize)];

        flipped_buf.extend_from_slice(in_slice);
    }

    flipped_buf
}

fn write_framebuffer_to_png<S: AsRef<str>>(name: S, width: u32, height: u32)
{
    write_rgba_to_png(
        name,
        width,
        height,
        read_framebuffer_argb(width, height).as_slice()
    );
}

fn create_context_and_run<R, F>(
    event_loop: &EventLoop<()>,
    width: u32,
    height: u32,
    action: F
) -> R
where
    F: FnOnce(&mut GLRenderer) -> R
{
    let context_builder = glutin::ContextBuilder::new()
        .with_gl_debug_flag(true)
        .with_multisampling(0)
        .with_gl(glutin::GlRequest::Specific(glutin::Api::OpenGl, (2, 0)));

    #[cfg(not(target_os = "linux"))]
    let context = context_builder
        .build_windowed(
            glutin::window::WindowBuilder::new()
                .with_inner_size(PhysicalSize::new(width, height)),
            &event_loop
        )
        .unwrap();

    #[cfg(target_os = "linux")]
    let context = context_builder
        .with_vsync(false)
        .build_headless(&event_loop, PhysicalSize::new(width, height))
        .unwrap();

    let context = unsafe { context.make_current().unwrap() };

    // Used for glReadPixels/etc
    gl::load_with(|ptr| context.get_proc_address(ptr) as *const _);

    let mut renderer = unsafe {
        GLRenderer::new_for_gl_context((width, height), |name| {
            context.get_proc_address(name) as *const _
        })
        .unwrap()
    };

    action(&mut renderer)
}

fn run_test_with_new_context<S: AsRef<str>, F: FnOnce(&mut GLRenderer)>(
    event_loop: &EventLoop<()>,
    expected_image_name: S,
    width: u32,
    height: u32,
    action: F
)
{
    let expected_image = read_png_argb8(expected_image_name.as_ref());

    let actual_image = create_context_and_run(event_loop, width, height, |renderer| {
        action(renderer);

        let actual_image = read_framebuffer_argb(width, height);

        if expected_image.is_none()
            || (&expected_image).as_ref().unwrap() != &actual_image
        {
            write_framebuffer_to_png(
                format!("{}_ACTUAL", expected_image_name.as_ref()),
                width,
                height
            );
        }

        actual_image
    });

    assert!(expected_image.is_some(), "Expected image does not exist");

    let expected_image = expected_image.unwrap();

    assert_eq!(
        width * height * 4,
        expected_image.len().try_into().unwrap(),
        "Expected image size mismatch"
    );

    assert_eq!(
        width * height * 4,
        actual_image.len().try_into().unwrap(),
        "Actual image size mismatch"
    );

    assert_eq!(
        expected_image,
        actual_image,
        "Generated image did not match expected ({})",
        expected_image_name.as_ref()
    );
}

struct GLTest
{
    width: u32,
    height: u32,
    name: String,
    action: Box<dyn FnOnce(&mut GLRenderer)>
}

fn main()
{
    simple_logger::SimpleLogger::new().init().unwrap();

    let event_loop = EventLoop::new();

    let mut tests = Vec::new();

    tests.push(GLTest {
        width: 50,
        height: 50,
        name: "basic_rectangles".to_string(),
        action: Box::new(|renderer| {
            renderer.draw_frame(|graphics| {
                graphics.clear_screen(Color::BLUE);

                graphics.draw_rectangle(
                    Rectangle::from_tuples((10.0, 20.0), (30.0, 40.0)),
                    Color::MAGENTA
                );

                graphics.draw_rectangle(
                    Rectangle::from_tuples((15.0, 30.0), (49.0, 48.0)),
                    Color::GREEN
                );
            });
        })
    });

    tests.push(GLTest {
        width: 50,
        height: 50,
        name: "lines_horizontal".to_string(),
        action: Box::new(|renderer| {
            renderer.draw_frame(|graphics| {
                graphics.clear_screen(Color::WHITE);

                graphics.draw_line((10.0, 10.5), (30.0, 10.5), 1.0, Color::BLUE);

                graphics.draw_line((20.0, 14.0), (40.0, 14.0), 2.0, Color::DARK_GRAY);

                graphics.draw_line((1.0, 20.5), (49.0, 20.5), 5.0, Color::LIGHT_GRAY);
            });
        })
    });

    tests.push(GLTest {
        width: 50,
        height: 50,
        name: "lines_vertical".to_string(),
        action: Box::new(|renderer| {
            renderer.draw_frame(|graphics| {
                graphics.clear_screen(Color::WHITE);

                graphics.draw_line((10.5, 10.0), (10.5, 30.0), 1.0, Color::BLUE);

                graphics.draw_line((14.0, 20.0), (14.0, 40.0), 2.0, Color::DARK_GRAY);

                graphics.draw_line((20.5, 1.0), (20.5, 49.0), 5.0, Color::LIGHT_GRAY);
            });
        })
    });

    tests.push(GLTest {
        width: 50,
        height: 50,
        name: "basic_circles".to_string(),
        action: Box::new(|renderer| {
            renderer.draw_frame(|graphics| {
                graphics.clear_screen(Color::WHITE);

                graphics.draw_circle((20.0, 20.0), 10.0, Color::RED);

                graphics.draw_circle((40.0, 40.0), 5.0, Color::BLUE);
            });
        })
    });

    tests.push(GLTest {
        width: 300,
        height: 300,
        name: "half_circle".to_string(),
        action: Box::new(|renderer| {
            renderer.draw_frame(|graphics| {
                graphics.clear_screen(Color::WHITE);

                graphics.draw_circle_section_triangular_three_color(
                    [
                        Vector2::new(100.0, 100.0),
                        Vector2::new(200.0, 100.0),
                        Vector2::new(200.0, 200.0)
                    ],
                    [
                        Color::MAGENTA.clone(),
                        Color::MAGENTA.clone(),
                        Color::MAGENTA.clone()
                    ],
                    [
                        Vector2::new(-1.0, -1.0),
                        Vector2::new(1.0, -1.0),
                        Vector2::new(1.0, 1.0)
                    ]
                );
            });
        })
    });

    tests.push(GLTest {
        width: 1400,
        height: 500,
        name: "basic_text_white_background".to_string(),
        action: Box::new(|renderer| {
            let typeface = Font::new(NOTO_SANS_REGULAR_BYTES).unwrap();

            let text = typeface.layout_text(
                "The quick brown föx jumped over the lazy dog!",
                64.0,
                TextOptions::new()
            );

            renderer.draw_frame(|graphics| {
                graphics.clear_screen(Color::WHITE);

                graphics.draw_rectangle(
                    Rectangle::from_tuples(
                        (0.0, 0.0),
                        (text.width().round(), text.height().round())
                    ),
                    Color::from_rgb(0.9, 0.9, 1.0)
                );

                graphics.draw_rectangle(
                    Rectangle::from_tuples(
                        (0.0, 0.0),
                        (
                            text.width().round(),
                            text.iter_lines().next().unwrap().ascent().round()
                        )
                    ),
                    Color::from_rgb(0.8, 0.8, 1.0)
                );

                graphics.draw_text(Vector2::new(0.0, 0.0), Color::BLACK, &text);

                graphics.draw_text(Vector2::new(0.0, 100.0), Color::RED, &text);

                graphics.draw_text(Vector2::new(0.0, 200.0), Color::GREEN, &text);

                graphics.draw_text(Vector2::new(0.0, 300.0), Color::BLUE, &text);

                graphics.draw_text(Vector2::new(0.0, 400.0), Color::WHITE, &text);
            });
        })
    });

    tests.push(GLTest {
        width: 1400,
        height: 500,
        name: "basic_text_black_background".to_string(),
        action: Box::new(|renderer| {
            let typeface = Font::new(NOTO_SANS_REGULAR_BYTES).unwrap();

            let text = typeface.layout_text(
                "The quick brown föx jumped over the lazy dog!",
                64.0,
                TextOptions::new()
            );

            renderer.draw_frame(|graphics| {
                graphics.clear_screen(Color::BLACK);

                graphics.draw_text(Vector2::new(0.0, 0.0), Color::BLACK, &text);

                graphics.draw_text(Vector2::new(0.0, 100.0), Color::RED, &text);

                graphics.draw_text(Vector2::new(0.0, 200.0), Color::GREEN, &text);

                graphics.draw_text(Vector2::new(0.0, 300.0), Color::BLUE, &text);

                graphics.draw_text(Vector2::new(0.0, 400.0), Color::WHITE, &text);
            });
        })
    });

    tests.push(GLTest {
        width: 640,
        height: 640,
        name: "wrapped_text_1".to_string(),
        action: Box::new(|renderer| {
            let typeface = Font::new(NOTO_SANS_REGULAR_BYTES).unwrap();

            let first_text = typeface.layout_text(
                "The quick brown föx jumped over the lazy dog!",
                64.0,
                TextOptions::new().with_wrap_to_width(400.0, TextAlignment::Left)
            );

            renderer.draw_frame(|graphics| {
                graphics.clear_screen(Color::WHITE);

                graphics.draw_rectangle(
                    Rectangle::from_tuples(
                        (0.0, 0.0),
                        (first_text.width().round(), first_text.height().round())
                    ),
                    Color::from_rgb(0.9, 0.9, 1.0)
                );

                graphics.draw_rectangle(
                    Rectangle::from_tuples(
                        (0.0, 0.0),
                        (
                            first_text.width().round(),
                            first_text.iter_lines().next().unwrap().ascent().round()
                        )
                    ),
                    Color::from_rgb(0.8, 0.8, 1.0)
                );

                graphics.draw_text((0.0, 0.0), Color::BLACK, &first_text);

                let small_width = 90.0;

                graphics.draw_rectangle(
                    Rectangle::from_tuples((100.0, 200.0), (100.0 + small_width, 640.0)),
                    Color::from_rgb(0.9, 0.9, 1.0)
                );

                graphics.draw_text(
                    (100.0, 200.0),
                    Color::BLACK,
                    &typeface.layout_text(
                        "The quick brown föx jumped over the lazy dog!",
                        64.0,
                        TextOptions::new()
                            .with_wrap_to_width(small_width, TextAlignment::Left)
                    )
                );

                let small_width = 30.0;

                graphics.draw_rectangle(
                    Rectangle::from_tuples((200.0, 200.0), (200.0 + small_width, 640.0)),
                    Color::from_rgb(0.9, 0.9, 1.0)
                );

                graphics.draw_text(
                    (200.0, 200.0),
                    Color::BLACK,
                    &typeface.layout_text(
                        "The quick brown föx jumped over the lazy dog!",
                        64.0,
                        TextOptions::new()
                            .with_wrap_to_width(small_width, TextAlignment::Left)
                    )
                );
            });
        })
    });

    tests.push(GLTest {
        width: 640,
        height: 640,
        name: "text_tracking".to_string(),
        action: Box::new(|renderer| {
            let typeface = Font::new(NOTO_SANS_REGULAR_BYTES).unwrap();

            let text = typeface.layout_text(
                "The quick brown föx jumped over the lazy dog!",
                30.0,
                TextOptions::new()
                    .with_wrap_to_width(400.0, TextAlignment::Left)
                    .with_tracking(100.0)
            );

            renderer.draw_frame(|graphics| {
                graphics.clear_screen(Color::WHITE);

                graphics.draw_text((10.0, 10.0), Color::BLACK, &text);
            });
        })
    });

    tests.push(GLTest {
        width: 640,
        height: 640,
        name: "text_alignment".to_string(),
        action: Box::new(|renderer| {
            let typeface = Font::new(NOTO_SANS_REGULAR_BYTES).unwrap();

            renderer.draw_frame(|graphics| {
                graphics.clear_screen(Color::WHITE);

                graphics.draw_rectangle(
                    Rectangle::from_tuples((10.0, 10.0), (410.0, 640.0)),
                    Color::from_rgb(0.9, 0.9, 1.0)
                );

                graphics.draw_text(
                    (10.0, 10.0),
                    Color::BLACK,
                    &typeface.layout_text(
                        "The quick brown föx jumped over the lazy dog!",
                        40.0,
                        TextOptions::new()
                            .with_wrap_to_width(400.0, TextAlignment::Right)
                    )
                );

                graphics.draw_text(
                    (10.0, 210.0),
                    Color::BLACK,
                    &typeface.layout_text(
                        "The quick brown föx jumped over the lazy dog!",
                        40.0,
                        TextOptions::new()
                            .with_wrap_to_width(400.0, TextAlignment::Center)
                    )
                );
            });
        })
    });

    tests.push(GLTest {
        width: 640,
        height: 640,
        name: "text_line_spacing".to_string(),
        action: Box::new(|renderer| {
            let typeface = Font::new(NOTO_SANS_REGULAR_BYTES).unwrap();

            renderer.draw_frame(|graphics| {
                graphics.clear_screen(Color::WHITE);

                graphics.draw_rectangle(
                    Rectangle::from_tuples((10.0, 10.0), (410.0, 640.0)),
                    Color::from_rgb(0.9, 0.9, 1.0)
                );

                graphics.draw_text(
                    (10.0, 10.0),
                    Color::BLACK,
                    &typeface.layout_text(
                        "The quick brown föx jumped over the lazy dog!",
                        40.0,
                        TextOptions::new()
                            .with_wrap_to_width(400.0, TextAlignment::Left)
                            .with_line_spacing_multiplier(0.7)
                    )
                );

                graphics.draw_text(
                    (10.0, 210.0),
                    Color::BLACK,
                    &typeface.layout_text(
                        "The quick brown föx jumped over the lazy dog!",
                        40.0,
                        TextOptions::new()
                            .with_wrap_to_width(400.0, TextAlignment::Left)
                            .with_line_spacing_multiplier(2.0)
                    )
                );
            });
        })
    });

    tests.push(GLTest {
        width: 500,
        height: 500,
        name: "text_line_break_1".to_string(),
        action: Box::new(|renderer| {
            let typeface = Font::new(NOTO_SANS_REGULAR_BYTES).unwrap();

            let text = typeface.layout_text(
                "The quick brown föx\njumped ov\ner the lazy dog!",
                32.0,
                TextOptions::new()
            );

            renderer.draw_frame(|graphics| {
                graphics.clear_screen(Color::WHITE);
                graphics.draw_text(Vector2::new(0.0, 0.0), Color::BLACK, &text);
            });
        })
    });

    tests.push(GLTest {
        width: 500,
        height: 500,
        name: "text_line_break_2".to_string(),
        action: Box::new(|renderer| {
            let typeface = Font::new(NOTO_SANS_REGULAR_BYTES).unwrap();

            let text = typeface.layout_text(
                "\nThe quick brown föx\nj\n\numped ov\ner the lazy dog!",
                32.0,
                TextOptions::new()
            );

            renderer.draw_frame(|graphics| {
                graphics.clear_screen(Color::WHITE);
                graphics.draw_text(Vector2::new(0.0, 0.0), Color::BLACK, &text);
            });
        })
    });

    tests.push(GLTest {
        width: 640,
        height: 640,
        name: "wrapped_text_line_break".to_string(),
        action: Box::new(|renderer| {
            let typeface = Font::new(NOTO_SANS_REGULAR_BYTES).unwrap();

            let first_text = typeface.layout_text(
                "The quick brown föx jumped\n over the lazy dog!",
                64.0,
                TextOptions::new().with_wrap_to_width(400.0, TextAlignment::Left)
            );

            renderer.draw_frame(|graphics| {
                graphics.clear_screen(Color::WHITE);

                graphics.draw_rectangle(
                    Rectangle::from_tuples(
                        (0.0, 0.0),
                        (first_text.width().round(), first_text.height().round())
                    ),
                    Color::from_rgb(0.9, 0.9, 1.0)
                );

                graphics.draw_rectangle(
                    Rectangle::from_tuples(
                        (0.0, 0.0),
                        (
                            first_text.width().round(),
                            first_text.iter_lines().next().unwrap().ascent().round()
                        )
                    ),
                    Color::from_rgb(0.8, 0.8, 1.0)
                );

                graphics.draw_text((0.0, 0.0), Color::BLACK, &first_text);

                let small_width = 200.0;

                graphics.draw_rectangle(
                    Rectangle::from_tuples((100.0, 200.0), (100.0 + small_width, 640.0)),
                    Color::from_rgb(0.9, 0.9, 1.0)
                );

                graphics.draw_text(
                    (100.0, 200.0),
                    Color::BLACK,
                    &typeface.layout_text(
                        "The\n quick brown föx jumped over the lazy dog!",
                        32.0,
                        TextOptions::new()
                            .with_wrap_to_width(small_width, TextAlignment::Left)
                    )
                );
            });
        })
    });

    tests.push(GLTest {
        width: 3000,
        height: 2000,
        name: "huge_text".to_string(),
        action: Box::new(|renderer| {
            let typeface = Font::new(NOTO_SANS_REGULAR_BYTES).unwrap();

            let text = typeface.layout_text("Hello World", 1000.0, TextOptions::new());

            renderer.draw_frame(|graphics| {
                graphics.clear_screen(Color::WHITE);
                graphics.draw_text(Vector2::new(0.0, 0.0), Color::BLACK, &text);
            });
        })
    });

    tests.push(GLTest {
        width: 640,
        height: 640,
        name: "image_load_from_raw_pixels".to_string(),
        action: Box::new(|renderer| {
            let image =
                image::open("test/assets/expected_images/test_half_circle.png").unwrap();
            let size = image.dimensions();

            renderer.draw_frame(|graphics| {
                graphics.clear_screen(Color::WHITE);

                let texture = graphics
                    .create_image_from_raw_pixels(
                        ImageDataType::RGBA,
                        ImageSmoothingMode::Linear,
                        Vector2::new(size.0, size.1),
                        &image.to_rgba8()
                    )
                    .unwrap();

                graphics.draw_image(Vector2::new(200.0, 200.0), &texture);
            });
        })
    });

    tests.push(GLTest {
        width: 640,
        height: 640,
        name: "image_load_from_raw_pixels_multiple_times".to_string(),
        action: Box::new(|renderer| {
            let image =
                image::open("test/assets/expected_images/test_half_circle.png").unwrap();
            let size = image.dimensions();

            for _ in 0..10 {
                renderer.draw_frame(|graphics| {
                    graphics.clear_screen(Color::WHITE);

                    let texture = graphics
                        .create_image_from_raw_pixels(
                            ImageDataType::RGBA,
                            ImageSmoothingMode::Linear,
                            Vector2::new(size.0, size.1),
                            &image.to_rgba8()
                        )
                        .unwrap();

                    graphics.draw_image(Vector2::new(200.0, 200.0), &texture);
                });
            }
        })
    });

    tests.push(GLTest {
        width: 640,
        height: 640,
        name: "image_load_from_raw_pixels_no_alpha".to_string(),
        action: Box::new(|renderer| {
            let image =
                image::open("test/assets/expected_images/test_half_circle.png").unwrap();
            let size = image.dimensions();

            renderer.draw_frame(|graphics| {
                graphics.clear_screen(Color::WHITE);

                let texture = graphics
                    .create_image_from_raw_pixels(
                        ImageDataType::RGB,
                        ImageSmoothingMode::Linear,
                        Vector2::new(size.0, size.1),
                        &image.to_rgb8()
                    )
                    .unwrap();

                graphics.draw_image(Vector2::new(200.0, 200.0), &texture);
            });
        })
    });

    #[cfg(feature = "image-loading")]
    tests.push(GLTest {
        width: 640,
        height: 640,
        name: "image_load_from_file_path".to_string(),
        action: Box::new(|renderer| {
            let image = renderer
                .create_image_from_file_path(
                    None,
                    ImageSmoothingMode::Linear,
                    "test/assets/expected_images/test_half_circle.png"
                )
                .unwrap();

            renderer.draw_frame(|graphics| {
                graphics.clear_screen(Color::WHITE);
                graphics.draw_image(Vector2::new(200.0, 200.0), &image);
            });
        })
    });

    #[cfg(feature = "image-loading")]
    tests.push(GLTest {
        width: 640,
        height: 640,
        name: "image_load_from_file_bytes".to_string(),
        action: Box::new(|renderer| {
            let image = renderer
                .create_image_from_file_bytes(
                    None,
                    ImageSmoothingMode::Linear,
                    std::io::Cursor::new(include_bytes!(
                        "assets/expected_images/test_half_circle.png"
                    ))
                )
                .unwrap();

            renderer.draw_frame(|graphics| {
                graphics.clear_screen(Color::WHITE);
                graphics.draw_image(Vector2::new(200.0, 200.0), &image);
            });
        })
    });

    tests.push(GLTest {
        width: 640,
        height: 640,
        name: "image_load_from_raw_pixels_smiley".to_string(),
        action: Box::new(|renderer| {
            let image =
                image::open("test/assets/test_images/smiley_colormap.png").unwrap();
            let size = image.dimensions();

            renderer.draw_frame(|graphics| {
                graphics.clear_screen(Color::LIGHT_GRAY);

                let texture = graphics
                    .create_image_from_raw_pixels(
                        ImageDataType::RGB,
                        ImageSmoothingMode::NearestNeighbor,
                        Vector2::new(size.0, size.1),
                        &image.to_rgb8()
                    )
                    .unwrap();

                graphics.draw_image(Vector2::new(100.0, 100.0), &texture);
            });
        })
    });

    tests.push(GLTest {
        width: 100,
        height: 100,
        name: "clip_area".to_string(),
        action: Box::new(|renderer| {
            renderer.draw_frame(|graphics| {
                graphics.clear_screen(Color::LIGHT_GRAY);

                graphics.set_clip(Some(Rectangle::from_tuples((10, 10), (30, 20))));
                graphics.draw_rectangle(
                    Rectangle::from_tuples((0.0, 0.0), (20.0, 40.0)),
                    Color::RED
                );
                graphics.draw_rectangle(
                    Rectangle::from_tuples((20.0, 0.0), (40.0, 40.0)),
                    Color::BLUE
                );
            });
        })
    });

    tests.push(GLTest {
        width: 400,
        height: 150,
        name: "clip_area_2".to_string(),
        action: Box::new(|renderer| {
            let typeface = Font::new(NOTO_SANS_REGULAR_BYTES).unwrap();

            let text = typeface.layout_text("Hello World", 100.0, TextOptions::new());

            renderer.draw_frame(|graphics| {
                graphics.clear_screen(Color::WHITE);
                graphics.set_clip(Some(Rectangle::from_tuples((25, 25), (250, 75))));
                graphics.clear_screen(Color::GREEN);
                graphics.draw_text(Vector2::new(0.0, 0.0), Color::BLACK, &text);
            });
        })
    });

    for test in tests {
        log::info!("Running test {}", test.name);

        run_test_with_new_context(
            &event_loop,
            test.name,
            test.width,
            test.height,
            test.action
        );
    }

    log::info!("All tests succeeded");
}
