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

use std::rc::Rc;

use crate::dimen::Vector2;
use crate::glwrapper::GLTexture;

/// The data type of the pixels making up the raw image data.
#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub enum ImageDataType
{
    /// Each pixel in the image is represented by three `u8` values: red, green,
    /// and blue.
    RGB,

    /// Each pixel in the image is represented by four `u8` values: red, green,
    /// blue, and alpha.
    RGBA
}

/// Represents a handle for a loaded image.
///
/// Note: this handle can only be used in the graphics context in which it was
/// created.
#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct ImageHandle
{
    pub(crate) size: Vector2<u32>,
    pub(crate) texture: Rc<GLTexture>
}

impl ImageHandle
{
    /// Returns the size of the image in pixels.
    pub fn size(&self) -> &Vector2<u32>
    {
        &self.size
    }
}

/// `ImageSmoothingMode` defines how images are rendered when the pixels of the
/// source image don't align perfectly with the pixels of the screen. This could
/// be because the image is a different size, or because it is rendered at a
/// position which is a non-integer number of pixels.
#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub enum ImageSmoothingMode
{
    /// The pixel drawn on the screen will be the closest pixel from the source
    /// image. This may cause aliasing/jagginess, so for a smoother result
    /// the `Linear` mode may be more suitable.
    NearestNeighbor,

    /// The pixel drawn on the screen will be the weighted average of the four
    /// nearest pixels in the source image. This produces a smoother result
    /// than `NearestNeighbor`, but in cases where the image is intended to
    /// be pixel-aligned it may cause unnecessary blurriness.
    Linear
}
