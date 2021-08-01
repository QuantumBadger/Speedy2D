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

use speedy2d::color::Color;
use speedy2d::dimen::Vector2;
use speedy2d::font::{Font, TextLayout, TextOptions};
use speedy2d::shape::Rectangle;
use speedy2d::time::Timer;
use speedy2d::window::{MouseButton, WindowHandler, WindowHelper, WindowStartupInfo};
use speedy2d::{Graphics2D, WebCanvas};

#[cfg(not(target_arch = "wasm32"))]
compile_error!("This sample only builds for WebAssembly (wasm32)");

struct MyHandler
{
    font: Font,
    timer: Timer
}

impl WindowHandler for MyHandler
{
    fn on_start(&mut self, _helper: &mut WindowHelper, _info: WindowStartupInfo) {}

    fn on_mouse_grab_status_changed(
        &mut self,
        _helper: &mut WindowHelper<()>,
        mouse_grabbed: bool
    )
    {
        log::info!("Mouse grab status changed: {}", mouse_grabbed)
    }

    fn on_draw(&mut self, helper: &mut WindowHelper<()>, graphics: &mut Graphics2D)
    {
        graphics.clear_screen(Color::from_rgb(0.8, 0.9, 1.0));

        graphics.draw_rectangle(
            Rectangle::from_tuples((100.0, 200.0), (200.0, 300.0)),
            Color::WHITE
        );

        // TODO layout text at start
        // TODO add explanation of grabbing
        graphics.draw_text(
            (100.0, 100.0),
            Color::BLACK,
            &self
                .font
                .layout_text("WebGL Hello World", 24.0, TextOptions::new())
        );

        let elapsed_secs = self.timer.secs_elapsed();

        let center = Vector2::new(400.0, 400.0);
        let offset = 200.0;

        let position = center
            + Vector2::new(elapsed_secs.cos() * offset, elapsed_secs.sin() * offset)
                .into_f32();

        graphics.draw_circle(position, 75.0, Color::from_rgb(0.6, 0.8, 1.0));

        // Request that we draw another frame once this one has finished
        helper.request_redraw();
    }

    fn on_mouse_move(&mut self, helper: &mut WindowHelper, position: Vector2<f32>)
    {
        helper.set_title(format!("Mouse position: ({}, {})", position.x, position.y));
    }

    fn on_mouse_button_down(&mut self, helper: &mut WindowHelper<()>, button: MouseButton)
    {
        log::info!("Button down: {:?}", button);
        //helper.terminate_loop();
        helper.set_cursor_grab(button == MouseButton::Left).unwrap();
    }

    fn on_mouse_button_up(&mut self, _helper: &mut WindowHelper<()>, button: MouseButton)
    {
        log::info!("Button up: {:?}", button);
    }
}

fn main()
{
    wasm_logger::init(wasm_logger::Config::default());
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));

    log::info!("Speedy2D WebGL sample");

    let font =
        Font::new(include_bytes!("../../../assets/fonts/NotoSans-Regular.ttf")).unwrap();

    WebCanvas::new_for_id(
        "my_canvas",
        MyHandler {
            font,
            timer: Timer::new().unwrap()
        },
        None
    )
    .unwrap();
}
