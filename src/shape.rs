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

use crate::dimen::{Vec2, Vector2};
use crate::numeric::{max, min, PrimitiveZero};

/// A struct representing an axis-aligned rectangle. Two points are stored: the
/// top left vertex, and the bottom right vertex.
///
/// Alias for a rectangle with u32 coordinates.
pub type URect = Rectangle<u32>;

/// A struct representing an axis-aligned rectangle. Two points are stored: the
/// top left vertex, and the bottom right vertex.
///
/// Alias for a rectangle with i32 coordinates.
pub type IRect = Rectangle<i32>;

/// A struct representing an axis-aligned rectangle. Two points are stored: the
/// top left vertex, and the bottom right vertex.
///
/// Alias for a rectangle with f32 coordinates.
pub type Rect = Rectangle<f32>;

/// A struct representing an axis-aligned rectangle. Two points are stored: the
/// top left vertex, and the bottom right vertex.
#[derive(Debug, PartialEq, Eq, Clone)]
#[repr(C)]
pub struct Rectangle<T = f32>
{
    top_left: Vector2<T>,
    bottom_right: Vector2<T>
}

impl<T> AsRef<Rectangle<T>> for Rectangle<T>
{
    fn as_ref(&self) -> &Self
    {
        self
    }
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

    /// Returns the x value of the left border
    #[inline]
    pub fn left(&self) -> T
    {
        self.top_left.x
    }

    /// Returns the x value of the right border
    #[inline]
    pub fn right(&self) -> T
    {
        self.bottom_right.x
    }

    /// Returns the y value of the top border
    #[inline]
    pub fn top(&self) -> T
    {
        self.top_left.y
    }

