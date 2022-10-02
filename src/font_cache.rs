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

use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use std::convert::TryInto;
use std::fmt::{Display, Formatter};
use std::ops::Div;
use std::rc::Rc;

use crate::color::Color;
use crate::dimen::{IVec2, UVec2, Vec2};
use crate::error::{BacktraceError, Context, ErrorMessage};
use crate::font;
use crate::glwrapper::{
    GLContextManager,
    GLTexture,
    GLTextureImageFormatU8,
    GLTextureSmoothing
};
use crate::numeric::RoundFloat;
use crate::renderer2d::{Renderer2DAction, Renderer2DVertex};
use crate::shape::Rectangle;
use crate::texture_packer::{TexturePacker, TexturePackerError};

// TODO unnecessary?
#[repr(transparent)]
#[derive(Debug, Hash, Eq, PartialEq, Clone)]
struct QuantizedDimension
{
    /// The number of pixels, multiplied by 10
    inner_value: i32
}

impl QuantizedDimension
{
    fn from_pixels(pixels: f32) -> Self
    {
        QuantizedDimension {
            // Round to nearest
            inner_value: ((10.0 * pixels) + 0.5) as i32
        }
    }

    fn to_pixels(&self) -> f32
    {
        (self.inner_value as f32) / 10.0
    }
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
struct GlyphCacheKey
{
    font_id: usize,

    /// Value between -0.5 and 0.5
    subpixel_offset: (QuantizedDimension, QuantizedDimension),

    scale: QuantizedDimension,
    glyph_id: rusttype::GlyphId
}

impl GlyphCacheKey
{
    #[inline]
    fn from(
        font_id: usize,
        positioned_glyph: &rusttype::PositionedGlyph,
        screen_offset: Vec2
    ) -> Self
    {
        // Assuming scale is uniform
        let scale = QuantizedDimension::from_pixels(positioned_glyph.scale().y);

        let pos = Vec2::new(
            positioned_glyph.position().x + screen_offset.x,
            positioned_glyph.position().y + screen_offset.y
        );

        let subpixel_offset = (
            QuantizedDimension::from_pixels(pos.x - pos.x.round()),
            QuantizedDimension::from_pixels(pos.y - pos.y.round())
        );

        GlyphCacheKey {
            font_id,
            subpixel_offset,
            scale,
            glyph_id: positioned_glyph.id()
        }
    }
}

pub(crate) struct GlyphCache
{
    last_frame: HashSet<GlyphCacheKey>,
    this_frame: HashSet<GlyphCacheKey>,

    cache_entries: HashMap<GlyphCacheKey, GlyphCacheEntry>,
    textures: Vec<GlyphCacheTexture>
}

impl GlyphCache
{
    #[inline]
    pub(crate) fn get_renderer2d_actions(
        &self,
        glyph: &font::FormattedGlyph,
        position: Vec2,
        color: Color,
        runner: &mut impl FnMut(Renderer2DAction)
    )
    {
        let positioned_glyph = glyph.glyph();

        let key = GlyphCacheKey::from(glyph.font_id(), positioned_glyph, position);

        let entry = match self.cache_entries.get(&key) {
            None => return, // This is valid for many glyphs, e.g. space
            Some(entry) => entry
        };

        let texture_cache = self.textures.get(entry.texture_id.unwrap()).unwrap();

        let texture_entry = texture_cache.entries.get(&key).unwrap();

        let texture_size = GlyphCacheTexture::SIZE as f32;

        let texture_region = Rectangle::new(
            texture_entry
                .texture_area
                .top_left()
                .into_f32()
                .div(texture_size),
            texture_entry
                .texture_area
                .bottom_right()
                .into_f32()
                .div(texture_size)
        );

        let position = position + Vec2::from(positioned_glyph.position());

        // We round the position here as the offset is between -0.5 and 0.5
        let screen_region_start = position.round().into_i32() + entry.bounding_box_offset;

        let screen_region = Rectangle::new(
            screen_region_start,
            screen_region_start + texture_entry.texture_area.size().into_i32()
        )
        .into_f32();

        runner(Renderer2DAction {
            texture: Some(texture_cache.texture.clone()),
            vertices_clockwise: [
                Renderer2DVertex {
                    position: *screen_region.top_left(),
                    texture_coord: *texture_region.top_left(),
                    color,
                    texture_mix: 1.0,
                    circle_mix: 0.0
                },
                Renderer2DVertex {
                    position: screen_region.top_right(),
                    texture_coord: texture_region.top_right(),
                    color,
                    texture_mix: 1.0,
                    circle_mix: 0.0
                },
                Renderer2DVertex {
                    position: *screen_region.bottom_right(),
                    texture_coord: *texture_region.bottom_right(),
                    color,
                    texture_mix: 1.0,
                    circle_mix: 0.0
                }
            ]
        });

        runner(Renderer2DAction {
            texture: Some(texture_cache.texture.clone()),
            vertices_clockwise: [
                Renderer2DVertex {
                    position: *screen_region.bottom_right(),
                    texture_coord: *texture_region.bottom_right(),
                    color,
                    texture_mix: 1.0,
                    circle_mix: 0.0
                },
                Renderer2DVertex {
                    position: screen_region.bottom_left(),
                    texture_coord: texture_region.bottom_left(),
                    color,
                    texture_mix: 1.0,
                    circle_mix: 0.0
                },
                Renderer2DVertex {
                    position: *screen_region.top_left(),
                    texture_coord: *texture_region.top_left(),
                    color,
                    texture_mix: 1.0,
                    circle_mix: 0.0
                }
            ]
        });
    }

