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

use buttons::*;
use speedy2d::color::Color;
use speedy2d::dimen::{Vec2, Vector2};
use speedy2d::font::Font;
use speedy2d::time::Stopwatch;
use speedy2d::window::{
    KeyScancode,
    ModifiersState,
    MouseButton,
    MouseScrollDistance,
    VirtualKeyCode,
    WindowFullscreenMode,
    WindowHandler,
    WindowHelper,
    WindowStartupInfo
};
use speedy2d::{Graphics2D, WebCanvas};

#[cfg(not(target_arch = "wasm32"))]
compile_error!("This sample only builds for WebAssembly (wasm32)");

mod buttons;

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
enum UserEvent
{
    ButtonClickGrabMouse,
    ButtonClickEnableFullscreen,
    ButtonClickTerminate
}

struct MyHandler
{
    font: Font,
    timer: Stopwatch,
    buttons: ButtonGroup<UserEvent>,
    scale: f32
}

impl WindowHandler<UserEvent> for MyHandler
{
    fn on_start(&mut self, helper: &mut WindowHelper<UserEvent>, info: WindowStartupInfo)
    {
        helper.set_title("Speedy2D WebGL Sample");

        self.scale = info.scale_factor() as f32;

        let event_sender = helper.create_user_event_sender();

        self.buttons.add(Button::new(
            "Grab mouse cursor",
            self.font.clone(),
            TriggerableEvent::new(&event_sender, UserEvent::ButtonClickGrabMouse)
        ));

        self.buttons.add(Button::new(
            "Enable fullscreen",
            self.font.clone(),
            TriggerableEvent::new(&event_sender, UserEvent::ButtonClickEnableFullscreen)
        ));

        self.buttons.add(Button::new(
            "Terminate",
            self.font.clone(),
            TriggerableEvent::new(&event_sender, UserEvent::ButtonClickTerminate)
        ));
    }

    fn on_user_event(
        &mut self,
        helper: &mut WindowHelper<UserEvent>,
        user_event: UserEvent
    )
    {
        log::info!("Got user event: {:?}", user_event);
        match user_event {
            UserEvent::ButtonClickGrabMouse => helper.set_cursor_grab(true).unwrap(),
            UserEvent::ButtonClickEnableFullscreen => {
                helper.set_fullscreen_mode(WindowFullscreenMode::FullscreenBorderless)
            }
            UserEvent::ButtonClickTerminate => helper.terminate_loop()
        }
    }

    fn on_mouse_grab_status_changed(
        &mut self,
        _helper: &mut WindowHelper<UserEvent>,
        mouse_grabbed: bool
    )
    {
        log::info!("Mouse grab status changed: {}", mouse_grabbed)
    }

    fn on_fullscreen_status_changed(
        &mut self,
        _helper: &mut WindowHelper<UserEvent>,
        fullscreen: bool
    )
    {
        log::info!("Fullscreen status changed callback: {}", fullscreen)
    }

    fn on_scale_factor_changed(
        &mut self,
        _helper: &mut WindowHelper<UserEvent>,
        scale_factor: f64
    )
    {
        log::info!("Scale factor is now {}", scale_factor);
        self.scale = scale_factor as f32;
    }

    fn on_draw(&mut self, helper: &mut WindowHelper<UserEvent>, graphics: &mut Graphics2D)
    {
        graphics.clear_screen(Color::from_rgb(0.9, 0.95, 1.0));

        self.buttons
            .draw(graphics, Vec2::new(20.0, 20.0), self.scale);

        let elapsed_secs = self.timer.secs_elapsed();

        let center = Vec2::new(400.0, 400.0);
        let offset = 200.0;

        let position = center
            + Vector2::<f64>::new(elapsed_secs.cos() * offset, elapsed_secs.sin() * offset)
                .into_f32();

        graphics.draw_circle(
            position * self.scale,
            75.0 * self.scale,
            Color::from_rgb(0.6, 0.8, 1.0)
        );

        // Request that we draw another frame once this one has finished
        helper.request_redraw();
    }

    fn on_mouse_move(&mut self, _helper: &mut WindowHelper<UserEvent>, position: Vec2)
    {
        self.buttons.on_mouse_move(position);
    }

    fn on_mouse_button_down(
        &mut self,
        _helper: &mut WindowHelper<UserEvent>,
        button: MouseButton
    )
    {
        if button == MouseButton::Left {
            self.buttons.on_mouse_left_down();
        }
    }

    fn on_mouse_button_up(
        &mut self,
        _helper: &mut WindowHelper<UserEvent>,
        button: MouseButton
    )
    {
        if button == MouseButton::Left {
            self.buttons.on_mouse_left_up();
        }
    }

    fn on_mouse_wheel_scroll(
        &mut self,
        _helper: &mut WindowHelper<UserEvent>,
        distance: MouseScrollDistance
    )
    {
        log::info!("on_mouse_wheel_scroll: {:?}", distance)
    }

    fn on_key_down(
        &mut self,
        _helper: &mut WindowHelper<UserEvent>,
        virtual_key_code: Option<VirtualKeyCode>,
        scancode: KeyScancode
    )
    {
        log::info!(
            "on_key_down: key='{:?}' code='{}'",
            virtual_key_code,
            scancode
        );
    }

    fn on_key_up(
        &mut self,
        _helper: &mut WindowHelper<UserEvent>,
        virtual_key_code: Option<VirtualKeyCode>,
        scancode: KeyScancode
    )
    {
        log::info!(
            "on_key_up: key='{:?}' code='{}'",
            virtual_key_code,
            scancode
        );
    }

    fn on_keyboard_char(
        &mut self,
        _helper: &mut WindowHelper<UserEvent>,
        unicode_codepoint: char
    )
    {
        log::info!("on_keyboard_char: '{}'", unicode_codepoint);
    }

    fn on_keyboard_modifiers_changed(
        &mut self,
        _helper: &mut WindowHelper<UserEvent>,
        state: ModifiersState
    )
    {
        log::info!("on_keyboard_modifiers_changed: {:?}", state);
    }
}

fn main()
{
    wasm_logger::init(wasm_logger::Config::default());
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));

    log::info!("Speedy2D WebGL sample");

    let font =
        Font::new(include_bytes!("../../../assets/fonts/NotoSans-Regular.ttf")).unwrap();

    WebCanvas::new_for_id_with_user_events(
        "my_canvas",
        MyHandler {
            font,
            timer: Stopwatch::new().unwrap(),
            buttons: ButtonGroup::new(),
            scale: 1.0
        }
    )
    .unwrap();
}
