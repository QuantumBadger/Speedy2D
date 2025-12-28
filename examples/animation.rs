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

use std::time::Instant;

use log::LevelFilter;
use speedy2d::color::Color;
use speedy2d::dimen::Vec2;
use speedy2d::window::{WindowHandler, WindowHelper};
use speedy2d::{Graphics2D, Window};

fn main()
{
    simple_logger::SimpleLogger::new()
        .with_level(LevelFilter::Info)
        .init()
        .unwrap();

    let window = Window::new_centered("Speedy2D: Animation", (800, 800)).unwrap();

    window.run_loop(MyWindowHandler {
        start_time: Instant::now()
    })
}

struct MyWindowHandler
{
    start_time: Instant
}

impl WindowHandler for MyWindowHandler
{
    fn on_draw(&mut self, helper: &mut WindowHelper, graphics: &mut Graphics2D)
    {
        graphics.clear_screen(Color::WHITE);

        let elapsed_secs = self.start_time.elapsed().as_secs_f32();

        let center = Vec2::new(400.0, 400.0);
        let offset = 200.0;

        let position =
            center + Vec2::new(elapsed_secs.cos() * offset, elapsed_secs.sin() * offset);

        graphics.draw_circle(position, 75.0, Color::from_rgb(0.8, 0.9, 1.0));

        // Request that we draw another frame once this one has finished
        helper.request_redraw();
    }
}
