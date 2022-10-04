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
use std::time::Instant;

use speedy2d::color::Color;
use speedy2d::dimen::Vec2;
use speedy2d::font::{Font, FormattedTextBlock, TextLayout, TextOptions};
use speedy2d::window::{WindowHandler, WindowHelper};
use speedy2d::{Graphics2D, Window};

fn main()
{
    simple_logger::SimpleLogger::new().init().unwrap();

    let window = Window::new_centered("Speedy2D: Moving Text", (800, 800)).unwrap();

    let font = Font::new(include_bytes!("../assets/fonts/NotoSans-Regular.ttf")).unwrap();

    let text = font.layout_text("Hello world!", 64.0, TextOptions::new());

    window.run_loop(MyWindowHandler {
        text,
        start_time: Instant::now()
    })
}

struct MyWindowHandler
{
    text: Rc<FormattedTextBlock>,
    start_time: Instant
}

impl WindowHandler for MyWindowHandler
{
    fn on_draw(&mut self, helper: &mut WindowHelper, graphics: &mut Graphics2D)
    {
        graphics.clear_screen(Color::WHITE);

        let elapsed_secs = self.start_time.elapsed().as_secs_f32();

        let center = Vec2::new(200.0, 200.0);
        let offset = 50.0;

        let position =
            center + Vec2::new(elapsed_secs.cos() * offset, elapsed_secs.sin() * offset);

        graphics.draw_text(position, Color::BLACK, &self.text);

        // Request that we draw another frame once this one has finished
        helper.request_redraw();
    }
}
