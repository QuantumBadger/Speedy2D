## 1.0.0

* Initial release

## 1.0.1

* Fixed issue when deleting a texture while binding a new one

## 1.0.2

* Fixed build issue when using without windowing enabled

## 1.0.3

* Fixed negative overflow issue where monitor size is misdetected or less than window size

## 1.0.4

* No longer specifies core profile for GL 2.0 (fixes issue on Macs)

## 1.0.5

* Setting multisampling level to 16 by default

## 1.0.6

* Ensure event loop gets woken up when redrawing on Windows

## 1.0.7

* Fix error on some platforms due to GL shader program validation

## 1.1.0

### New APIs

* `WindowCreationOptions::with_resizable()`
* `WindowCreationOptions::with_always_on_top()`
* `WindowCreationOptions::with_maximized()`
* `WindowCreationOptions::with_decorations()`
* `Graphics2D::create_image_from_file_path()`
* `Graphics2D::create_image_from_file_bytes()`
* `GLRenderer::create_image_from_file_path()`
* `GLRenderer::create_image_from_file_bytes()`
* `GLRenderer::new_for_gl_context()`, to create a `GLRenderer` from a GL loader function

### Other changes

* `Graphics2D::draw_image()` is now able to take a tuple as a position
* When creating an image from raw bytes, the number of bytes is checked
* Fixed texture load issues where the horizontal byte stride was not a multiple of 4

## 1.1.1

* Now works correctly in Wayland
* Fixed error when primary monitor is not found

## 1.1.2

* Fixed issue with lines under text when a dark background and antialiasing are used

## 1.2

### New APIs

Thanks to [Revertron](https://github.com/Revertron):

* `Graphics2D::set_clip()`
* `ModifiersState::default()`

## 1.3

* WebGL support introduced

### New APIs

* `WebCanvas`, providing full rendering and event handling for an HTML canvas
* `GLRenderer::new_for_web_canvas_by_id()`, for rendering only (no event handling)
* The `time` module, for access to the system clock in a cross-platform way.

### New callbacks

* `WindowHandler::on_mouse_grab_status_changed()`
* `WindowHandler::on_fullscreen_status_changed()`

## 1.3.1

* Ensure that mouse position is scaled using device pixel ratio

## 1.4.0

* Line breaks (`\n`) now handled when laying out text

### New APIs

* `WindowHandler::on_mouse_wheel_scroll()` (thanks
  to [GreatGodOfFire](https://github.com/GreatGodOfFire))
* `TextLayout::empty_line_vertical_metrics()`

## 1.5.0

* Ability to draw polygons (thanks to [chilipepperhott](https://github.com/chilipepperhott))

## 1.6.0

Thanks to [dnlmlr](https://github.com/dnlmlr):

* Set the position before making the window visible
* Added option to create transparent windows

Thanks to [UdHo](https://github.com/UdHo)

* Fixed Wayland regression

## 1.7.0

* Remove `Sized` requirement from `TextLayout` functions (allowing them to be called
  on `&dyn TextLayout`)

## 1.8.0

* Added convenient type aliases for:
    * `Vector2` types: `Vec2`, `IVec2`, and `UVec2`
    * `Rectangle` types: `Rect`, `IRect`, and `URect`
* Allow adding tuples to vectors, for example `my_vec + (1.0, 2.0)`
* Added example code for managing GL context with glutin (thanks
  to [btbaggin](https://github.com/btbaggin))

### New APIs

* `Vector2::new_x()`
* `Vector2::new_y()`
* `Rectangle::with_negative_offset()`
* `Rectangle::ZERO`
* `Color::from_gray()`

## 1.9.0

### New APIs

* `Graphics2D::draw_text_cropped()`, for efficiently drawing a block of text cropped to the
  specified area
* `Graphics2D::capture()`, for capturing the current contents of the window
* Added assignment operators (`+=`, `-=`, `*=`, `/=`) to `Vector2` (thanks
  to [amarao](https://github.com/amarao))

### Fixes

* Text drawn at a non-pixel-aligned position was getting snapped to the nearest pixel -- this is now
  fixed.

## 1.10.0

### New APIs

* `Rectangle::as_f32()`

### Fixes

* Fix for issue #55 (text appearing as solid rectangles due to texture not being unbound correctly)

## 1.11.0

### Fixes

* Fix for issue #74 (in some cases, dropping an `ImageHandle` resulted in a panic)

## 1.12.0

### Fixes

* Fix for issue #34 (incorrect alpha compositing on a transparent background)

## 1.13.0

### Changed APIs

* Functions which previously took a `Rectangle` now accept anything which implements
  `AsRef<Rectangle>`. This allows you to either pass in a reference to a Rectangle,
  or move the Rectangle as before.

### New APIs

* `WindowHelper.get_size_pixels()` (thanks to [dennisorlando](https://github.com/dennisorlando))

## 2.0.0

### Changed APIs

* APIs which previously returned or accepted an `Rc<FormattedTextBlock>` now use
  a `FormattedTextBlock` directly
* `FormattedTextBlock` can now be cheaply cloned, and sent between threads.
* Removed the deprecated function `new_for_current_context` -- please switch
  to `new_for_gl_context` instead.
* `UserEventSender` now implements `Clone` even if the event type doesn't (thanks
  to [Alex Kladov](https://github.com/matklad))

### New APIs

* `TextOptions::with_trim_each_line()` (thanks
  to [InfiniteCoder01](https://github.com/InfiniteCoder01))

## 2.1.0

### New APIs

Ability to draw rounded rectangles, thanks to [dennisorlando](https://github.com/dennisorlando):

* `RoundedRectangle` struct
* `Rectangle.rounded(radius)`
* `Graphics2D.draw_rounded_rectangle()`

## 3.0.0

Updated to the latest version of Glutin.

### New APIs

* `MouseButton.Back` and `MouseButton.Forward`
* `WindowCreationError.EventLoopCreationFailed`

### Changed APIs

* `MouseButton`, `VirtualKeyCode`, and `WindowCreationError` are marked as `non_exhaustive`