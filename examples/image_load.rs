use image::GenericImageView;
use speedy2d::color::Color;
use speedy2d::dimen::Vector2;
use speedy2d::image::{ImageDataType, ImageSmoothingMode};
use speedy2d::window::{WindowHandler, WindowHelper};
use speedy2d::{Graphics2D, Window};

fn main()
{
    simple_logger::SimpleLogger::new().init().unwrap();
    let window = Window::new_centered("Speedy2D: Hello World", (640, 240)).unwrap();
    let image = image::open("test/assets/expected_images/test_half_circle.png").unwrap();
    window.run_loop(MyWindowHandler { image })
}

struct MyWindowHandler
{
    image: image::DynamicImage
}

impl WindowHandler for MyWindowHandler
{
    fn on_draw(&mut self, helper: &mut WindowHelper, graphics: &mut Graphics2D)
    {
        graphics.clear_screen(Color::WHITE);
        let size = self.image.dimensions();
        let _handle = graphics
            .create_image_from_raw_pixels(
                ImageDataType::RGBA,
                ImageSmoothingMode::Linear,
                Vector2::new(size.0, size.1),
                &self.image.to_rgba8()
            )
            .unwrap();
        helper.request_redraw();
    }
}
