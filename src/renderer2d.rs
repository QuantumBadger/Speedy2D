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

#[cfg(any(feature = "image-loading", doc, doctest))]
use {
    crate::image::ImageFileFormat,
    image::GenericImageView,
    std::fs::File,
    std::io::{BufRead, BufReader, Seek},
    std::path::Path
};

use crate::color::Color;
use crate::dimen::Vector2;
use crate::error::{BacktraceError, Context, ErrorMessage};
use crate::font::FormattedTextBlock;
use crate::font_cache::{GlyphCache, GlyphCacheInterface};
use crate::glwrapper::*;
use crate::image::{ImageDataType, ImageHandle, ImageSmoothingMode};

struct AttributeBuffers
{
    position: Vec<f32>,
    color: Vec<f32>,
    texture_coord: Vec<f32>,
    texture_mix: Vec<f32>,
    circle_mix: Vec<f32>,

    glbuf_position: GLBuffer,
    glbuf_color: GLBuffer,
    glbuf_texture_coord: GLBuffer,
    glbuf_texture_mix: GLBuffer,
    glbuf_circle_mix: GLBuffer
}

impl AttributeBuffers
{
    pub fn new(
        context: &GLContextManager,
        program: &GLProgram
    ) -> Result<Self, BacktraceError<ErrorMessage>>
    {
        Ok(AttributeBuffers {
            position: Vec::new(),
            color: Vec::new(),
            texture_coord: Vec::new(),
            texture_mix: Vec::new(),
            circle_mix: Vec::new(),

            glbuf_position: context
                .new_buffer(
                    GLBufferTarget::Array,
                    2,
                    program
                        .get_attribute_handle(Renderer2D::ATTR_NAME_POSITION)
                        .context("Failed to get attribute POSITION")?
                )
                .context("Failed to create buffer for attribute POSITION")?,

            glbuf_color: context
                .new_buffer(
                    GLBufferTarget::Array,
                    4,
                    program
                        .get_attribute_handle(Renderer2D::ATTR_NAME_COLOR)
                        .context("Failed to get attribute COLOR")?
                )
                .context("Failed to create buffer for attribute COLOR")?,

            glbuf_texture_coord: context
                .new_buffer(
                    GLBufferTarget::Array,
                    2,
                    program
                        .get_attribute_handle(Renderer2D::ATTR_NAME_TEXTURE_COORD)
                        .context("Failed to get attribute TEXTURE_COORD")?
                )
                .context("Failed to create buffer for attribute TEXTURE_COORD")?,

            glbuf_texture_mix: context
                .new_buffer(
                    GLBufferTarget::Array,
                    1,
                    program
                        .get_attribute_handle(Renderer2D::ATTR_NAME_TEXTURE_MIX)
                        .context("Failed to get attribute TEXTURE_MIX")?
                )
                .context("Failed to create buffer for attribute TEXTURE_MIX")?,

            glbuf_circle_mix: context
                .new_buffer(
                    GLBufferTarget::Array,
                    1,
                    program
                        .get_attribute_handle(Renderer2D::ATTR_NAME_CIRCLE_MIX)
                        .context("Failed to get attribute CIRCLE_MIX")?
                )
                .context("Failed to create buffer for attribute CIRCLE_MIX")?
        })
    }

    #[inline]
    pub fn get_vertex_count(&self) -> usize
    {
        self.texture_mix.len()
    }

    pub fn upload_and_clear(&mut self)
    {
        self.glbuf_position.set_data(&self.position);
        self.glbuf_color.set_data(&self.color);
        self.glbuf_texture_coord.set_data(&self.texture_coord);
        self.glbuf_texture_mix.set_data(&self.texture_mix);
        self.glbuf_circle_mix.set_data(&self.circle_mix);
        self.clear();
    }

    pub fn clear(&mut self)
    {
        self.position.clear();
        self.color.clear();
        self.texture_coord.clear();
        self.texture_mix.clear();
        self.circle_mix.clear();
    }

