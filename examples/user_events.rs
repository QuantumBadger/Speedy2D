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

use std::time::Duration;

use speedy2d::color::Color;
use speedy2d::dimen::UVec2;
use speedy2d::window::{
    UserEventSender,
    WindowCreationOptions,
    WindowHandler,
    WindowHelper,
    WindowSize,
    WindowStartupInfo
};
use speedy2d::{Graphics2D, Window};

fn main()
{
    simple_logger::SimpleLogger::new().init().unwrap();

    // Change "String" below to any event type of your choice
    let window: Window<String> = Window::new_with_user_events(
        "Speedy2D: User Events Example",
        WindowCreationOptions::new_windowed(
            WindowSize::PhysicalPixels(UVec2::new(640, 480)),
            None
        )
    )
    .unwrap();

    // Creates a UserEventSender, which can be used to post custom
    // events to this event loop from another thread.
    //
    // It's also possible to create an event sender using
    // `WindowHelper::create_user_event_sender()`.
    let user_event_sender = window.create_user_event_sender();

    window.run_loop(MyWindowHandler { user_event_sender })
}

struct MyWindowHandler
{
    user_event_sender: UserEventSender<String>
}

impl WindowHandler<String> for MyWindowHandler
{
    fn on_start(&mut self, _helper: &mut WindowHelper<String>, _info: WindowStartupInfo)
    {
        let user_event_sender = self.user_event_sender.clone();

        std::thread::spawn(move || {
            loop {
                // Send a message every 300ms
                user_event_sender
                    .send_event("Message from thread".to_string())
                    .unwrap();
                std::thread::sleep(Duration::from_millis(300));
            }
        });
    }

    fn on_user_event(&mut self, _helper: &mut WindowHelper<String>, user_event: String)
    {
        log::info!("Received user event: '{}'", user_event);
    }

    fn on_draw(&mut self, _helper: &mut WindowHelper<String>, graphics: &mut Graphics2D)
    {
        graphics.clear_screen(Color::from_rgb(0.8, 0.9, 1.0));

        self.user_event_sender
            .send_event("Message from on_draw".to_string())
            .unwrap();
    }
}
