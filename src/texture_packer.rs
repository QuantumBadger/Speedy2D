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

use crate::dimen::Vector2;
use crate::shape::Rectangle;
use crate::texture_packer::TexturePackerError::NotEnoughSpace;

#[derive(Debug)]
struct FreeRegion
{
    rect: Rectangle<u32>
}

impl FreeRegion
{
    #[inline]
    fn from_rectangle(rect: Rectangle<u32>) -> Self
    {
        FreeRegion { rect }
    }

    #[inline]
    fn new(width: u32, height: u32) -> Self
    {
        FreeRegion::from_rectangle(Rectangle::new(
            Vector2::ZERO,
            Vector2::new(width, height)
        ))
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub(crate) enum TexturePackerError
{
    NotEnoughSpace
}

#[derive(Debug)]
pub(crate) struct TexturePacker
{
    areas: Vec<FreeRegion>
}

impl TexturePacker
{
    pub(crate) fn new(width: u32, height: u32) -> Self
    {
        TexturePacker {
            areas: vec![FreeRegion::new(width, height)]
        }
    }

    pub(crate) fn try_allocate(
        &mut self,
        size: Vector2<u32>
    ) -> Result<Rectangle<u32>, TexturePackerError>
    {
        if size.x == 0 || size.y == 0 {
            return Ok(Rectangle::new(Vector2::ZERO, size));
        }

        let width = size.x;
        let height = size.y;

        let mut best_area: Option<&mut FreeRegion> = None;

        for area in &mut self.areas {
            let area_width = area.rect.width();
            let area_height = area.rect.height();

            if width > area.rect.width() || height > area.rect.height() {
                continue;
            }

            let update_best = if let Some(current_best) = &best_area {
                current_best.rect.width() >= area_width
                    && current_best.rect.height() >= area_height
            } else {
                true
            };

            if update_best {
                best_area = Some(area);
            }
        }

        let best_area = best_area.ok_or(NotEnoughSpace)?;

        let result =
            Rectangle::new(*best_area.rect.top_left(), best_area.rect.top_left() + size);

        let space_underneath = Rectangle::new(
            Vector2::new(best_area.rect.top_left().x, result.bottom_right().y),
            *best_area.rect.bottom_right()
        );

        let space_right = Rectangle::new(
            Vector2::new(result.bottom_right().x, best_area.rect.top_left().y),
            space_underneath.top_right()
        );

        if space_right.is_zero_area() {
            best_area.rect = space_underneath
        } else {
            best_area.rect = space_right;

            if !space_underneath.is_zero_area() {
                self.areas
                    .push(FreeRegion::from_rectangle(space_underneath));
            }
        }

        Ok(result)
    }
}

#[cfg(test)]
mod test
{

    use super::*;

    #[test]
    fn pack_test_fill_four_squares()
    {
        let mut packer = TexturePacker::new(64, 64);

        assert_eq!(
            Ok(Rectangle::from_tuples((0, 0), (32, 32))),
            packer.try_allocate(Vector2::new(32, 32))
        );

        assert_eq!(
            Ok(Rectangle::from_tuples((32, 0), (64, 32))),
            packer.try_allocate(Vector2::new(32, 32))
        );

        assert_eq!(
            Ok(Rectangle::from_tuples((0, 32), (32, 64))),
            packer.try_allocate(Vector2::new(32, 32))
        );

        assert_eq!(
            Ok(Rectangle::from_tuples((32, 32), (64, 64))),
            packer.try_allocate(Vector2::new(32, 32))
        );

        assert_eq!(
            Err(NotEnoughSpace),
            packer.try_allocate(Vector2::new(32, 32))
        );
    }

    #[test]
    fn pack_test_nonfill_four_squares()
    {
        let mut packer = TexturePacker::new(64, 64);

        assert_eq!(
            Ok(Rectangle::from_tuples((0, 0), (30, 30))),
            packer.try_allocate(Vector2::new(30, 30))
        );

        assert_eq!(
            Ok(Rectangle::from_tuples((30, 0), (60, 30))),
            packer.try_allocate(Vector2::new(30, 30))
        );

        assert_eq!(
            Ok(Rectangle::from_tuples((0, 30), (30, 60))),
            packer.try_allocate(Vector2::new(30, 30))
        );

        assert_eq!(
            Ok(Rectangle::from_tuples((30, 30), (60, 60))),
            packer.try_allocate(Vector2::new(30, 30))
        );

        assert_eq!(
            Err(NotEnoughSpace),
            packer.try_allocate(Vector2::new(30, 30))
        );
    }

    #[test]
    fn pack_test_uneven_squares()
    {
        let mut packer = TexturePacker::new(64, 64);

        assert_eq!(
            Ok(Rectangle::from_tuples((0, 0), (16, 16))),
            packer.try_allocate(Vector2::new(16, 16))
        );

        assert_eq!(
            Ok(Rectangle::from_tuples((0, 16), (16, 48))),
            packer.try_allocate(Vector2::new(16, 32))
        );

        assert_eq!(
            Ok(Rectangle::from_tuples((16, 16), (48, 48))),
            packer.try_allocate(Vector2::new(32, 32))
        );

        assert_eq!(
            Ok(Rectangle::from_tuples((16, 0), (32, 16))),
            packer.try_allocate(Vector2::new(16, 16))
        );
    }
}
