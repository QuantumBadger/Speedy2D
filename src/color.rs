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

/// A struct representing a color with red, green, blue, and alpha components.
/// Each component is stored as a float.
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Color
{
    r: f32,
    g: f32,
    b: f32,
    a: f32
}

impl Color
{
    /// Color constant for transparency, with the alpha value set to zero.
    pub const TRANSPARENT: Color = Color::from_rgba(0.0, 0.0, 0.0, 0.0);

    /// Constant for the color black.
    pub const BLACK: Color = Color::from_rgb(0.0, 0.0, 0.0);

    /// Constant for the color white.
    pub const WHITE: Color = Color::from_rgb(1.0, 1.0, 1.0);

    /// Constant for the color red.
    pub const RED: Color = Color::from_rgb(1.0, 0.0, 0.0);

    /// Constant for the color green.
    pub const GREEN: Color = Color::from_rgb(0.0, 1.0, 0.0);

    /// Constant for the color blue.
    pub const BLUE: Color = Color::from_rgb(0.0, 0.0, 1.0);

    /// Constant for the color yellow.
    pub const YELLOW: Color = Color::from_rgb(1.0, 1.0, 0.0);

    /// Constant for the color cyan.
    pub const CYAN: Color = Color::from_rgb(0.0, 1.0, 1.0);

    /// Constant for the color magenta.
    pub const MAGENTA: Color = Color::from_rgb(1.0, 0.0, 1.0);

    /// Constant for the color gray.
    pub const GRAY: Color = Color::from_rgb(0.5, 0.5, 0.5);

    /// Constant for the color light gray.
    pub const LIGHT_GRAY: Color = Color::from_rgb(0.75, 0.75, 0.75);

    /// Constant for the color dark gray.
    pub const DARK_GRAY: Color = Color::from_rgb(0.25, 0.25, 0.25);

    /// Creates a color with the specified components, including an alpha
    /// component. Each component should be in the range `0.0` to `1.0`.
    #[inline]
    pub const fn from_rgba(r: f32, g: f32, b: f32, a: f32) -> Self
    {
        Color { r, g, b, a }
    }

    /// Creates a color with the specified components. The alpha component will
    /// be set to 1.0 (full opacity). Each component should be in the range
    /// `0.0` to `1.0`.
    #[inline]
    pub const fn from_rgb(r: f32, g: f32, b: f32) -> Self
    {
        Color { r, g, b, a: 1.0 }
    }

    /// Creates a color with the specified components, including an alpha
    /// component. Each component should be in the range `0` to `255`.
    #[inline]
    pub fn from_int_rgba(r: u8, g: u8, b: u8, a: u8) -> Self
    {
        Color {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: a as f32 / 255.0
        }
    }

    /// Creates a color with the specified components. The alpha component will
    /// be set to 255 (full opacity). Each component should be in the range
    /// `0` to `255`.
    #[inline]
    pub fn from_int_rgb(r: u8, g: u8, b: u8) -> Self
    {
        Color {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: 1.0
        }
    }

    /// Creates a color from the specified integer value, including an alpha
    /// component.
    ///
    /// For example, the input value `0xAABBCCDD` will result in a color with:
    ///
    /// * Alpha = `0xAA`
    /// * Red   = `0xBB`
    /// * Green = `0xCC`
    /// * Blue  = `0xDD`
    ///
    /// Note: If you don't specify the alpha component, the color will be
    /// transparent.
    #[inline]
    pub fn from_hex_argb(argb: u32) -> Self
    {
        Color::from_int_rgba(
            (argb >> 16) as u8,
            (argb >> 8) as u8,
            argb as u8,
            (argb >> 24) as u8
        )
    }

    /// Creates a color from the specified integer value, with the alpha
    /// component set to `255` (full opacity).
    ///
    /// For example, the input value `0xAABBCC` will result in a color with:
    ///
    /// * Alpha = `0xFF`
    /// * Red   = `0xAA`
    /// * Green = `0xBB`
    /// * Blue  = `0xCC`
    ///
    /// Note: if an alpha component is specified in the high bits of the
    /// integer, it will be ignored. See [Color::from_hex_argb] if you wish to
    /// specify the alpha component.
    #[inline]
    pub fn from_hex_rgb(rgb: u32) -> Self
    {
        Color::from_int_rgb((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)
    }

    /// Returns the red component of the color, as a value in the range `0.0` to
    /// `1.0`.
    #[inline]
    pub const fn r(&self) -> f32
    {
        self.r
    }

    /// Returns the green component of the color, as a value in the range `0.0`
    /// to `1.0`.
    #[inline]
    pub const fn g(&self) -> f32
    {
        self.g
    }

    /// Returns the blue component of the color, as a value in the range `0.0`
    /// to `1.0`.
    #[inline]
    pub const fn b(&self) -> f32
    {
        self.b
    }

    /// Returns the alpha component of the color, as a value in the range `0.0`
    /// to `1.0`. The value `0.0` is fully transparent, and the value `1.0`
    /// is fully opaque.
    #[inline]
    pub const fn a(&self) -> f32
    {
        self.a
    }

    /// Returns the brightness of the color as perceived by a human, as a value
    /// in the range `0.0` to `1.0`.
    ///
    /// This is calculated using the following formula:
    ///
    /// ```
    /// # let red = 0.0;
    /// # let green = 0.0;
    /// # let blue = 0.0;
    /// # let result =
    /// red * 0.299 + green * 0.587 + blue * 0.114
    /// # ;
    /// ```
    pub fn subjective_brightness(&self) -> f32
    {
        self.r * 0.299 + self.g * 0.587 + self.b * 0.114
    }
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn test_from_hex()
    {
        // We're comparing floats for equality here, which is normally a bad idea, but
        // here the result should be deterministic as it's computed the same way both
        // times.

        assert_eq!(
            Color::from_hex_rgb(0xFF5511),
            Color::from_int_rgb(0xFF, 0x55, 0x11)
        );

        assert_eq!(
            Color::from_hex_argb(0xAAFF5511),
            Color::from_int_rgba(0xFF, 0x55, 0x11, 0xAA)
        );
    }
}
