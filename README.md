# Speedy2D

[![Crate](https://img.shields.io/crates/v/speedy2d.svg)](https://crates.io/crates/speedy2d)
[![Documentation](https://docs.rs/speedy2d/badge.svg)](https://docs.rs/speedy2d)
[![CI](https://github.com/QuantumBadger/Speedy2D/actions/workflows/build.yml/badge.svg)](hhttps://github.com/QuantumBadger/Speedy2D/actions)

Hardware-accelerated drawing of shapes, images, and text, with an easy to
use API.

Speedy2D aims to be:

 - The simplest Rust API for creating a window, rendering graphics/text, and
   handling input
 - Compatible with any device supporting OpenGL 2.0+ or OpenGL ES 2.0+
 - Very fast

By default, Speedy2D contains support for setting up a window with an OpenGL
context, and receiving input events. If you'd like to handle this yourself, and
use Speedy2D only for rendering, you can disable the `windowing` feature.

## Useful Links

* Documentation and getting started guide: https://docs.rs/speedy2d
* Crate: https://crates.io/crates/speedy2d
* Example code: https://github.com/QuantumBadger/Speedy2D

## Example code

* [Hello world, with text rendering](examples/hello_world.rs)
* [All input callbacks](examples/input_callbacks.rs)
* [User-generated events](examples/user_events.rs)

The example projects can be run using `cargo run --example=hello_world` (just
change `hello_world` to the name of the example source file).

[![Screenshot](assets/screenshots/hello_world.png)](examples/hello_world.rs)

## Quick Start

**Step 1:** Add Speedy2D to your `Cargo.toml` dependencies:

```toml
[dependencies]
speedy2d = "1.0"
```

**Step 2:** Create a window:

```rust
use speedy2d::Window;

let window = Window::new_centered("Title", (640, 480)).unwrap();
```

**Step 3:** Create a struct implementing the `WindowHandler` trait. Override
whichever callbacks you're interested in, for example `on_draw()`,
`on_mouse_move()`, or `on_key_down()`.

```rust
use speedy2d::window::{WindowHandler, WindowHelper};
use speedy2d::Graphics2D;

struct MyWindowHandler {}

impl WindowHandler for MyWindowHandler
{
    fn on_draw(&mut self, helper: &mut WindowHelper, graphics: &mut Graphics2D)
    {
        graphics.clear_screen(Color::from_rgb(0.8, 0.9, 1.0));
        graphics.draw_circle((100.0, 100.0), 75.0, Color::BLUE);

        // Request that we draw another frame once this one has finished
        helper.request_redraw();
    }

   // If desired, on_mouse_move(), on_key_down(), etc...
}
```

**Step 4:** Finally, start the event loop by passing your new `WindowHandler`
to the `run_loop()` function. 

```rust
window.run_loop(MyWindowHandler{});
```

For a more detailed getting started guide, including a full list of `WindowHandler`
callbacks, and how to render text, go to
[docs.rs/speedy2d](https://docs.rs/speedy2d). 

### Alternative: Managing the GL context yourself

If you'd rather handle the window creation and OpenGL context management
yourself, simply disable Speedy2D's `windowing` feature in your `Cargo.toml`
file, and create a context as follows:

```rust
use speedy2d::GLRenderer;

let mut renderer = unsafe {
    GLRenderer::new_for_current_context((640, 480))
}.unwrap();
```

Then, draw a frame using `GLRenderer::draw_frame()`:

```rust
renderer.draw_frame(|graphics| {
    graphics.clear_screen(Color::WHITE);
    graphics.draw_circle((100.0, 100.0), 75.0, Color::BLUE);
});
```

## License

Speedy2D is licensed under the Apache license, version 2.0. See
[LICENSE](LICENSE) for more details.

## Contributing

Pull requests for Speedy2D are always welcome. Please ensure the following
checks pass locally before submitting:

```bash
cargo test
cargo test --no-default-features --lib --examples --tests
cargo clippy
cargo +nightly fmt -- --check
cargo doc
```

Some tests require the ability to create a headless OpenGL context.
