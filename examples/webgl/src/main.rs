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
use speedy2d::window::{WindowHandler, WindowHelper, WindowStartupInfo};
use speedy2d::{GLRenderer, WebCanvas};

#[cfg(not(target_arch = "wasm32"))]
compile_error!("This sample only builds for WebAssembly (wasm32)");

struct MyHandler {}

impl WindowHandler for MyHandler
{
    fn on_start(&mut self, _helper: &mut WindowHelper, _info: WindowStartupInfo) {}

    fn on_mouse_move(&mut self, _helper: &mut WindowHelper, position: Vector2<f32>)
    {
        log::info!("Mouse is now at {:?}", position);
    }
}

fn main()
{
    wasm_logger::init(wasm_logger::Config::default());
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));

    log::info!("Speedy2D WebGL sample");

    let mut renderer =
        GLRenderer::new_for_web_canvas_by_id((640, 480), "my_canvas").unwrap();

    let font =
        Font::new(include_bytes!("../../../assets/fonts/NotoSans-Regular.ttf")).unwrap();

    renderer.draw_frame(|graphics| {
        graphics.clear_screen(Color::BLUE);

        graphics.draw_rectangle(
            Rectangle::from_tuples((100.0, 200.0), (200.0, 300.0)),
            Color::WHITE
        );

        graphics.draw_text(
            (100.0, 100.0),
            Color::RED,
            &font.layout_text("WebGL Hello World", 24.0, TextOptions::new())
        );
    });

    WebCanvas::new_for_id("my_canvas", MyHandler {}).unwrap();
}
