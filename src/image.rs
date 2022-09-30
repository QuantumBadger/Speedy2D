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

use crate::dimen::UVec2;
use crate::glwrapper::GLTexture;

/// The data type of the pixels making up the raw image data.
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
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
    pub(crate) size: UVec2,
    pub(crate) texture: GLTexture
}

impl ImageHandle
{
    /// Returns the size of the image in pixels.
    pub fn size(&self) -> &UVec2
    {
        &self.size
    }
}

/// `ImageSmoothingMode` defines how images are rendered when the pixels of the
/// source image don't align perfectly with the pixels of the screen. This could
/// be because the image is a different size, or because it is rendered at a
/// position which is a non-integer number of pixels.
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
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

/// Supported image formats.
///
///  The following image formats are supported:
///
/// * `PNG`
/// * `JPEG` (baseline and progressive)
/// * `GIF`
/// * `BMP`
/// * `ICO`
/// * `TIFF`: Baseline (no fax support) + LZW + PackBits
/// * `WebP`: Lossy (luma channel only)
/// * `AVIF`: Only 8-bit
/// * `PNM`: PBM, PGM, PPM, standard PAM
/// * `DDS`: DXT1, DXT3, DXT5
/// * `TGA`
/// * `farbfeld`
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
#[allow(missing_docs)]
pub enum ImageFileFormat
{
    PNG,
    JPEG,
    GIF,
    BMP,
    ICO,
    TIFF,
    WebP,
    AVIF,
    PNM,
    DDS,
    TGA,
    Farbfeld
}

/// A type to represent some raw pixel data, with an associated width and height
/// in pixels.
#[derive(Clone)]
pub struct RawBitmapData
{
    data: Vec<u8>,
    size: UVec2,
    format: ImageDataType
}

impl RawBitmapData
{
    pub(crate) fn new(
        data: Vec<u8>,
        size: impl Into<UVec2>,
        format: ImageDataType
    ) -> Self
    {
        Self {
            data,
            size: size.into(),
            format
        }
    }

    /// Returns a reference to the raw pixel data.
    pub fn data(&self) -> &Vec<u8>
    {
        &self.data
    }

    /// Returns the width and height of this data in pixels.
    pub fn size(&self) -> UVec2
    {
        self.size
    }

    /// Returns the format of this data.
    pub fn format(&self) -> ImageDataType
    {
        self.format
    }

    /// Transfers ownership of the raw pixel data to the caller.
    pub fn into_data(self) -> Vec<u8>
    {
        self.data
    }
}
