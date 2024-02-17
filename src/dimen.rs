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

use std::convert::TryInto;

use rusttype::Point;

use crate::numeric::{PrimitiveZero, RoundFloat};

/// A vector with two f32 values.
pub type Vec2 = Vector2<f32>;

/// A vector with two i32 values.
pub type IVec2 = Vector2<i32>;

/// A vector with two u32 values.
pub type UVec2 = Vector2<u32>;

/// A vector containing two numeric values. This may represent a size or
/// position.
#[repr(C)]
#[derive(PartialEq, Eq, Clone, Copy, Hash, Debug, Default)]
pub struct Vector2<T>
{
    /// The horizontal component of the vector.
    pub x: T,
    /// The vertical component of the vector.
    pub y: T
}

impl<T> Vector2<T>
{
    /// Instantiates a new `Vector2` from the specified horizontal and vertical
    /// components.
    #[inline]
    #[must_use]
    pub const fn new(x: T, y: T) -> Self
    {
        Vector2 { x, y }
    }
}

impl<T: PrimitiveZero> Vector2<T>
{
    /// A constant representing a vector of zero magnitude. Each component is
    /// set to zero.
    pub const ZERO: Vector2<T> = Vector2::new(T::ZERO, T::ZERO);

    /// Instantiates a new `Vector2` from the specified horizontal component,
    /// setting the vertical component to zero.
    #[inline]
    #[must_use]
    pub fn new_x(x: T) -> Self
    {
        Vector2 { x, y: T::ZERO }
    }

    /// Instantiates a new `Vector2` from the specified vertical component,
    /// setting the horizontal component to zero.
    #[inline]
    #[must_use]
    pub fn new_y(y: T) -> Self
    {
        Vector2 { x: T::ZERO, y }
    }
}

impl Vec2
{
    /// Returns the magnitude of the vector, squared.
    #[inline]
    #[must_use]
    pub fn magnitude_squared(&self) -> f32
    {
        self.x * self.x + self.y * self.y
    }

    /// Returns the magnitude of the vector.
    #[inline]
    #[must_use]
    pub fn magnitude(&self) -> f32
    {
        self.magnitude_squared().sqrt()
    }

    /// Normalizes the vector so that the magnitude is `1.0`. If the current
    /// magnitude of the vector is `0.0`, then `None` is returned to avoid a
    /// division by zero.
    #[inline]
    #[must_use]
    pub fn normalize(&self) -> Option<Vec2>
    {
        let magnitude = self.magnitude();

        if magnitude == 0.0 {
            return None;
        }

        Some(self / magnitude)
    }
}
impl<T: std::ops::Neg<Output = T> + Copy> Vector2<T>
{
    /// Rotates the vector by 90 degrees in the clockwise direction.
    #[inline]
    #[must_use]
    pub fn rotate_90_degrees_clockwise(&self) -> Vector2<T>
    {
        Vector2::new(-self.y, self.x)
    }

    /// Rotates the vector by 90 degrees in the anti-clockwise direction.
    #[inline]
    #[must_use]
    pub fn rotate_90_degrees_anticlockwise(&self) -> Vector2<T>
    {
        Vector2::new(self.y, -self.x)
    }
}

/* ----- Casting ----- */
impl<T: num_traits::AsPrimitive<f32>> Vector2<T>
{
    /// Returns a new vector with each element cast to `f32`, using the `as`
    /// operator.
    #[inline]
    #[must_use]
    pub fn into_f32(self) -> Vec2
    {
        Vector2::new(self.x.as_(), self.y.as_())
    }
}

impl<T: num_traits::AsPrimitive<i32>> Vector2<T>
{
    /// Returns a new vector with each element cast to `i32`, using the `as`
    /// operator.
    #[inline]
    #[must_use]
    pub fn into_i32(self) -> IVec2
    {
        Vector2::new(self.x.as_(), self.y.as_())
    }
}

impl<T: num_traits::AsPrimitive<u32>> Vector2<T>
{
    /// Returns a new vector with each element cast to `u32`, using the `as`
    /// operator.
    #[inline]
    #[must_use]
    pub fn into_u32(self) -> UVec2
    {
        Vector2::new(self.x.as_(), self.y.as_())
    }
}

impl<T: TryInto<i32>> Vector2<T>
{
    /// Attempts to convert each element of this vector to an `i32`, returning
    /// an error if this fails.
    #[inline]
    pub fn try_into_i32(self) -> Result<IVec2, T::Error>
    {
        Ok(Vector2::new(self.x.try_into()?, self.y.try_into()?))
    }
}

