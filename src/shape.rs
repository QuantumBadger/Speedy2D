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

use rusttype::Rect;

use crate::dimen::Vector2;

/// A struct representing an axis-aligned rectangle. Two points are stored: the
/// top left vertex, and the bottom right vertex.
#[derive(Debug, PartialEq, Clone)]
#[repr(C)]
pub struct Rectangle<T = f32>
{
    top_left: Vector2<T>,
    bottom_right: Vector2<T>
}

impl<T> Rectangle<T>
{
    /// Constructs a new `Rectangle`. The top left vertex must be above and to
    /// the left of the bottom right vertex.
    #[inline]
    pub const fn new(top_left: Vector2<T>, bottom_right: Vector2<T>) -> Self
    {
        Rectangle {
            top_left,
            bottom_right
        }
    }

    /// Constructs a new `Rectangle`. The top left vertex must be above and to
    /// the left of the bottom right vertex.
    #[inline]
    pub fn from_tuples(top_left: (T, T), bottom_right: (T, T)) -> Self
    {
        Rectangle {
            top_left: Vector2::new(top_left.0, top_left.1),
            bottom_right: Vector2::new(bottom_right.0, bottom_right.1)
        }
    }

    /// Returns a reference to the top left vertex.
    #[inline]
    pub const fn top_left(&self) -> &Vector2<T>
    {
        &self.top_left
    }

    /// Returns a reference to the bottom right vertex.
    #[inline]
    pub const fn bottom_right(&self) -> &Vector2<T>
    {
        &self.bottom_right
    }
}

impl<T: Copy> Rectangle<T>
{
    /// Returns a vector representing the top right vertex.
    #[inline]
    pub fn top_right(&self) -> Vector2<T>
    {
        Vector2::new(self.bottom_right.x, self.top_left.y)
    }

    /// Returns a vector representing the bottom left vertex.
    #[inline]
    pub fn bottom_left(&self) -> Vector2<T>
    {
        Vector2::new(self.top_left.x, self.bottom_right.y)
    }
}

impl<T: std::ops::Sub<Output = T> + Copy> Rectangle<T>
{
    /// Returns the width of the rectangle.
    #[inline]
    pub fn width(&self) -> T
    {
        self.bottom_right.x - self.top_left.x
    }

    /// Returns the height of the rectangle.
    #[inline]
    pub fn height(&self) -> T
    {
        self.bottom_right.y - self.top_left.y
    }

    /// Returns a `Vector2` containing the width and height of the rectangle.
    #[inline]
    pub fn size(&self) -> Vector2<T>
    {
        Vector2::new(self.width(), self.height())
    }
}

impl<T: PartialEq> Rectangle<T>
{
    /// Returns `true` if the rectangle has zero area.
    #[inline]
    pub fn is_zero_area(&self) -> bool
    {
        self.top_left.x == self.bottom_right.x || self.top_left.y == self.bottom_right.y
    }
}

impl<T: Copy> Rectangle<T>
where
    Vector2<T>: std::ops::Add<Output = Vector2<T>>
{
    /// Returns a new rectangle, whose vertices are offset relative to the
    /// current rectangle by the specified amount. This is equivalent to
    /// adding the specified vector to each vertex.
    #[inline]
    pub fn with_offset(&self, offset: Vector2<T>) -> Self
    {
        Rectangle::new(self.top_left + offset, self.bottom_right + offset)
    }
}

impl<T> From<rusttype::Rect<T>> for Rectangle<T>
{
    fn from(rect: Rect<T>) -> Self
    {
        Rectangle::new(Vector2::from(rect.min), Vector2::from(rect.max))
    }
}

impl<T: num_traits::AsPrimitive<f32>> Rectangle<T>
{
    /// Returns a new rectangle where the coordinates have been cast to `f32`
    /// values, using the `as` operator.
    #[inline]
    #[must_use]
    pub fn into_f32(self) -> Rectangle<f32>
    {
        Rectangle::new(self.top_left.into_f32(), self.bottom_right.into_f32())
    }
}