    /// Returns the y value of the bottom border
    #[inline]
    pub fn bottom(&self) -> T
    {
        self.bottom_right.y
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

impl<T: PartialOrd<T> + Copy> Rectangle<T>
{
    /// Returns true if the specified point is inside this rectangle. This is
    /// inclusive of the top and left coordinates, and exclusive of the bottom
    /// and right coordinates.
    #[inline]
    #[must_use]
    pub fn contains(&self, point: Vector2<T>) -> bool
    {
        point.x >= self.top_left.x
            && point.y >= self.top_left.y
            && point.x < self.bottom_right.x
            && point.y < self.bottom_right.y
    }
}

impl<T: PartialOrd + Copy> Rectangle<T>
{
    /// Finds the intersection of two rectangles -- in other words, the area
    /// that is common to both of them.
    ///
    /// If there is no common area between the two rectangles, then this
    /// function will return `None`.
    #[inline]
    #[must_use]
    pub fn intersect(&self, other: &Self) -> Option<Self>
    {
        let result = Self {
            top_left: Vector2::new(
                max(self.top_left.x, other.top_left.x),
                max(self.top_left.y, other.top_left.y)
            ),
            bottom_right: Vector2::new(
                min(self.bottom_right.x, other.bottom_right.x),
                min(self.bottom_right.y, other.bottom_right.y)
            )
        };

        if result.is_positive_area() {
            Some(result)
        } else {
            None
        }
    }
}

impl<T: PrimitiveZero> Rectangle<T>
{
    /// A constant representing a rectangle with position (0, 0) and zero area.
    /// Each component is set to zero.
    pub const ZERO: Rectangle<T> = Rectangle::new(Vector2::ZERO, Vector2::ZERO);
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

impl<T: PartialOrd> Rectangle<T>
{
    /// Returns `true` if the rectangle has an area greater than zero.
    #[inline]
    pub fn is_positive_area(&self) -> bool
    {
        self.top_left.x < self.bottom_right.x && self.top_left.y < self.bottom_right.y
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
    pub fn with_offset(&self, offset: impl Into<Vector2<T>>) -> Self
    {
        let offset = offset.into();
        Rectangle::new(self.top_left + offset, self.bottom_right + offset)
    }
}

impl<T: Copy> Rectangle<T>
where
    Vector2<T>: std::ops::Sub<Output = Vector2<T>>
{
    /// Returns a new rectangle, whose vertices are negatively offset relative
    /// to the current rectangle by the specified amount. This is equivalent
    /// to subtracting the specified vector to each vertex.
    #[inline]
    pub fn with_negative_offset(&self, offset: impl Into<Vector2<T>>) -> Self
    {
        let offset = offset.into();
        Rectangle::new(self.top_left - offset, self.bottom_right - offset)
    }
}

impl<T> From<rusttype::Rect<T>> for Rectangle<T>
{
    fn from(rect: rusttype::Rect<T>) -> Self
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

impl<T: num_traits::AsPrimitive<f32> + Copy> Rectangle<T>
{
    /// Returns a new rectangle where the coordinates have been cast to `f32`
    /// values, using the `as` operator.
    #[inline]
    #[must_use]
    pub fn as_f32(&self) -> Rectangle<f32>
    {
        Rectangle::new(self.top_left.into_f32(), self.bottom_right.into_f32())
    }
}

/// A struct representing a polygon.
#[derive(Debug, Clone)]
pub struct Polygon
{
    pub(crate) triangles: Vec<[Vec2; 3]>
}

impl Polygon
{
    /// Generate a new polygon given points that describe it's outline.
    ///
    /// The points must be in either clockwise or couter-clockwise order.
    pub fn new<Point: Into<Vec2> + Copy>(vertices: &[Point]) -> Self
    {
        // We have to flatten the vertices in order for
        // [earcutr](https://github.com/frewsxcv/earcutr/) to accept it.
        // In the future, we can add a triangulation algorithm directly into Speed2D if
        // performance is an issue, but for now, this is simpler and easier
        let mut flattened = Vec::with_capacity(vertices.len() * 2);

        for vertex in vertices {
            let vertex: Vec2 = (*vertex).into();

            flattened.push(vertex.x);
            flattened.push(vertex.y);
        }

        let mut triangulation = earcutr::earcut(&flattened, &Vec::new(), 2);
        let mut triangles = Vec::with_capacity(triangulation.len() / 3);

        while !triangulation.is_empty() {
            triangles.push([
                vertices[triangulation.pop().unwrap()].into(),
                vertices[triangulation.pop().unwrap()].into(),
                vertices[triangulation.pop().unwrap()].into()
            ])
        }

        Polygon { triangles }
    }
}

#[cfg(test)]
mod test
{
    use crate::shape::URect;

    #[test]
    pub fn test_intersect_1()
    {
        let r1 = URect::from_tuples((100, 100), (200, 200));
        let r2 = URect::from_tuples((100, 300), (200, 400));
        let r3 = URect::from_tuples((125, 50), (175, 500));

        assert_eq!(None, r1.intersect(&r2));

        assert_eq!(
            Some(URect::from_tuples((125, 100), (175, 200))),
            r1.intersect(&r3)
        );

        assert_eq!(
            Some(URect::from_tuples((125, 300), (175, 400))),
            r2.intersect(&r3)
        );

        assert_eq!(Some(r1.clone()), r1.intersect(&r1));
        assert_eq!(Some(r2.clone()), r2.intersect(&r2));
        assert_eq!(Some(r3.clone()), r3.intersect(&r3));
    }

    #[test]
    pub fn test_intersect_2()
    {
        let r1 = URect::from_tuples((100, 100), (200, 200));
        let r2 = URect::from_tuples((100, 200), (200, 300));

        assert_eq!(None, r1.intersect(&r2));
    }
}

///////////////////////////////////

/// A struct representing an axis-aligned rounded rectangle. Two points and a float are
/// stored: the top left vertex, the bottom right vertex and the radius of the
/// rounded corners.
///
/// Alias for a rectangle with u32 coordinates.
pub type URoundRect = RoundedRectangle<u32>;

/// A struct representing an axis-aligned rounded rectangle. Two points and a float are
/// stored: the top left vertex, the bottom right vertex and the radius of the
/// rounded corners.
///
/// Alias for a rectangle with i32 coordinates.
pub type IRoundRect = RoundedRectangle<i32>;

/// A struct representing an axis-aligned rounded rectangle. Two points and a float are
/// stored: the top left vertex, the bottom right vertex and the radius of the
/// rounded corners.
///
/// Alias for a rectangle with f32 coordinates.
pub type RoundRect = RoundedRectangle<f32>;

/// A struct representing an axis-aligned rounded rectangle. Two points and a float are
/// stored: the top left vertex, the bottom right vertex and the radius of the
/// rounded corners.
#[derive(Debug, PartialEq, Eq, Clone)]
#[repr(C)]
pub struct RoundedRectangle<T = f32>
{
    top_left: Vector2<T>,
    bottom_right: Vector2<T>,
    radius: T
}

impl<T> AsRef<RoundedRectangle<T>> for RoundedRectangle<T>
{
    fn as_ref(&self) -> &Self
    {
        self
    }
}

impl<T> RoundedRectangle<T>
{
    /// Constructs a new `RoundedRectangle`. The top left vertex must be above
    /// and to the left of the bottom right vertex. A negative radius will
    /// be clamped to 0. A big radius (larger than half the width or height)
    /// might produce unexpected behavior but it won't be checked.
    #[inline]
    pub const fn new(top_left: Vector2<T>, bottom_right: Vector2<T>, radius: T) -> Self
    {
        RoundedRectangle {
            top_left,
            bottom_right,
            radius
        }
    }

    /// Constructs a new `RoundedRectangle`. The top left vertex must be above
    /// and to the left of the bottom right vertex. A negative radius will
    /// be clamped to 0. A big radius (larger than half the width or height)
    /// might produce unexpected behavior but it won't be checked.
    #[inline]
    pub fn from_tuples(top_left: (T, T), bottom_right: (T, T), radius: T) -> Self
    {
        RoundedRectangle {
            top_left: Vector2::new(top_left.0, top_left.1),
            bottom_right: Vector2::new(bottom_right.0, bottom_right.1),
            radius
        }
    }

    /// Constructs a new `RoundedRectangle` from a `Rectangle` and a radius.
    /// A big radius (larger than half the width or height) might produce
    /// unexpected behavior but it won't be checked.
    #[inline]
    pub fn from_rectangle(rectangle: Rectangle<T>, radius: T) -> Self
    {
        RoundedRectangle {
            top_left: rectangle.top_left,
            bottom_right: rectangle.bottom_right,
            radius
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

impl<T: Copy> RoundedRectangle<T>
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

    /// Returns the radius of the rounded corners.
    #[inline]
    pub fn radius(&self) -> T
    {
        self.radius
    }

    /// Returns the x value of the left border
    #[inline]
    pub fn left(&self) -> T
    {
        self.top_left.x
    }

    /// Returns the x value of the right border
    #[inline]
    pub fn right(&self) -> T
    {
        self.bottom_right.x
    }

    /// Returns the y value of the top border
    #[inline]
    pub fn top(&self) -> T
    {
        self.top_left.y
    }

    /// Returns the y value of the bottom border
    #[inline]
    pub fn bottom(&self) -> T
    {
        self.bottom_right.y
    }

    /// Returns a `Rectangle` representing the rectangle that encloses this
    /// rounded rectangle.
    #[inline]
    pub fn as_rectangle(&self) -> Rectangle<T>
    {
        Rectangle::new(self.top_left, self.bottom_right)
    }
}

impl<T: std::ops::Sub<Output = T> + Copy> RoundedRectangle<T>
{
    /// Returns the width of the rounded rectangle.
    #[inline]
    pub fn width(&self) -> T
    {
        self.bottom_right.x - self.top_left.x
    }

    /// Returns the height of the rounded rectangle.
    #[inline]
    pub fn height(&self) -> T
    {
        self.bottom_right.y - self.top_left.y
    }

    /// Returns a `Vector2` containing the width and height of the rounded
    /// rectangle.
    #[inline]
    pub fn size(&self) -> Vector2<T>
    {
        Vector2::new(self.width(), self.height())
    }
}

impl<T> RoundedRectangle<T>
where
    T: num_traits::AsPrimitive<f32>
        + std::cmp::PartialOrd
        + std::ops::Add<Output = T>
        + std::ops::Sub<Output = T>
        + std::ops::Mul<Output = T>
        + std::ops::Neg<Output = T>
        + std::ops::Div<Output = f32>
        + std::ops::Div<f32, Output = T>
{
    /// Returns true if the specified point is inside this rounded rectangle.
    /// Note: this is always inclusive, in contrast to the `contains` method
    /// of `Rect` which is sometimes exclusive.
    #[inline]
    #[must_use]
    pub fn contains(&self, point: Vector2<T>) -> bool
    {
        //if outside the enclosing rectangle then call it a win and don't proceed
        if point.x <= self.left()
            || point.x >= self.right()
            || point.y >= self.bottom()
            || point.y <= self.top()
        {
            return false;
        }

        //...by looking at the rounded rectangle as 2 rectangles in a cross and 4
        //...by circles (overlapping rectangles should be slightly better
        // than 3 rectangles in this case (I think)):
        //first rectangle:
        if point.x >= self.top_left.x
            && point.y >= self.top_left.y + self.radius
            && point.x <= self.bottom_right.x
            && point.y <= self.bottom_right.y - self.radius
        {
            return true;
        }
        //second rectangle:
        if point.x >= self.top_left.x + self.radius
            && point.y >= self.top_left.y
            && point.x <= self.bottom_right.x - self.radius
            && point.y <= self.bottom_right.y
        {
            return true;
        }

        //check if the point is inside the 4 circles on the corners by getting the
        // center of the circles and checking if the distance between the point
        // and the center is smaller than the radius
        if (self.top_left + Vector2::new(self.radius, self.radius) - point).magnitude()
            <= self.radius.as_()
        {
            return true;
        }
        if (self.top_right() + Vector2::new(-self.radius, self.radius) - point).magnitude()
            <= self.radius.as_()
        {
            return true;
        }
        if (self.bottom_left() + Vector2::new(self.radius, -self.radius) - point).magnitude()
            <= self.radius.as_()
        {
            return true;
        }
        if (self.bottom_right() + Vector2::new(-self.radius, -self.radius) - point).magnitude()
            <= self.radius.as_()
        {
            return true;
        }

        false
    }
}

impl<T: PartialEq> RoundedRectangle<T>
{
    /// Returns `true` if the rectangle containing this rounded rectangle has
    /// zero area. (the radius is not taken into account)
    #[inline]
    pub fn is_zero_area(&self) -> bool
    {
        self.top_left.x == self.bottom_right.x || self.top_left.y == self.bottom_right.y
    }
}

impl<T: PartialOrd> RoundedRectangle<T>
{
    /// Returns `true` if the rectangle containing this rounded rectangle has
    /// positive area. (the radius is not taken into account)
    #[inline]
    pub fn is_positive_area(&self) -> bool
    {
        self.top_left.x < self.bottom_right.x && self.top_left.y < self.bottom_right.y
    }
}

impl<T: Copy> RoundedRectangle<T>
where
    Vector2<T>: std::ops::Add<Output = Vector2<T>>
{
    /// Returns a new rounded rectangle, whose vertices are offset relative to
    /// the current rounded rectangle by the specified amount. This is
    /// equivalent to adding the specified vector to each vertex.
    #[inline]
    pub fn with_offset(&self, offset: impl Into<Vector2<T>>) -> Self
    {
        let offset = offset.into();
        RoundedRectangle::new(
            self.top_left + offset,
            self.bottom_right + offset,
            self.radius
        )
    }
}

impl<T: Copy> RoundedRectangle<T>
where
    Vector2<T>: std::ops::Sub<Output = Vector2<T>>
{
    /// Returns a new rounded rectangle, whose vertices are negatively offset
    /// relative to the current rectangle by the specified amount. This is
    /// equivalent to subtracting the specified vector to each vertex.
    #[inline]
    pub fn with_negative_offset(&self, offset: impl Into<Vector2<T>>) -> Self
    {
        let offset = offset.into();
        RoundedRectangle::new(
            self.top_left - offset,
            self.bottom_right - offset,
            self.radius
        )
    }
}

impl<T: num_traits::AsPrimitive<f32>> RoundedRectangle<T>
{
    /// Returns a new rounded rectangle where the coordinates and the radius
    /// have been cast to `f32` values, using the `as` operator.
    #[inline]
    #[must_use]
    pub fn into_f32(self) -> RoundedRectangle<f32>
    {
        RoundedRectangle::new(
            self.top_left.into_f32(),
            self.bottom_right.into_f32(),
            self.radius.as_()
        )
    }
}

impl<T: num_traits::AsPrimitive<f32> + Copy> RoundedRectangle<T>
{
    /// Returns a new rectangle where the coordinates have been cast to `f32`
    /// values, using the `as` operator.
    #[inline]
    #[must_use]
    pub fn as_f32(&self) -> RoundedRectangle<f32>
    {
        RoundedRectangle::new(
            self.top_left.into_f32(),
            self.bottom_right.into_f32(),
            self.radius.as_()
        )
    }
}