impl<T> From<(T, T)> for Vector2<T>
where
    T: Copy
{
    #[inline]
    #[must_use]
    fn from(value: (T, T)) -> Self
    {
        Vector2::new(value.0, value.1)
    }
}

impl<T> From<&(T, T)> for Vector2<T>
where
    T: Copy
{
    #[inline]
    #[must_use]
    fn from(value: &(T, T)) -> Self
    {
        Vector2::new(value.0, value.1)
    }
}

impl<T> From<T> for Vector2<T>
where
    T: Copy
{
    #[inline]
    #[must_use]
    fn from(value: T) -> Self
    {
        Vector2::new(value, value)
    }
}

impl<T> From<&T> for Vector2<T>
where
    T: Copy
{
    #[inline]
    #[must_use]
    fn from(value: &T) -> Self
    {
        Vector2::new(*value, *value)
    }
}

impl<T> From<&Self> for Vector2<T>
where
    T: Copy
{
    #[inline]
    #[must_use]
    fn from(value: &Self) -> Self
    {
        *value
    }
}

impl<T> From<&mut Self> for Vector2<T>
where
    T: Copy
{
    #[inline]
    #[must_use]
    fn from(value: &mut Self) -> Self
    {
        *value
    }
}

/* ----- Operators ----- */
impl<T: Copy + std::ops::Add<Output = T>, R: Into<Self>> std::ops::Add<R> for Vector2<T>
{
    type Output = Vector2<T>;

