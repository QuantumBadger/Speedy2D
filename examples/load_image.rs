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
use speedy2d::image::{ImageHandle, ImageSmoothingMode};
use speedy2d::window::{WindowHandler, WindowHelper};
use speedy2d::{Graphics2D, Window};

fn main() {
    simple_logger::SimpleLogger::new().init().unwrap();

    let window = Window::new_centered("Speedy2D: Load image", (800, 600)).unwrap();

    window.run_loop(MyWindowHandler { image: None })
}

struct MyWindowHandler {
    image: Option<ImageHandle>,
}

impl WindowHandler for MyWindowHandler {
    fn on_draw(&mut self, helper: &mut WindowHelper, graphics: &mut Graphics2D) {
        if self.image.is_none() {
            let image = graphics
                .create_image_from_file_path(
                    None,
                    ImageSmoothingMode::NearestNeighbor,
                    "assets/screenshots/hello_world.png",
                )
                .unwrap();

            helper.set_size_pixels(*image.size());
            self.image = Some(image);
        }

        graphics.clear_screen(Color::WHITE);

        graphics.draw_image((0.0, 0.0), self.image.as_ref().unwrap());
    }
}