    #[inline]
    pub fn append(
        &mut self,
        position: &Vector2<f32>,
        color: &Color,
        texture_coord: &Vector2<f32>,
        texture_mix: f32,
        circle_mix: f32
    )
    {
        AttributeBuffers::push_vec2(&mut self.position, position);
        AttributeBuffers::push_color(&mut self.color, color);
        AttributeBuffers::push_vec2(&mut self.texture_coord, texture_coord);
        self.texture_mix.push(texture_mix);
        self.circle_mix.push(circle_mix);
    }

    #[inline]
    fn push_vec2(dest: &mut Vec<f32>, vertices: &Vector2<f32>)
    {
        dest.push(vertices.x);
        dest.push(vertices.y);
    }

    #[inline]
    fn push_color(dest: &mut Vec<f32>, color: &Color)
    {
        dest.push(color.r());
        dest.push(color.g());
        dest.push(color.b());
        dest.push(color.a());
    }
}

struct Uniforms
{
    scale_x: GLUniformHandle,
    scale_y: GLUniformHandle,
    texture: GLUniformHandle
}

impl Uniforms
{
    fn new(program: &Rc<GLProgram>) -> Result<Uniforms, BacktraceError<ErrorMessage>>
    {
        Ok(Uniforms {
            scale_x: program
                .get_uniform_handle(Renderer2D::UNIFORM_NAME_SCALE_X)
                .context("Failed to find SCALE_X uniform")?,
            scale_y: program
                .get_uniform_handle(Renderer2D::UNIFORM_NAME_SCALE_Y)
                .context("Failed to find SCALE_Y uniform")?,
            texture: program
                .get_uniform_handle(Renderer2D::UNIFORM_NAME_TEXTURE)
                .context("Failed to find TEXTURE uniform")?
        })
    }

    fn set_viewport_size_pixels(&self, viewport_size_pixels: Vector2<u32>)
    {
        self.scale_x
            .set_value_float(2.0 / viewport_size_pixels.x as f32);
        self.scale_y
            .set_value_float(-2.0 / viewport_size_pixels.y as f32);
    }

    fn set_texture_unit(&self, texture_unit: i32)
    {
        self.texture.set_value_int(texture_unit);
    }
}

pub(crate) struct Renderer2DVertex
{
    pub position: Vector2<f32>,
    pub texture_coord: Vector2<f32>,
    pub color: Color,
    pub texture_mix: f32,
    pub circle_mix: f32
}

impl Renderer2DVertex
{
    #[inline]
    fn append_to_attribute_buffers(&self, attribute_buffers: &mut AttributeBuffers)
    {
        attribute_buffers.append(
            &self.position,
            &self.color,
            &self.texture_coord,
            self.texture_mix,
            self.circle_mix
        );
    }
}

pub(crate) struct Renderer2DAction
{
    pub texture: Option<Rc<GLTexture>>,
    pub vertices_clockwise: [Renderer2DVertex; 3]
}

impl Renderer2DAction
{
    #[inline]
    fn update_current_texture_if_empty(
        &self,
        current_texture: &mut Option<Rc<GLTexture>>
    ) -> bool
    {
        match &self.texture {
            None => true,

            Some(own_texture) => match current_texture {
                None => {
                    *current_texture = Some(own_texture.clone());
                    true
                }
                Some(current_texture) => *current_texture == *own_texture
            }
        }
    }

    #[inline]
    fn append_to_attribute_buffers(&self, attribute_buffers: &mut AttributeBuffers)
    {
        for vertex in self.vertices_clockwise.iter() {
            vertex.append_to_attribute_buffers(attribute_buffers);
        }
    }
}

enum RenderQueueItem
{
    FormattedTextBlock
    {
        position: Vector2<f32>,
        color: Color,
        block: Rc<crate::font::FormattedTextBlock>
    },