    #[inline]
    #[must_use]
    fn add(self, rhs: R) -> Self::Output
    {
        let rhs = rhs.into();
        Vector2::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl<T: Copy + std::ops::Add<Output = T>, R: Into<Vector2<T>>> std::ops::Add<R>
    for &Vector2<T>
{
    type Output = Vector2<T>;

    #[inline]
    #[must_use]
    fn add(self, rhs: R) -> Self::Output
    {
        let rhs = rhs.into();
        Vector2::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl<T: Copy + std::ops::Sub<Output = T>, R: Into<Self>> std::ops::Sub<R> for Vector2<T>
{
    type Output = Vector2<T>;

    #[inline]
    #[must_use]
    fn sub(self, rhs: R) -> Self::Output
    {
        let rhs = rhs.into();
        Vector2::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl<T: Copy + std::ops::Sub<Output = T>, R: Into<Vector2<T>>> std::ops::Sub<R>
    for &Vector2<T>
{
    type Output = Vector2<T>;

    #[inline]
    #[must_use]
    fn sub(self, rhs: R) -> Self::Output
    {
        let rhs = rhs.into();
        Vector2::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl<T: Copy + std::ops::AddAssign, R: Into<Vector2<T>>> std::ops::AddAssign<R>
    for Vector2<T>
{
    #[inline]
    fn add_assign(&mut self, rhs: R)
    {
        let rhs = rhs.into();
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl<T: Copy + std::ops::AddAssign, R: Into<Vector2<T>>> std::ops::AddAssign<R>
    for &mut Vector2<T>
{
    #[inline]
    fn add_assign(&mut self, rhs: R)
    {
        let rhs = rhs.into();
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl<T: Copy + std::ops::SubAssign, R: Into<Vector2<T>>> std::ops::SubAssign<R>
    for Vector2<T>
{
    #[inline]
    fn sub_assign(&mut self, rhs: R)
    {
        let rhs = rhs.into();
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}

impl<T: Copy + std::ops::SubAssign, R: Into<Vector2<T>>> std::ops::SubAssign<R>
    for &mut Vector2<T>
{
    #[inline]
    fn sub_assign(&mut self, rhs: R)
    {
        let rhs = rhs.into();
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}

impl<T: Copy + std::ops::Mul<Output = T>> std::ops::Mul<T> for Vector2<T>
{
    type Output = Vector2<T>;

    #[inline]
    #[must_use]
    fn mul(self, rhs: T) -> Self::Output
    {
        Vector2::new(self.x * rhs, self.y * rhs)
    }
}

impl<T: Copy + std::ops::Mul<Output = T>> std::ops::Mul<T> for &Vector2<T>
{
    type Output = Vector2<T>;

    #[inline]
    #[must_use]
    fn mul(self, rhs: T) -> Self::Output
    {
        Vector2::new(self.x * rhs, self.y * rhs)
    }
}

impl<T: Copy + std::ops::Div<Output = T>> std::ops::Div<T> for Vector2<T>
{
    type Output = Vector2<T>;

    #[inline]
    #[must_use]
    fn div(self, rhs: T) -> Self::Output
    {
        Vector2::new(self.x / rhs, self.y / rhs)
    }
}

impl<T: Copy + std::ops::Div<Output = T>> std::ops::Div<T> for &Vector2<T>
{
    type Output = Vector2<T>;

    #[inline]
    #[must_use]
    fn div(self, rhs: T) -> Self::Output
    {
        Vector2::new(self.x / rhs, self.y / rhs)
    }
}

impl<T: Copy + std::ops::MulAssign> std::ops::MulAssign<T> for Vector2<T>
{
    #[inline]
    fn mul_assign(&mut self, rhs: T)
    {
        self.x *= rhs;
        self.y *= rhs;
    }
}

impl<T: Copy + std::ops::MulAssign> std::ops::MulAssign<T> for &mut Vector2<T>
{
    #[inline]
    fn mul_assign(&mut self, rhs: T)
    {
        self.x *= rhs;
        self.y *= rhs;
    }
}

impl<T: Copy + std::ops::DivAssign> std::ops::DivAssign<T> for Vector2<T>
{
    #[inline]
    fn div_assign(&mut self, rhs: T)
    {
        self.x /= rhs;
        self.y /= rhs;
    }
}

impl<T: Copy + std::ops::DivAssign> std::ops::DivAssign<T> for &mut Vector2<T>
{
    #[inline]
    fn div_assign(&mut self, rhs: T)
    {
        self.x /= rhs;
        self.y /= rhs;
    }
}

impl<T: Copy + std::ops::Mul<Output = T>> std::ops::Mul<&T> for Vector2<T>
{
    type Output = Vector2<T>;

    #[inline]
    #[must_use]
    fn mul(self, rhs: &T) -> Self::Output
    {
        Vector2::new(self.x * *rhs, self.y * *rhs)
    }
}

impl<T: Copy + std::ops::Mul<Output = T>> std::ops::Mul<&T> for &Vector2<T>
{
    type Output = Vector2<T>;

    #[inline]
    #[must_use]
    fn mul(self, rhs: &T) -> Self::Output
    {
        Vector2::new(self.x * *rhs, self.y * *rhs)
    }
}

impl<T: Copy + std::ops::Div<Output = T>> std::ops::Div<&T> for Vector2<T>
{
    type Output = Vector2<T>;

    #[inline]
    #[must_use]
    fn div(self, rhs: &T) -> Self::Output
    {
        Vector2::new(self.x / *rhs, self.y / *rhs)
    }
}

impl<T: Copy + std::ops::Div<Output = T>> std::ops::Div<&T> for &Vector2<T>
{
    type Output = Vector2<T>;

    #[inline]
    #[must_use]
    fn div(self, rhs: &T) -> Self::Output
    {
        Vector2::new(self.x / *rhs, self.y / *rhs)
    }
}

impl<T: Copy + std::ops::MulAssign> std::ops::MulAssign<&T> for Vector2<T>
{
    #[inline]
    fn mul_assign(&mut self, rhs: &T)
    {
        self.x *= *rhs;
        self.y *= *rhs;
    }
}

impl<T: Copy + std::ops::MulAssign> std::ops::MulAssign<&T> for &mut Vector2<T>
{
    #[inline]
    fn mul_assign(&mut self, rhs: &T)
    {
        self.x *= *rhs;
        self.y *= *rhs;
    }
}

impl<T: Copy + std::ops::DivAssign> std::ops::DivAssign<&T> for Vector2<T>
{
    #[inline]
    fn div_assign(&mut self, rhs: &T)
    {
        self.x /= *rhs;
        self.y /= *rhs;
    }
}

impl<T: Copy + std::ops::DivAssign> std::ops::DivAssign<&T> for &mut Vector2<T>
{
    #[inline]
    fn div_assign(&mut self, rhs: &T)
    {
        self.x /= *rhs;
        self.y /= *rhs;
    }
}

impl<T: RoundFloat> RoundFloat for Vector2<T>
{
    fn round(&self) -> Self
    {
        Vector2::new(self.x.round(), self.y.round())
    }
}

impl<T> From<Point<T>> for Vector2<T>
{
    #[inline]
    #[must_use]
    fn from(point: Point<T>) -> Self
    {
        Vector2::new(point.x, point.y)
    }
}

impl<T: Copy + std::ops::Mul<Output = T>> Vector2<T>
{
    /// Multiply two vectors componentwise
    #[inline]
    #[must_use]
    pub fn mul_components(&self, rhs: impl Into<Vector2<T>>) -> Vector2<T>
    {
        let rhs = rhs.into();
        Vector2::new(self.x * rhs.x, self.y * rhs.y)
    }
}

impl<T: Copy + std::ops::Div<Output = T>> Vector2<T>
{
    /// Divide two vectors componentwise
    #[inline]
    #[must_use]
    pub fn div_components(&self, rhs: impl Into<Vector2<T>>) -> Vector2<T>
    {
        let rhs = rhs.into();
        Vector2::new(self.x / rhs.x, self.y / rhs.y)
    }
}

#[cfg(test)]
mod test
{
    use super::*;

    #[test]
    fn test_arithmetic()
    {
        assert_eq!(
            Vector2::new(15, 20),
            Vector2::new(10, 4) + Vector2::new(5, 16)
        );

        assert_eq!(
            Vector2::new(5, -12),
            Vector2::new(10, 4) - Vector2::new(5, 16)
        );

        assert_eq!(
            Vector2::new(20, 16),
            Vector2::new(10, 4).mul_components(Vector2::new(2, 4))
        );

        assert_eq!(
            Vector2::new(5, 1),
            Vector2::new(10, 4).div_components(Vector2::new(2, 4))
        );

        assert_eq!(IVec2::new(-5, 10), IVec2::new(3, 10) - IVec2::new_x(8));

        assert_eq!(IVec2::new(-5, 17), IVec2::new(-5, 10) + IVec2::new_y(7));

        assert_eq!(
            Vec2::new(7.5, 0.0),
            Vec2::new(2.5, 1.0).mul_components(Vec2::new_x(3.0))
        );

        assert_eq!(
            Vec2::new(2.5, -1.0),
            Vec2::new(5.0, 1.0).div_components(Vec2::new(2.0, -1.0))
        );
    }

    #[test]
    #[allow(clippy::op_ref)]
    #[allow(clippy::needless_borrows_for_generic_args)]
    #[allow(clippy::needless_borrow)]
    fn test_arithmetic_ref()
    {
        assert_eq!(
            Vector2::new(15, 20),
            Vector2::new(10, 4) + &Vector2::new(5, 16)
        );

        assert_eq!(
            Vector2::new(5, -12),
            Vector2::new(10, 4) - &Vector2::new(5, 16)
        );

        assert_eq!(
            Vector2::new(15, 20),
            &Vector2::new(10, 4) + Vector2::new(5, 16)
        );

        assert_eq!(
            Vector2::new(5, -12),
            &Vector2::new(10, 4) - Vector2::new(5, 16)
        );

        assert_eq!(
            Vector2::new(15, 20),
            &Vector2::new(10, 4) + &Vector2::new(5, 16)
        );

        assert_eq!(
            Vector2::new(5, -12),
            &Vector2::new(10, 4) - &Vector2::new(5, 16)
        );

        assert_eq!(
            Vector2::new(20, 16),
            Vector2::new(10, 4).mul_components(&Vector2::new(2, 4))
        );

        assert_eq!(
            Vector2::new(5, 1),
            Vector2::new(10, 4).div_components(&Vector2::new(2, 4))
        );

        assert_eq!(
            Vector2::new(20, 16),
            (&Vector2::new(10, 4)).mul_components(Vector2::new(2, 4))
        );

        assert_eq!(
            Vector2::new(5, 1),
            (&Vector2::new(10, 4)).div_components(Vector2::new(2, 4))
        );

        assert_eq!(
            Vector2::new(20, 16),
            (&Vector2::new(10, 4)).mul_components(&Vector2::new(2, 4))
        );

        assert_eq!(
            Vector2::new(5, 1),
            (&Vector2::new(10, 4)).div_components(&Vector2::new(2, 4))
        );
    }

    #[test]
    #[allow(clippy::op_ref)]
    #[allow(clippy::needless_borrows_for_generic_args)]
    #[allow(clippy::needless_borrow)]
    fn test_arithmetic_tuples()
    {
        assert_eq!(Vector2::new(15, 20), Vector2::new(10, 4) + (5, 16));

        assert_eq!(Vector2::new(15, 20), Vector2::new(10, 4) + &(5, 16));

        assert_eq!(Vector2::new(15, 20), &Vector2::new(10, 4) + (5, 16));

        assert_eq!(Vector2::new(15, 20), &Vector2::new(10, 4) + &(5, 16));

        assert_eq!(Vector2::new(5, -12), Vector2::new(10, 4) - (5, 16));

        assert_eq!(Vector2::new(5, -12), Vector2::new(10, 4) - &(5, 16));

        assert_eq!(Vector2::new(5, -12), &Vector2::new(10, 4) - (5, 16));

        assert_eq!(Vector2::new(5, -12), &Vector2::new(10, 4) - &(5, 16));

        assert_eq!(
            Vector2::new(20, 16),
            Vector2::new(10, 4).mul_components((2, 4))
        );

        assert_eq!(
            Vector2::new(20, 16),
            Vector2::new(10, 4).mul_components(&(2, 4))
        );

        assert_eq!(
            Vector2::new(20, 16),
            (&Vector2::new(10, 4)).mul_components((2, 4))
        );

        assert_eq!(
            Vector2::new(20, 16),
            (&Vector2::new(10, 4)).mul_components(&(2, 4))
        );

        assert_eq!(
            Vector2::new(5, 1),
            Vector2::new(10, 4).div_components((2, 4))
        );

        assert_eq!(
            Vector2::new(5, 1),
            Vector2::new(10, 4).div_components(&(2, 4))
        );

        assert_eq!(
            Vector2::new(5, 1),
            (&Vector2::new(10, 4)).div_components((2, 4))
        );

        assert_eq!(
            Vector2::new(5, 1),
            (&Vector2::new(10, 4)).div_components(&(2, 4))
        );
    }

    #[test]
    #[allow(clippy::op_ref)]
    #[allow(clippy::needless_borrows_for_generic_args)]
    #[allow(clippy::needless_borrow)]
    fn test_arithmetic_values()
    {
        assert_eq!(Vector2::new(15, 20), Vector2::new(10, 15) + 5);

        assert_eq!(Vector2::new(15, 20), Vector2::new(10, 15) + &5);

        assert_eq!(Vector2::new(15, 20), &Vector2::new(10, 15) + 5);

        assert_eq!(Vector2::new(15, 20), &Vector2::new(10, 15) + &5);

        assert_eq!(Vector2::new(5, 10), Vector2::new(10, 15) - 5);

        assert_eq!(Vector2::new(5, 10), Vector2::new(10, 15) - &5);

        assert_eq!(Vector2::new(5, 10), &Vector2::new(10, 15) - 5);

        assert_eq!(Vector2::new(5, 10), &Vector2::new(10, 15) - &5);

        assert_eq!(Vector2::new(20, 30), Vector2::new(10, 15) * 2);

        assert_eq!(Vector2::new(20, 30), Vector2::new(10, 15) * &2);

        assert_eq!(Vector2::new(20, 30), &Vector2::new(10, 15) * 2);

        assert_eq!(Vector2::new(20, 30), &Vector2::new(10, 15) * &2);

        assert_eq!(Vector2::new(5, 4), Vector2::new(10, 8) / 2);

        assert_eq!(Vector2::new(5, 4), Vector2::new(10, 8) / &2);

        assert_eq!(Vector2::new(5, 4), &Vector2::new(10, 8) / 2);

        assert_eq!(Vector2::new(5, 4), &Vector2::new(10, 8) / &2);
    }

    #[test]
    fn test_add_assign()
    {
        let mut left = Vector2::new(1, 2);
        let right = Vector2::new(3, 4);
        left += right;
        assert_eq!(left, Vector2::new(4, 6));
        left += &right;
        assert_eq!(left, Vector2::new(7, 10));
        {
            let mut ref_left = &mut left;
            ref_left += right;
        }
        assert_eq!(left, Vector2::new(10, 14));
        {
            let mut ref_left = &mut left;
            ref_left += right;
        }
        assert_eq!(left, Vector2::new(13, 18));
    }

    #[test]
    fn test_sub_assign()
    {
        let mut left = Vector2::new(9, 8);
        let right = Vector2::new(1, 2);
        left -= right;
        assert_eq!(left, Vector2::new(8, 6));
        left -= &right;
        assert_eq!(left, Vector2::new(7, 4));
        {
            let mut ref_left = &mut left;
            ref_left -= right;
        }
        assert_eq!(left, Vector2::new(6, 2));
        {
            let mut ref_left = &mut left;
            ref_left -= right;
        }
        assert_eq!(left, Vector2::new(5, 0));
    }

    #[test]
    fn test_mul_assign()
    {
        let mut left = Vector2::new(2, 3);
        left *= 5;
        assert_eq!(left, Vector2::new(10, 15));
        {
            let mut ref_left = &mut left;
            ref_left *= 2;
        }
        assert_eq!(left, Vector2::new(20, 30));
    }

    #[test]
    fn test_div_assign()
    {
        let mut left = Vector2::new(12, 8);
        left /= 2;
        assert_eq!(left, Vector2::new(6, 4));
        {
            let mut ref_left = &mut left;
            ref_left /= 2;
        }
        assert_eq!(left, Vector2::new(3, 2));
    }
}
