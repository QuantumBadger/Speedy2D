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
use speedy2d::window::UserEventSender;
use speedy2d::Graphics2D;

pub struct TriggerableEvent<UserEventType: Clone + 'static> {
    sender: UserEventSender<UserEventType>,
    event: UserEventType,
}

impl<UserEventType: Clone> TriggerableEvent<UserEventType> {
    pub fn new(sender: &UserEventSender<UserEventType>, event: UserEventType) -> Self {
        TriggerableEvent {
            sender: sender.clone(),
            event,
        }
    }

    pub fn trigger(&self) {
        self.sender.send_event(self.event.clone()).unwrap()
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub enum ButtonMouseState {
    None,
    ClickingOnThis,
    ClickingOnOther,
}

pub struct Button<UserEventType: Clone + 'static> {
    text: String,
    font: Font,
    text_formatted: Option<Rc<FormattedTextBlock>>,
    text_position: Vector2<f32>,
    position: Rectangle,
    currently_hovering: bool,
    mouse_state: ButtonMouseState,
    action: TriggerableEvent<UserEventType>,
}

impl<UserEventType: Clone + 'static> Button<UserEventType> {
    const TEXT_SIZE: f32 = 16.0;
    const PADDING: f32 = 10.0;

    const COLOR_TEXT: Color = Color::BLACK;
    const COLOR_NORMAL: Color = Color::from_rgb(0.8, 0.9, 1.0);
    const COLOR_HOVER: Color = Color::from_rgb(0.7, 0.85, 1.0);
    const COLOR_CLICK: Color = Color::from_rgb(0.6, 0.8, 1.0);

    pub fn new<S: AsRef<str>>(
        text: S,
        font: Font,
        action: TriggerableEvent<UserEventType>,
    ) -> Self {
        Self {
            text: String::from(text.as_ref()),
            font,
            text_formatted: None,
            text_position: Vector2::ZERO,
            position: Rectangle::new(Vector2::ZERO, Vector2::ZERO),
            currently_hovering: false,
            mouse_state: ButtonMouseState::None,
            action,
        }
    }

    pub fn on_mouse_move(&mut self, position: Vector2<f32>) {
        self.currently_hovering = self.position.contains(position);
    }

    pub fn on_mouse_left_down(&mut self) {
        self.mouse_state = if self.currently_hovering {
            ButtonMouseState::ClickingOnThis
        } else {
            ButtonMouseState::ClickingOnOther
        }
    }

    pub fn on_mouse_left_up(&mut self) {
        if self.mouse_state == ButtonMouseState::ClickingOnThis && self.currently_hovering
        {
            log::info!("Clicked: {}", self.text);
            self.action.trigger();
        }

        self.mouse_state = ButtonMouseState::None;
    }

    pub fn layout(&mut self, top_left: Vector2<f32>, scale: f32) {
        let text_formatted = self.font.layout_text(
            self.text.as_str(),
            Self::TEXT_SIZE * scale,
            TextOptions::new(),
        );

        self.text_formatted = Some(text_formatted.clone());

        self.position = Rectangle::new(
            top_left.round(),
            (top_left
                + text_formatted.size()
                + Vector2::new(Self::PADDING, Self::PADDING) * 2.0 * scale)
                .round(),
        );

        self.text_position =
            top_left + Vector2::new(Self::PADDING, Self::PADDING) * scale;
    }

    pub fn draw(&mut self, graphics: &mut Graphics2D) {
        let color = if self.currently_hovering {
            match self.mouse_state {
                ButtonMouseState::None => Self::COLOR_HOVER,
                ButtonMouseState::ClickingOnThis => Self::COLOR_CLICK,
                ButtonMouseState::ClickingOnOther => Self::COLOR_NORMAL,
            }
        } else {
            match self.mouse_state {
                ButtonMouseState::None => Self::COLOR_NORMAL,
                ButtonMouseState::ClickingOnThis => Self::COLOR_HOVER,
                ButtonMouseState::ClickingOnOther => Self::COLOR_NORMAL,
            }
        };

        graphics.draw_rectangle(self.position.clone(), color);
        graphics.draw_text(
            self.text_position,
            Self::COLOR_TEXT,
            self.text_formatted.as_ref().unwrap(),
        );
    }

    pub fn width(&self) -> f32 {
        self.position.width()
    }
}

pub struct ButtonGroup<UserEventType: Clone + 'static> {
    buttons: Vec<Button<UserEventType>>,
    layout_position: Option<Vector2<f32>>,
    layout_scale: Option<f32>,
}

impl<UserEventType: Clone + 'static> ButtonGroup<UserEventType> {
    const GAP: f32 = 10.0;

    pub fn new() -> Self {
        Self {
            buttons: Vec::new(),
            layout_position: None,
            layout_scale: None,
        }
    }

    pub fn add(&mut self, button: Button<UserEventType>) {
        self.buttons.push(button);
        self.layout_position = None;
    }

    pub fn draw(
        &mut self,
        graphics: &mut Graphics2D,
        top_left: Vector2<f32>,
        scale: f32,
    ) {
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

    pub fn on_mouse_move(&mut self, position: Vector2<f32>) {
        for button in &mut self.buttons {
            button.on_mouse_move(position)
        }
    }

    pub fn on_mouse_left_down(&mut self) {
        for button in &mut self.buttons {
            button.on_mouse_left_down()
        }
    }

    pub fn on_mouse_left_up(&mut self) {
        for button in &mut self.buttons {
            button.on_mouse_left_up()
        }
    }
}