    CircleSectionColored
    {
        vertex_positions_clockwise: [Vector2<f32>; 3],
        vertex_colors_clockwise: [Color; 3],
        vertex_normalized_circle_coords_clockwise: [Vector2<f32>; 3]
    },

    TriangleColored
    {
        vertex_positions_clockwise: [Vector2<f32>; 3],
        vertex_colors_clockwise: [Color; 3]
    },

    TriangleTextured
    {
        vertex_positions_clockwise: [Vector2<f32>; 3],
        vertex_colors_clockwise: [Color; 3],
        vertex_texture_coords_clockwise: [Vector2<f32>; 3],
        texture: Rc<GLTexture>
    }
}

impl RenderQueueItem
{
    #[inline]
    fn generate_actions<T: GlyphCacheInterface>(
        &self,
        output: &mut Vec<Renderer2DAction>,
        glyph_cache: &T
    )
    {
        match self {
            RenderQueueItem::FormattedTextBlock {
                position,
                color,
                block
            } => {
                for line in block.iter_lines() {
                    for glyph in line.iter_glyphs() {
                        glyph_cache
                            .get_renderer2d_actions(glyph, *position, *color, output);
                    }
                }
            }

            RenderQueueItem::CircleSectionColored {
                vertex_positions_clockwise,
                vertex_colors_clockwise,
                vertex_normalized_circle_coords_clockwise
            } => output.push(Renderer2DAction {
                texture: None,
                vertices_clockwise: [
                    Renderer2DVertex {
                        position: vertex_positions_clockwise[0],
                        texture_coord: vertex_normalized_circle_coords_clockwise[0],
                        color: vertex_colors_clockwise[0],
                        texture_mix: 0.0,
                        circle_mix: 1.0
                    },
                    Renderer2DVertex {
                        position: vertex_positions_clockwise[1],
                        texture_coord: vertex_normalized_circle_coords_clockwise[1],
                        color: vertex_colors_clockwise[1],
                        texture_mix: 0.0,
                        circle_mix: 1.0
                    },
                    Renderer2DVertex {
                        position: vertex_positions_clockwise[2],
                        texture_coord: vertex_normalized_circle_coords_clockwise[2],
                        color: vertex_colors_clockwise[2],
                        texture_mix: 0.0,
                        circle_mix: 1.0
                    }
                ]
            }),

            RenderQueueItem::TriangleColored {
                vertex_positions_clockwise,
                vertex_colors_clockwise
            } => output.push(Renderer2DAction {
                texture: None,
                vertices_clockwise: [
                    Renderer2DVertex {
                        position: vertex_positions_clockwise[0],
                        texture_coord: Vector2::ZERO,
                        color: vertex_colors_clockwise[0],
                        texture_mix: 0.0,
                        circle_mix: 0.0
                    },
                    Renderer2DVertex {
                        position: vertex_positions_clockwise[1],
                        texture_coord: Vector2::ZERO,
                        color: vertex_colors_clockwise[1],
                        texture_mix: 0.0,
                        circle_mix: 0.0
                    },
                    Renderer2DVertex {
                        position: vertex_positions_clockwise[2],
                        texture_coord: Vector2::ZERO,
                        color: vertex_colors_clockwise[2],
                        texture_mix: 0.0,
                        circle_mix: 0.0
                    }
                ]
            }),

            RenderQueueItem::TriangleTextured {
                vertex_positions_clockwise,
                vertex_colors_clockwise,
                vertex_texture_coords_clockwise,
                texture
            } => output.push(Renderer2DAction {
                texture: Some(texture.clone()),
                vertices_clockwise: [
                    Renderer2DVertex {
                        position: vertex_positions_clockwise[0],
                        texture_coord: vertex_texture_coords_clockwise[0],
                        color: vertex_colors_clockwise[0],
                        texture_mix: 1.0,
                        circle_mix: 0.0
                    },
                    Renderer2DVertex {
                        position: vertex_positions_clockwise[1],
                        texture_coord: vertex_texture_coords_clockwise[1],
                        color: vertex_colors_clockwise[1],
                        texture_mix: 1.0,
                        circle_mix: 0.0
                    },
                    Renderer2DVertex {
                        position: vertex_positions_clockwise[2],
                        texture_coord: vertex_texture_coords_clockwise[2],
                        color: vertex_colors_clockwise[2],
                        texture_mix: 1.0,
                        circle_mix: 0.0
                    }
                ]
            })
        }
    }
}