    pub(crate) fn add_to_cache(
        &mut self,
        _context: &GLContextManager,
        formatted_glyph: &font::FormattedGlyph,
        position: Vec2
    )
    {
        let key = GlyphCacheKey::from(
            formatted_glyph.font_id(),
            formatted_glyph.glyph(),
            position
        );

        self.this_frame.insert(key.clone());

        let cache_entries = &mut self.cache_entries;

        match cache_entries.entry(key.clone()) {
            Entry::Occupied(_) => {
                // Already in the cache, nothing to do
            }

            Entry::Vacant(entry) => {
                let glyph = formatted_glyph
                    .glyph()
                    .unpositioned()
                    .unscaled()
                    .clone()
                    .scaled(rusttype::Scale::uniform(key.scale.to_pixels()))
                    .positioned(rusttype::point(
                        key.subpixel_offset.0.to_pixels(),
                        key.subpixel_offset.1.to_pixels()
                    ));

                let bounding_box = match glyph.pixel_bounding_box() {
                    None => return, // This is valid for some glyphs, e.g. space
                    Some(bounding_box) => bounding_box
                };

                let bounding_box_size =
                    UVec2::new(bounding_box.width() as u32, bounding_box.height() as u32);

                if bounding_box_size.x > GlyphCacheTexture::SIZE
                    || bounding_box_size.y > GlyphCacheTexture::SIZE
                {
                    log::error!(
                        "Glyph too big to render ({}x{}). Limit is {} px.",
                        bounding_box_size.x,
                        bounding_box_size.y,
                        GlyphCacheTexture::SIZE
                    );

                    return;
                }

                let mut bitmap = BitmapRGBA::new(bounding_box_size);

                bitmap.draw_glyph(&glyph);

                entry.insert(GlyphCacheEntry {
                    glyph_bitmap: Rc::new(bitmap),
                    bounding_box_offset: IVec2::new(
                        bounding_box.min.x,
                        bounding_box.min.y
                    ),
                    texture_id: None
                });
            }
        }
    }

    pub(crate) fn on_new_frame_start(&mut self)
    {
        self.last_frame.clear();
        std::mem::swap(&mut self.last_frame, &mut self.this_frame);
    }

    pub(crate) fn prepare_for_draw(
        &mut self,
        context: &GLContextManager
    ) -> Result<(), BacktraceError<ErrorMessage>>
    {
        if self.try_insert_pending().is_err() {
            // Not enough space. Rearrange everything!

            self.textures.iter_mut().for_each(|texture| texture.clear());

            let cache_entries = &mut self.cache_entries;
            let last_frame = &self.last_frame;
            let this_frame = &self.this_frame;

            cache_entries
                .iter_mut()
                .for_each(|(_, entry)| entry.texture_id = None);

            cache_entries
                .retain(|key, _| last_frame.contains(key) || this_frame.contains(key));

            // Sort entries by height

            let mut all_entries: Vec<_> = cache_entries.iter_mut().collect();

            all_entries.sort_unstable_by(|(_, a), (_, b)| {
                b.glyph_bitmap.size.y.cmp(&a.glyph_bitmap.size.y)
            });

            // Insert in height order

            let mut cleared_textures = Vec::new();
            std::mem::swap(&mut self.textures, &mut cleared_textures);

            cleared_textures
                .iter_mut()
                .for_each(|texture| texture.clear());

            for (key, entry) in &mut all_entries {
                let texture_id = GlyphCache::internal_rearrange_append_glyph(
                    context,
                    &mut self.textures,
                    &mut cleared_textures,
                    key,
                    &entry.glyph_bitmap
                )
                .map_err(|err| {
                    ErrorMessage::msg_with_cause("Glyph rearrangement failed", err)
                })?;

                entry.texture_id = Some(texture_id);
            }

            // Delete all but one spare texture
            if let Some(texture) = cleared_textures.pop() {
                self.textures.push(texture);
            }
        }

        for texture in &mut self.textures {
            texture
                .revalidate(context)
                .map_err(|err| err.context("Failed to revalidate texture"))?;
        }

        Ok(())
    }

