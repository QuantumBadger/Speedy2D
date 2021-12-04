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
use speedy2d::numeric::RoundFloat;
use speedy2d::shape::Rectangle;
use speedy2d::time::Timer;
use speedy2d::window::{
    KeyScancode,
    ModifiersState,
    MouseButton,
    UserEventSender,
    VirtualKeyCode,
    WindowFullscreenMode,
    WindowHandler,
    WindowHelper,
    WindowStartupInfo
};
use speedy2d::{Graphics2D, WebCanvas};

#[cfg(not(target_arch = "wasm32"))]
compile_error!("This sample only builds for WebAssembly (wasm32)");

// TODO make this sample much simpler, move this example code into a different
// file

struct TriggerableEvent<UserEventType: Clone + 'static>
{
    sender: UserEventSender<UserEventType>,
    event: UserEventType
}

impl<UserEventType: Clone> TriggerableEvent<UserEventType>
{
    pub fn new(sender: &UserEventSender<UserEventType>, event: UserEventType) -> Self
    {
        TriggerableEvent {
            sender: sender.clone(),
            event
        }
    }

    pub fn trigger(&self)
    {
        self.sender.send_event(self.event.clone()).unwrap()
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
enum UserEvent
{
    ButtonClickGrabMouse,
    ButtonClickEnableFullscreen,
    ButtonClickTerminate
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
enum ButtonMouseState
{
    None,
    ClickingOnThis,
    ClickingOnOther
}

struct Button
{
    text: String,
    font: Font,
    text_formatted: Option<Rc<FormattedTextBlock>>,
    text_position: Vector2<f32>,
    position: Rectangle,
    currently_hovering: bool,
    mouse_state: ButtonMouseState,
    action: TriggerableEvent<UserEvent>
}

impl Button
{
    const TEXT_SIZE: f32 = 16.0;
    const PADDING: f32 = 10.0;

    const COLOR_TEXT: Color = Color::BLACK;
    const COLOR_NORMAL: Color = Color::from_rgb(0.8, 0.9, 1.0);
    const COLOR_HOVER: Color = Color::from_rgb(0.7, 0.85, 1.0);
    const COLOR_CLICK: Color = Color::from_rgb(0.6, 0.8, 1.0);

    pub fn new<S: AsRef<str>>(
        text: S,
        font: Font,
        action: TriggerableEvent<UserEvent>
    ) -> Self
    {
        Self {
            text: String::from(text.as_ref()),
            font,
            text_formatted: None,
            text_position: Vector2::ZERO,
            position: Rectangle::new(Vector2::ZERO, Vector2::ZERO),
            currently_hovering: false,
            mouse_state: ButtonMouseState::None,
            action
        }
    }

    pub fn on_mouse_move(&mut self, position: Vector2<f32>)
    {
        self.currently_hovering = self.position.contains(position);
    }

    pub fn on_mouse_left_down(&mut self)
    {
        self.mouse_state = if self.currently_hovering {
            ButtonMouseState::ClickingOnThis
        } else {
            ButtonMouseState::ClickingOnOther
        }
    }

    pub fn on_mouse_left_up(&mut self)
    {
        if self.mouse_state == ButtonMouseState::ClickingOnThis && self.currently_hovering
        {
            log::info!("Clicked: {}", self.text);
            self.action.trigger();
        }

        self.mouse_state = ButtonMouseState::None;
    }

    pub fn layout(&mut self, top_left: Vector2<f32>, scale: f32)
    {
        let text_formatted = self.font.layout_text(
            self.text.as_str(),
            Self::TEXT_SIZE * scale,
            TextOptions::new()
        );

        self.text_formatted = Some(text_formatted.clone());

        self.position = Rectangle::new(
            top_left.round(),
            (top_left
                + text_formatted.size()
                + Vector2::new(Self::PADDING, Self::PADDING) * 2.0 * scale)
                .round()
        );

        self.text_position =
            top_left + Vector2::new(Self::PADDING, Self::PADDING) * scale;
    }

    pub fn draw(&mut self, graphics: &mut Graphics2D)
    {
        let color = if self.currently_hovering {
            match self.mouse_state {
                ButtonMouseState::None => Self::COLOR_HOVER,
                ButtonMouseState::ClickingOnThis => Self::COLOR_CLICK,
                ButtonMouseState::ClickingOnOther => Self::COLOR_NORMAL
            }
        } else {
            match self.mouse_state {
                ButtonMouseState::None => Self::COLOR_NORMAL,
                ButtonMouseState::ClickingOnThis => Self::COLOR_HOVER,
                ButtonMouseState::ClickingOnOther => Self::COLOR_NORMAL
            }
        };

        graphics.draw_rectangle(self.position.clone(), color);
        graphics.draw_text(
            self.text_position,
            Self::COLOR_TEXT,
            self.text_formatted.as_ref().unwrap()
        );
    }

    pub fn width(&self) -> f32
    {
        self.position.width()
    }
}

struct ButtonGroup
{
    buttons: Vec<Button>,
    layout_position: Option<Vector2<f32>>,
    layout_scale: Option<f32>
}

impl ButtonGroup
{
    const GAP: f32 = 10.0;

    pub fn new() -> Self
    {
        Self {
            buttons: Vec::new(),
            layout_position: None,
            layout_scale: None
        }
    }

    pub fn add(&mut self, button: Button)
    {
        self.buttons.push(button);
        self.layout_position = None;
    }

    pub fn draw(&mut self, graphics: &mut Graphics2D, top_left: Vector2<f32>, scale: f32)
    {
        if self.layout_position != Some(top_left) || self.layout_scale != Some(scale) {
            let mut x_pos = 0.0;

            for button in &mut self.buttons {
                button.layout(top_left + Vector2::new(x_pos, 0.0), scale);
                x_pos += button.width() + Self::GAP * scale;
            }

            self.layout_position = Some(top_left);
            self.layout_scale = Some(scale);
        }

        for button in &mut self.buttons {
            button.draw(graphics);
        }
    }

    pub fn on_mouse_move(&mut self, position: Vector2<f32>)
    {
        for button in &mut self.buttons {
            button.on_mouse_move(position)
        }
    }

    pub fn on_mouse_left_down(&mut self)
    {
        for button in &mut self.buttons {
            button.on_mouse_left_down()
        }
    }

    pub fn on_mouse_left_up(&mut self)
    {
        for button in &mut self.buttons {
            button.on_mouse_left_up()
        }
    }
}

struct MyHandler
{
    font: Font,
    timer: Timer,
    buttons: ButtonGroup,
    scale: f32
}

impl WindowHandler<UserEvent> for MyHandler
{
    fn on_start(&mut self, helper: &mut WindowHelper<UserEvent>, info: WindowStartupInfo)
    {
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
            .draw(graphics, Vector2::new(20.0, 20.0), self.scale);

        let elapsed_secs = self.timer.secs_elapsed();

        let center = Vector2::new(400.0, 400.0);
        let offset = 200.0;

        let position = center
            + Vector2::new(elapsed_secs.cos() * offset, elapsed_secs.sin() * offset)
                .into_f32();

        graphics.draw_circle(
            position * self.scale,
            75.0 * self.scale,
            Color::from_rgb(0.6, 0.8, 1.0)
        );

        // Request that we draw another frame once this one has finished
        helper.request_redraw();
    }

    fn on_mouse_move(
        &mut self,
        _helper: &mut WindowHelper<UserEvent>,
        position: Vector2<f32>
    )
    {
        self.buttons.on_mouse_move(position * self.scale);
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
        log::info!("Got on_keyboard_char callback: '{}'", unicode_codepoint);
    }

    fn on_keyboard_modifiers_changed(
        &mut self,
        _helper: &mut WindowHelper<UserEvent>,
        state: ModifiersState
    )
    {
        log::info!("Keyboard modifiers changed: {:?}", state);
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
            timer: Timer::new().unwrap(),
            buttons: ButtonGroup::new(),
            scale: 1.0
        },
        None
    )
    .unwrap();
}