pub struct Renderer2D
{
    context: Rc<GLContextManager>,

    program: Rc<GLProgram>,

    render_queue: Vec<RenderQueueItem>,
    render_action_queue: Vec<Renderer2DAction>,

    glyph_cache: crate::font_cache::GlyphCache,
    attribute_buffers: AttributeBuffers,
    current_texture: Option<Rc<GLTexture>>,

    #[allow(dead_code)]
    uniforms: Uniforms
}

impl Renderer2D
{
    const ATTR_NAME_POSITION: &'static str = "in_Position";
    const ATTR_NAME_COLOR: &'static str = "in_Color";
    const ATTR_NAME_TEXTURE_COORD: &'static str = "in_TextureCoord";
    const ATTR_NAME_TEXTURE_MIX: &'static str = "in_TextureMix";
    const ATTR_NAME_CIRCLE_MIX: &'static str = "in_CircleMix";

    const UNIFORM_NAME_SCALE_X: &'static str = "in_ScaleX";
    const UNIFORM_NAME_SCALE_Y: &'static str = "in_ScaleY";
    const UNIFORM_NAME_TEXTURE: &'static str = "in_Texture";

    const ALL_ATTRIBUTES: [&'static str; 5] = [
        Renderer2D::ATTR_NAME_POSITION,
        Renderer2D::ATTR_NAME_COLOR,
        Renderer2D::ATTR_NAME_TEXTURE_COORD,
        Renderer2D::ATTR_NAME_TEXTURE_MIX,
        Renderer2D::ATTR_NAME_CIRCLE_MIX
    ];

    pub fn new(
        context: &Rc<GLContextManager>,
        viewport_size_pixels: Vector2<u32>
    ) -> Result<Self, BacktraceError<ErrorMessage>>
    {
        let vertex_shader = context
            .new_shader(
                GLShaderType::Vertex,
                include_str!("shaders/r2d_vertex.glsl")
            )
            .context("Failed to create Renderer2D vertex shader")?;

        let fragment_shader = context
            .new_shader(
                GLShaderType::Fragment,
                include_str!("shaders/r2d_fragment.glsl")
            )
            .context("Failed to create Renderer2D fragment shader")?;

        let program = context
            .new_program(
                &vertex_shader,
                &fragment_shader,
                &Renderer2D::ALL_ATTRIBUTES
            )
            .context("Failed to create Renderer2D program")?;

        let attribute_buffers = AttributeBuffers::new(context, &program)?;
        let uniforms = Uniforms::new(&program)?;

        context.use_program(&program);

        uniforms.set_texture_unit(0);

        uniforms.set_viewport_size_pixels(viewport_size_pixels);

        Ok(Renderer2D {
            context: context.clone(),
            program,
            render_queue: Vec::new(),
            render_action_queue: Vec::new(),
            glyph_cache: GlyphCache::new(),
            attribute_buffers,
            current_texture: None,
            uniforms
        })
    }

    pub fn set_viewport_size_pixels(&self, viewport_size_pixels: Vector2<u32>)
    {
        self.uniforms.set_viewport_size_pixels(viewport_size_pixels);
    }