    pub(crate) fn new() -> Self
    {
        Self {
            last_frame: HashSet::new(),
            this_frame: HashSet::new(),
            cache_entries: HashMap::new(),
            textures: Vec::new()
        }
    }

    fn try_insert_pending(&mut self) -> Result<(), GlyphCacheTextureAppendError>
    {
        for (key, entry) in &mut self.cache_entries {
            if entry.texture_id == None {
                let texture_id = Self::try_append_to_existing_texture(
                    &mut self.textures,
                    key,
                    &entry.glyph_bitmap
                )?;

                entry.texture_id = Some(texture_id);
            }
        }

        Ok(())
    }

    fn try_append_to_existing_texture(
        all_textures: &mut [GlyphCacheTexture],
        key: &GlyphCacheKey,
        glyph_bitmap: &Rc<BitmapRGBA>
    ) -> Result<usize, GlyphCacheTextureAppendError>
    {
        let mut last_error: GlyphCacheTextureAppendError =
            GlyphCacheTextureAppendError::NotEnoughSpace;

        for (i, texture) in all_textures.iter_mut().enumerate() {
            match texture.try_append_glyph(key, glyph_bitmap) {
                Ok(_) => return Ok(i),
                Err(err) => last_error = err
            }
        }

        Err(last_error)
    }

    fn internal_rearrange_append_glyph(
        context: &GLContextManager,
        current_textures: &mut Vec<GlyphCacheTexture>,
        previous_textures: &mut Vec<GlyphCacheTexture>,
        key: &GlyphCacheKey,
        glyph_bitmap: &Rc<BitmapRGBA>
    ) -> Result<usize, BacktraceError<ErrorMessage>>
    {
        for (i, texture) in current_textures.iter_mut().enumerate() {
            if texture.try_append_glyph(key, glyph_bitmap).is_ok() {
                return Ok(i);
            }
        }

        if !previous_textures.is_empty() {
            current_textures.push(previous_textures.pop().unwrap());

            if current_textures
                .last_mut()
                .unwrap()
                .try_append_glyph(key, glyph_bitmap)
                .is_ok()
            {
                return Ok(current_textures.len() - 1);
            }
        }

        log::info!(
            "No more space in existing textures ({}). Creating new.",
            current_textures.len()
        );

        current_textures.push(match GlyphCacheTexture::new(context) {
            Ok(texture) => texture,
            Err(err) => {
                return Err(ErrorMessage::msg_with_cause(
                    "Failed to create new texture",
                    err
                ))
            }
        });

        match current_textures
            .last_mut()
            .unwrap()
            .try_append_glyph(key, glyph_bitmap)
        {
            Ok(_) => Ok(current_textures.len() - 1),
            Err(err) => Err(ErrorMessage::msg_with_cause(
                "Internal bug: Could not append to new texture",
                err
            ))
        }
    }
}

struct BitmapRGBA
{
    data: Vec<u8>,
    size: UVec2
}

impl BitmapRGBA
{
    #[inline]
    fn new(size: UVec2) -> Self
    {
        let data = vec![0; (size.x * size.y * 4).try_into().unwrap()];
        BitmapRGBA { data, size }
    }

    fn clear(&mut self)
    {
        self.data.fill(0);
    }

    #[inline]
    fn draw_glyph(&mut self, glyph: &rusttype::PositionedGlyph)
    {
        glyph.draw(|x, y, alpha| {
            let start = (4 * (self.size.x * y + x)) as usize;
            self.data[start] = 255;
            self.data[start + 1] = 255;
            self.data[start + 2] = 255;
            self.data[start + 3] = (alpha * 255.0).round() as u8;
        })
    }

