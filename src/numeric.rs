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

/// A trait defined for primitives which have a zero value.
pub trait PrimitiveZero {
    /// The number zero.
    const ZERO: Self;
}

impl PrimitiveZero for i8 {
    const ZERO: Self = 0;
}
impl PrimitiveZero for i16 {
    const ZERO: Self = 0;
}
impl PrimitiveZero for i32 {
    const ZERO: Self = 0;
}
impl PrimitiveZero for i64 {
    const ZERO: Self = 0;
}
impl PrimitiveZero for i128 {
    const ZERO: Self = 0;
}
impl PrimitiveZero for isize {
    const ZERO: Self = 0;
}

impl PrimitiveZero for u8 {
    const ZERO: Self = 0;
}
impl PrimitiveZero for u16 {
    const ZERO: Self = 0;
}
impl PrimitiveZero for u32 {
    const ZERO: Self = 0;
}
impl PrimitiveZero for u64 {
    const ZERO: Self = 0;
}
impl PrimitiveZero for u128 {
    const ZERO: Self = 0;
}
impl PrimitiveZero for usize {
    const ZERO: Self = 0;
}

impl PrimitiveZero for f32 {
    const ZERO: Self = 0.0;
}
impl PrimitiveZero for f64 {
    const ZERO: Self = 0.0;
}

/// Types implementing this trait can be rounded to the nearest integer value.
/// In the case of vectors or other types containing multiple elements, each
/// element will be individually rounded.
pub trait RoundFloat {
    /// Round this value to the nearest integer. In the case of vectors or other
    /// types containing multiple elements, each element will be
    /// individually rounded.
    fn round(&self) -> Self;
}

impl RoundFloat for f32 {
    #[inline]
    fn round(&self) -> Self {
        f32::round(*self)
    }
}

impl RoundFloat for f64 {
    #[inline]
    fn round(&self) -> Self {
        f64::round(*self)
    }
}

pub(crate) fn min<T: PartialOrd + Copy>(a: T, b: T) -> T {
    if a < b {
        a
    } else {
        b
    }
}

pub(crate) fn max<T: PartialOrd + Copy>(a: T, b: T) -> T {
    if a > b {
        a
    } else {
        b
    }
}