    pub fn flush_render_queue(&mut self)
    {
        if self.render_queue.is_empty() {
            return;
        }

        self.render_action_queue.clear();
        self.attribute_buffers.clear();

        let mut has_text = false;

        for item in &self.render_queue {
            if let RenderQueueItem::FormattedTextBlock { block, .. } = item {
                for line in block.iter_lines() {
                    for glyph in line.iter_glyphs() {
                        self.glyph_cache.add_to_cache(&self.context, glyph);
                    }
                }

                has_text = true;
            }
        }

        if has_text {
            if let Err(err) = self.glyph_cache.prepare_for_draw(&self.context) {
                log::error!("Error updating font texture, continuing anyway: {:?}", err);
            }
        }

        for item in &self.render_queue {
            item.generate_actions(&mut self.render_action_queue, &self.glyph_cache);
        }

        self.render_queue.clear();

        for action in &self.render_action_queue {
            if !action.update_current_texture_if_empty(&mut self.current_texture) {
                Renderer2D::draw_buffers(
                    &self.context,
                    &self.program,
                    &mut self.attribute_buffers,
                    &mut self.current_texture
                );

                self.current_texture = action.texture.clone();
            }

            action.append_to_attribute_buffers(&mut self.attribute_buffers);
        }

        self.render_action_queue.clear();

        Renderer2D::draw_buffers(
            &self.context,
            &self.program,
            &mut self.attribute_buffers,
            &mut self.current_texture
        );
    }

    fn draw_buffers(
        context: &GLContextManager,
        program: &Rc<GLProgram>,
        attribute_buffers: &mut AttributeBuffers,
        current_texture: &mut Option<Rc<GLTexture>>
    )
    {
        let vertex_count = attribute_buffers.get_vertex_count();

        if vertex_count == 0 {
            return;
        }

        context.use_program(program);

        attribute_buffers.upload_and_clear();

        let current_texture = current_texture.take();

        match &current_texture {
            None => context.unbind_texture(),
            Some(texture) => context.bind_texture(texture)
        }

        context.draw_triangles(
            GLBlendEnabled::Enabled(GLBlendMode::OneMinusSrcAlpha),
            vertex_count
        );
    }

    pub(crate) fn create_image_from_raw_pixels<S: Into<Vector2<u32>>>(
        &self,
        data_type: ImageDataType,
        smoothing_mode: ImageSmoothingMode,
        size: S,
        data: &[u8]
    ) -> Result<ImageHandle, BacktraceError<ErrorMessage>>
    {
        let size = size.into();

        let gl_format = match data_type {
            ImageDataType::RGB => GLTextureImageFormatU8::RGB,
            ImageDataType::RGBA => GLTextureImageFormatU8::RGBA
        };

        let gl_smoothing = match smoothing_mode {
            ImageSmoothingMode::NearestNeighbor => GLTextureSmoothing::NearestNeighbour,
            ImageSmoothingMode::Linear => GLTextureSmoothing::Linear
        };

        let texture = self
            .context
            .new_texture()
            .context("Failed to create GPU texture")?;

        texture
            .set_image_data(&self.context, gl_format, gl_smoothing, &size, data)
            .context("Failed to upload image data")?;

        Ok(ImageHandle { size, texture })
    }

    #[cfg(any(feature = "image-loading", doc, doctest))]
    pub fn create_image_from_file_path<P: AsRef<Path>>(
        &mut self,
        data_type: Option<ImageFileFormat>,
        smoothing_mode: ImageSmoothingMode,
        path: P
    ) -> Result<ImageHandle, BacktraceError<ErrorMessage>>
    {
        let file = File::open(path.as_ref()).context(format!(
            "Failed to open file '{:?}' for reading",
            path.as_ref()
        ))?;

        self.create_image_from_file_bytes(data_type, smoothing_mode, BufReader::new(file))
    }

