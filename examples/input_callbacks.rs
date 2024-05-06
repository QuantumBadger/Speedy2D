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

use log::LevelFilter;
use speedy2d::color::Color;
use speedy2d::dimen::{UVec2, Vec2};
use speedy2d::window::{
    KeyScancode,
    ModifiersState,
    MouseButton,
    MouseScrollDistance,
    VirtualKeyCode,
    WindowHandler,
    WindowHelper,
    WindowStartupInfo
};
use speedy2d::{Graphics2D, Window};

fn main()
{
    simple_logger::SimpleLogger::new()
        .with_level(LevelFilter::Info)
        .init()
        .unwrap();

    let window =
        Window::new_centered("Speedy2D: Input Callbacks Example", (640, 480)).unwrap();

    window.run_loop(MyWindowHandler {
        mouse_pos: Vec2::ZERO,
        mouse_button_down: false
    })
}

struct MyWindowHandler
{
    mouse_pos: Vec2,
    mouse_button_down: bool
}

impl WindowHandler for MyWindowHandler
{
    fn on_start(&mut self, helper: &mut WindowHelper, info: WindowStartupInfo)
    {
        log::info!("Got on_start callback: {:?}", info);
        helper.set_cursor_visible(false);
        helper.set_resizable(false);
    }

    fn on_resize(&mut self, _helper: &mut WindowHelper, size_pixels: UVec2)
    {
        log::info!("Got on_resize callback: {:?}", size_pixels);
    }

    fn on_scale_factor_changed(&mut self, _helper: &mut WindowHelper, scale_factor: f64)
    {
        log::info!("Got on_scale_factor_changed callback: {:.3}", scale_factor);
    }

    fn on_draw(&mut self, _helper: &mut WindowHelper, graphics: &mut Graphics2D)
    {
        // Clear the screen
        graphics.clear_screen(Color::from_rgb(0.8, 0.9, 1.0));

        // Red for down, blue for up
        let color = match self.mouse_button_down {
            true => Color::RED,
            false => Color::BLUE
        };

        // Draw a circle at the mouse pointer location
        graphics.draw_circle(self.mouse_pos, 20.0, color);
    }

    fn on_mouse_move(&mut self, helper: &mut WindowHelper, position: Vec2)
    {
        log::info!(
            "Got on_mouse_move callback: ({:.1}, {:.1})",
            position.x,
            position.y
        );

        self.mouse_pos = position;

        helper.request_redraw();
    }

    fn on_mouse_button_down(&mut self, helper: &mut WindowHelper, button: MouseButton)
    {
        log::info!("Got on_mouse_button_down callback: {:?}", button);

        if button == MouseButton::Left {
            self.mouse_button_down = true;
        }

        helper.request_redraw();
    }

    fn on_mouse_button_up(&mut self, helper: &mut WindowHelper, button: MouseButton)
    {
        log::info!("Got on_mouse_button_up callback: {:?}", button);

        if button == MouseButton::Left {
            self.mouse_button_down = false;
        }

        helper.request_redraw();
    }

    fn on_mouse_wheel_scroll(
        &mut self,
        _helper: &mut WindowHelper<()>,
        delta: MouseScrollDistance
    )
    {
        log::info!("Got on_mouse_wheel_scroll callback: {:?}", delta);
    }

    fn on_key_down(
        &mut self,
        _helper: &mut WindowHelper,
        virtual_key_code: Option<VirtualKeyCode>,
        scancode: KeyScancode
    )
    {
        log::info!(
            "Got on_key_down callback: {:?}, scancode {}",
            virtual_key_code,
            scancode
        );
    }

    fn on_key_up(
        &mut self,
        _helper: &mut WindowHelper,
        virtual_key_code: Option<VirtualKeyCode>,
        scancode: KeyScancode
    )
    {
        log::info!(
            "Got on_key_up callback: {:?}, scancode {}",
            virtual_key_code,
            scancode
        );
    }

    fn on_keyboard_char(&mut self, _helper: &mut WindowHelper, unicode_codepoint: char)
    {
        log::info!("Got on_keyboard_char callback: '{}'", unicode_codepoint);
    }

    fn on_keyboard_modifiers_changed(
        &mut self,
        _helper: &mut WindowHelper,
        state: ModifiersState
    )
    {
        log::info!("Got on_keyboard_modifiers_changed callback: {:?}", state);
    }
}