    #[inline]
    fn draw_bitmap_at(&mut self, bitmap: &Self, position: &UVec2)
    {
        let src_w_px: usize = bitmap.size.x.try_into().unwrap();
        let dest_w_px: usize = self.size.x.try_into().unwrap();

        let pos_x: usize = position.x.try_into().unwrap();
        let pos_y: usize = position.y.try_into().unwrap();

        let line_size_bytes: usize = src_w_px * 4;
        let dest_line_stride_bytes: usize = dest_w_px * 4;

        let mut src_pos_bytes: usize = 0;
        let mut dest_pos_bytes: usize = pos_y * dest_line_stride_bytes + pos_x * 4;

        while src_pos_bytes < bitmap.data.len() {
            assert!(bitmap.data.len() >= src_pos_bytes + line_size_bytes);
            assert!(self.data.len() >= dest_pos_bytes + line_size_bytes);

            // As much as I hate to use unsafe here, this more than doubles performance
            // with large glyphs in debug builds, compared to using
            // clone_from_slice.
            unsafe {
                std::ptr::copy_nonoverlapping(
                    bitmap.data.as_ptr().add(src_pos_bytes),
                    self.data.as_mut_ptr().add(dest_pos_bytes),
                    line_size_bytes
                );
            }

            src_pos_bytes += line_size_bytes;
            dest_pos_bytes += dest_line_stride_bytes;
        }
    }

    fn upload_to_texture(
        &self,
        context: &GLContextManager,
        texture: &GLTexture
    ) -> Result<(), BacktraceError<ErrorMessage>>
    {
        texture.set_image_data(
            context,
            GLTextureImageFormatU8::RGBA,
            GLTextureSmoothing::NearestNeighbour,
            &self.size,
            self.data.as_slice()
        )
    }
}

#[derive(Clone)]
struct GlyphCacheEntry
{
    glyph_bitmap: Rc<BitmapRGBA>,
    bounding_box_offset: IVec2,
    texture_id: Option<usize>
}

struct GlyphTextureCacheEntry
{
    texture_area: Rectangle<u32>
}

struct GlyphCacheTexture
{
    bitmap: BitmapRGBA,
    texture: GLTexture,
    invalidated: bool,

    packer: TexturePacker,

    entries: HashMap<GlyphCacheKey, GlyphTextureCacheEntry>
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub(crate) enum GlyphCacheTextureAppendError
{
    NotEnoughSpace
}

impl Display for GlyphCacheTextureAppendError
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result
    {
        match self {
            GlyphCacheTextureAppendError::NotEnoughSpace => {
                f.write_str("Not enough space")
            }
        }
    }
}

impl std::error::Error for GlyphCacheTextureAppendError {}

impl From<TexturePackerError> for GlyphCacheTextureAppendError
{
    fn from(value: TexturePackerError) -> Self
    {
        match value {
            TexturePackerError::NotEnoughSpace => {
                GlyphCacheTextureAppendError::NotEnoughSpace
            }
        }
    }
}

impl GlyphCacheTexture
{
    const SIZE: u32 = 1024;

    fn new(context: &GLContextManager) -> Result<Self, BacktraceError<ErrorMessage>>
    {
        Ok(GlyphCacheTexture {
            bitmap: BitmapRGBA::new(UVec2::new(
                GlyphCacheTexture::SIZE,
                GlyphCacheTexture::SIZE
            )),

            texture: context
                .new_texture()
                .context("GPU texture creation failed")?,

            invalidated: false,

            packer: TexturePacker::new(GlyphCacheTexture::SIZE, GlyphCacheTexture::SIZE),

            entries: HashMap::new()
        })
    }

    fn clear(&mut self)
    {
        self.invalidated = false;

        self.packer =
            TexturePacker::new(GlyphCacheTexture::SIZE, GlyphCacheTexture::SIZE);

        self.entries.clear();

        self.bitmap.clear();
    }

    fn try_append_glyph(
        &mut self,
        key: &GlyphCacheKey,
        glyph_bitmap: &Rc<BitmapRGBA>
    ) -> Result<(), GlyphCacheTextureAppendError>
    {
        let texture_area = self.packer.try_allocate(glyph_bitmap.size)?;

        self.bitmap
            .draw_bitmap_at(glyph_bitmap, texture_area.top_left());

        self.entries
            .insert(key.clone(), GlyphTextureCacheEntry { texture_area });

        self.invalidated = true;

        Ok(())
    }

    fn revalidate(
        &mut self,
        context: &GLContextManager
    ) -> Result<(), BacktraceError<ErrorMessage>>
    {
        if self.invalidated {
            self.invalidated = false;
            self.bitmap.upload_to_texture(context, &self.texture)
        } else {
            Ok(())
        }
    }
}