    #[cfg(any(feature = "image-loading", doc, doctest))]
    pub fn create_image_from_file_bytes<R: Seek + BufRead>(
        &mut self,
        data_type: Option<ImageFileFormat>,
        smoothing_mode: ImageSmoothingMode,
        file_bytes: R
    ) -> Result<ImageHandle, BacktraceError<ErrorMessage>>
    {
        let mut reader = image::io::Reader::new(file_bytes);

        match data_type {
            None => {
                reader = reader
                    .with_guessed_format()
                    .context("Could not guess file format")?
            }
            Some(format) => reader.set_format(match format {
                ImageFileFormat::PNG => image::ImageFormat::Png,
                ImageFileFormat::JPEG => image::ImageFormat::Jpeg,
                ImageFileFormat::GIF => image::ImageFormat::Gif,
                ImageFileFormat::BMP => image::ImageFormat::Bmp,
                ImageFileFormat::ICO => image::ImageFormat::Ico,
                ImageFileFormat::TIFF => image::ImageFormat::Tiff,
                ImageFileFormat::WebP => image::ImageFormat::WebP,
                ImageFileFormat::AVIF => image::ImageFormat::Avif,
                ImageFileFormat::PNM => image::ImageFormat::Pnm,
                ImageFileFormat::DDS => image::ImageFormat::Dds,
                ImageFileFormat::TGA => image::ImageFormat::Tga,
                ImageFileFormat::Farbfeld => image::ImageFormat::Farbfeld
            })
        }

        let image = reader.decode().context("Failed to parse image data")?;

        let dimensions = image.dimensions();

        let bytes_rgba8 = image.into_rgba8().into_raw();

        self.create_image_from_raw_pixels(
            ImageDataType::RGBA,
            smoothing_mode,
            dimensions,
            bytes_rgba8.as_slice()
        )
    }

    #[inline]
    pub(crate) fn clear_screen(&mut self, color: Color)
    {
        if color.a() < 1.0 {
            self.flush_render_queue();
        } else {
            self.render_queue.clear();
        }

        self.context.clear_screen(color);
    }

    #[inline]
    fn add_to_render_queue(&mut self, item: RenderQueueItem)
    {
        self.render_queue.push(item);

        if self.render_queue.len() > 100000 {
            self.flush_render_queue();
        }
    }

    #[inline]
    pub(crate) fn draw_triangle_three_color(
        &mut self,
        vertex_positions_clockwise: [Vector2<f32>; 3],
        vertex_colors_clockwise: [Color; 3]
    )
    {
        self.add_to_render_queue(RenderQueueItem::TriangleColored {
            vertex_positions_clockwise,
            vertex_colors_clockwise
        })
    }

    #[inline]
    pub(crate) fn draw_triangle_image_tinted(
        &mut self,
        vertex_positions_clockwise: [Vector2<f32>; 3],
        vertex_colors_clockwise: [Color; 3],
        vertex_texture_coords_clockwise: [Vector2<f32>; 3],
        image: &ImageHandle
    )
    {
        self.add_to_render_queue(RenderQueueItem::TriangleTextured {
            vertex_positions_clockwise,
            vertex_colors_clockwise,
            vertex_texture_coords_clockwise,
            texture: image.texture.clone()
        })
    }

    #[inline]
    pub(crate) fn draw_text<V: Into<Vector2<f32>>>(
        &mut self,
        position: V,
        color: Color,
        text: &Rc<FormattedTextBlock>
    )
    {
        self.add_to_render_queue(RenderQueueItem::FormattedTextBlock {
            position: position.into(),
            color,
            block: text.clone()
        })
    }

    #[inline]
    pub(crate) fn draw_circle_section(
        &mut self,
        vertex_positions_clockwise: [Vector2<f32>; 3],
        vertex_colors_clockwise: [Color; 3],
        vertex_normalized_circle_coords_clockwise: [Vector2<f32>; 3]
    )
    {
        self.add_to_render_queue(RenderQueueItem::CircleSectionColored {
            vertex_positions_clockwise,
            vertex_colors_clockwise,
            vertex_normalized_circle_coords_clockwise
        })
    }
}
