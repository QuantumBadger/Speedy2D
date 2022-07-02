use glutin::dpi::PhysicalSize;
use glutin::event::{Event, WindowEvent};
use glutin::event_loop::{ControlFlow, EventLoop};
use glutin::window::WindowBuilder;
use speedy2d::color::Color;
use speedy2d::font::{Font, FormattedTextBlock, TextLayout, TextOptions};
use speedy2d::{GLRenderer, Graphics2D};

fn main()
{
    let event_loop = EventLoop::new();

    let builder = WindowBuilder::new()
        .with_title("Speedy2D: Hello World")
        .with_visible(true)
        .with_inner_size(PhysicalSize::new(640, 240));

    let context = create_best_context(&builder, &event_loop).unwrap();
    let context = unsafe { context.make_current().unwrap() };
    let mut renderer = unsafe {
        GLRenderer::new_for_gl_context((640, 240), |fn_name| {
            context.get_proc_address(fn_name) as *const _
        })
        .unwrap()
    };

    let font = Font::new(include_bytes!("../assets/fonts/NotoSans-Regular.ttf")).unwrap();
    let text = font.layout_text("Hello world!", 64.0, TextOptions::new());

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                _ => {}
            },

            Event::RedrawRequested(_) => context.window().request_redraw(),
            Event::RedrawEventsCleared => {
                renderer.draw_frame(|graphics| {
                    render_frame(graphics, text.clone());
                });
                context.swap_buffers().unwrap();
            }
            _ => {}
        }
    });
}

fn render_frame(graphics: &mut Graphics2D, text: std::rc::Rc<FormattedTextBlock>)
{
    graphics.clear_screen(Color::WHITE);
    graphics.draw_circle((150.0, 120.0), 75.0, Color::from_rgb(0.8, 0.9, 1.0));
    graphics.draw_text((290.0, 90.0), Color::BLACK, &text);
}

fn create_best_context(
    window_builder: &WindowBuilder,
    event_loop: &EventLoop<()>
) -> Option<glutin::WindowedContext<glutin::NotCurrent>>
{
    for vsync in &[true, false] {
        for multisampling in &[16, 8, 4, 2, 1, 0] {
            let mut windowed_context = glutin::ContextBuilder::new()
                .with_vsync(*vsync)
                .with_gl(glutin::GlRequest::Specific(glutin::Api::OpenGl, (2, 0)));

            if *multisampling > 1 {
                windowed_context = windowed_context.with_multisampling(*multisampling);
            }

            let result =
                windowed_context.build_windowed(window_builder.clone(), event_loop);

            match result {
                Ok(context) => {
                    return Some(context);
                }
                Err(_) => {}
            }
        }
    }

    None
}
