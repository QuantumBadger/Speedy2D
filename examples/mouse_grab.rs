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

#![deny(warnings)]

use std::rc::Rc;

use speedy2d::color::Color;
use speedy2d::dimen::Vector2;
use speedy2d::font::{Font, FormattedTextBlock, TextLayout, TextOptions};
use speedy2d::window::{MouseButton, WindowHandler, WindowHelper, WindowStartupInfo};
use speedy2d::{Graphics2D, Window};

fn main()
{
    simple_logger::SimpleLogger::new().init().unwrap();

    let window =
        Window::new_centered("Speedy2D: Mouse Grab Example", (640, 480)).unwrap();

    let font = Font::new(include_bytes!("../assets/fonts/NotoSans-Regular.ttf")).unwrap();

    let text = font.layout_text(
        "Left click to grab the cursor. Right click (or press a key) to release.",
        20.0,
        TextOptions::new()
    );

    window.run_loop(MyWindowHandler {
        offset: Vector2::ZERO,
        text,
        grabbed: false,
        window_size: Vector2::ZERO
    })
}

struct MyWindowHandler
{
    offset: Vector2<f32>,
    text: Rc<FormattedTextBlock>,
    grabbed: bool,
    window_size: Vector2<u32>
}

impl WindowHandler for MyWindowHandler
{
    fn on_start(&mut self, _helper: &mut WindowHelper, info: WindowStartupInfo)
    {
        log::info!("Got on_start callback: {:?}", info);
        self.window_size = *info.viewport_size_pixels();
    }

    fn on_resize(&mut self, _helper: &mut WindowHelper<()>, size_pixels: Vector2<u32>)
    {
        self.window_size = size_pixels;
    }

    fn on_mouse_grab_status_changed(
        &mut self,
        helper: &mut WindowHelper<()>,
        mouse_grabbed: bool
    )
    {
        log::info!("Mouse grab status changed: {}", mouse_grabbed);
        self.grabbed = mouse_grabbed;

        helper.set_cursor_visible(!mouse_grabbed);
    }

    fn on_draw(&mut self, _helper: &mut WindowHelper, graphics: &mut Graphics2D)
    {
        // Clear the screen
        graphics.clear_screen(Color::from_rgb(0.8, 0.9, 1.0));

        // Red for down, blue for up
        let color = if self.grabbed {
            Color::RED
        } else {
            Color::from_rgb(0.6, 0.8, 1.0)
        };

        // Draw a circle at the mouse pointer location
        graphics.draw_circle(self.offset, 20.0, color);

        graphics.draw_text((20.0, 20.0), Color::BLACK, &self.text);
    }

    fn on_mouse_move(&mut self, helper: &mut WindowHelper, position: Vector2<f32>)
    {
        log::info!(
            "Got on_mouse_move callback: ({:.1}, {:.1})",
            position.x,
            position.y
        );

        if self.grabbed {
            self.offset = self.offset + position;
            self.offset.x = self.offset.x.rem_euclid(self.window_size.x as f32);
            self.offset.y = self.offset.y.rem_euclid(self.window_size.y as f32);
        } else {
            self.offset = position;
        }

        helper.request_redraw();
    }

    fn on_mouse_button_down(&mut self, helper: &mut WindowHelper, button: MouseButton)
    {
        log::info!("Got on_mouse_button_down callback: {:?}", button);

        if button == MouseButton::Left {
            helper.set_cursor_grab(true).unwrap();
        } else {
            helper.set_cursor_grab(false).unwrap();
        }
    }

    fn on_keyboard_char(
        &mut self,
        helper: &mut WindowHelper<()>,
        _unicode_codepoint: char
    )
    {
        helper.set_cursor_grab(false).unwrap();
    }
}
