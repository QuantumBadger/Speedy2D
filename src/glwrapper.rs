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

use std::cell::RefCell;
use std::collections::HashMap;
use std::convert::TryInto;
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::num::TryFromIntError;
use std::ptr;
use std::rc::{Rc, Weak};

use crate::color::Color;
use crate::dimen::UVec2;
use crate::error::{BacktraceError, Context, ErrorMessage};
use crate::glbackend::constants::*;
use crate::glbackend::types::{
    GLTypeBuffer,
    GLTypeProgram,
    GLTypeShader,
    GLTypeTexture,
    GLTypeUniformLocation,
    GLenum,
    GLint,
    GLuint
};
use crate::glbackend::GLBackend;
use crate::{ImageDataType, RawBitmapData};

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
#[allow(dead_code)]
pub enum GLVersion
{
    OpenGL2_0,
    WebGL2_0
}

impl From<TryFromIntError> for BacktraceError<ErrorMessage>
{
    fn from(_: TryFromIntError) -> Self
    {
        ErrorMessage::msg("Integer conversion failed/out of bounds")
    }
}

fn gl_check_error_always(
    context: &GLContextManager
) -> Result<(), BacktraceError<ErrorMessage>>
{
    context.with_gl_backend(|backend| backend.gl_check_error_always())
}

fn gl_clear_and_log_old_error(context: &GLContextManager)
{
    context.with_gl_backend(|backend| backend.gl_clear_and_log_old_error())
}

trait GLHandleOwner<HandleType: GLHandleId>
{
    fn get_handle(&self) -> HandleType::HandleRawType;
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
enum GLHandleType
{
    Program,
    Shader,
    Buffer,
    Texture
}

trait GLHandleId: Debug + Hash + PartialEq + Eq
{
    type HandleRawType;
    fn delete(&self, context: &GLContextManager);
}

#[derive(Debug, Hash, PartialEq, Eq)]
struct GLHandleTypeProgram
{
    handle: GLTypeProgram
}

#[derive(Debug, Hash, PartialEq, Eq)]
struct GLHandleTypeShader
{
    handle: GLTypeShader
}

#[derive(Debug, Hash, PartialEq, Eq)]
struct GLHandleTypeBuffer
{
    handle: GLTypeBuffer
}

#[derive(Debug, Hash, PartialEq, Eq)]
struct GLHandleTypeTexture
{
    handle: GLTypeTexture
}

struct GLHandle<HandleType: GLHandleId>
{
    context: Weak<RefCell<GLContextManagerState>>,
    handle: HandleType,
    handle_type: GLHandleType
}

impl<HandleType: GLHandleId> Debug for GLHandle<HandleType>
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result
    {
        f.debug_struct("GLHandle")
            .field("handle", &self.handle)
            .field("handle_type", &self.handle_type)
            .finish()
    }
}

impl<HandleType: GLHandleId> Hash for GLHandle<HandleType>
{
    fn hash<H: Hasher>(&self, state: &mut H)
    {
        self.handle.hash(state);
        self.handle_type.hash(state);
    }
}

impl<HandleType: GLHandleId> PartialEq for GLHandle<HandleType>
{
    fn eq(&self, other: &Self) -> bool
    {
        match self.context.upgrade() {
            None => return false,
            Some(self_context) => match other.context.upgrade() {
                None => return false,
                Some(other_context) => {
                    if *RefCell::borrow(&self_context) != *RefCell::borrow(&other_context)
                    {
                        return false;
                    }
                }
            }
        }

        self.handle == other.handle && self.handle_type == other.handle_type
    }
}

impl<HandleType: GLHandleId> Eq for GLHandle<HandleType> {}

impl<HandleType: GLHandleId> GLHandle<HandleType>
{
    fn wrap<F>(
        context: &GLContextManager,
        handle_type: GLHandleType,
        handle_creator: F
    ) -> Result<Self, BacktraceError<ErrorMessage>>
    where
        F: FnOnce() -> Result<HandleType, BacktraceError<ErrorMessage>>
    {
        match handle_type {
            GLHandleType::Program => gl_clear_and_log_old_error(context),
            GLHandleType::Shader => gl_clear_and_log_old_error(context),
            GLHandleType::Buffer => {}
            GLHandleType::Texture => {}
        }

        let handle = handle_creator().context("Handle creation failed")?;

        match handle_type {
            GLHandleType::Program => gl_check_error_always(context)?,
            GLHandleType::Shader => gl_check_error_always(context)?,
            GLHandleType::Buffer => {}
            GLHandleType::Texture => {}
        }

        Ok(GLHandle {
            context: Rc::downgrade(&context.state),
            handle,
            handle_type
        })
    }

    #[inline]
    #[must_use]
    fn obtain_context_if_valid(&self) -> Option<GLContextManager>
    {
        obtain_context_from_weak_if_valid(&self.context)
    }
}

impl<HandleType: GLHandleId> Drop for GLHandle<HandleType>
{
    fn drop(&mut self)
    {
        if let Some(context) = self.obtain_context_if_valid() {
            self.handle.delete(&context);
        }
    }
}

impl GLHandleId for GLHandleTypeProgram
{
    type HandleRawType = GLTypeProgram;

    fn delete(&self, context: &GLContextManager)
    {
        context
            .with_gl_backend(|backend| unsafe { backend.gl_delete_program(self.handle) });
    }
}

impl GLHandleId for GLHandleTypeShader
{
    type HandleRawType = GLTypeShader;

    fn delete(&self, context: &GLContextManager)
    {
        context
            .with_gl_backend(|backend| unsafe { backend.gl_delete_shader(self.handle) });
    }
}

impl GLHandleId for GLHandleTypeBuffer
{
    type HandleRawType = GLTypeBuffer;

    fn delete(&self, context: &GLContextManager)
    {
        context
            .with_gl_backend(|backend| unsafe { backend.gl_delete_buffer(self.handle) });
    }
}

impl GLHandleId for GLHandleTypeTexture
{
    type HandleRawType = GLTypeTexture;

    fn delete(&self, context: &GLContextManager)
    {
        context
            .with_gl_backend(|backend| unsafe { backend.gl_delete_texture(self.handle) });
    }
}

#[derive(Debug)]
pub struct GLProgram
{
    handle: GLHandle<GLHandleTypeProgram>,
    attribute_handles: HashMap<&'static str, GLAttributeHandle>
}

impl Hash for GLProgram
{
    fn hash<H: Hasher>(&self, state: &mut H)
    {
        self.handle.hash(state);
    }
}

impl PartialEq for GLProgram
{
    fn eq(&self, other: &Self) -> bool
    {
        ptr::eq(self, other)
    }
}

impl Eq for GLProgram {}

impl GLHandleOwner<GLHandleTypeProgram> for GLProgram
{
    fn get_handle(&self) -> <GLHandleTypeProgram as GLHandleId>::HandleRawType
    {
        self.handle.handle.handle
    }
}

impl GLProgram
{
    fn new(context: &GLContextManager) -> Result<Self, BacktraceError<ErrorMessage>>
    {
        context.with_gl_backend(|backend| {
            Ok(GLProgram {
                handle: GLHandle::wrap(context, GLHandleType::Program, || unsafe {
                    Ok(GLHandleTypeProgram {
                        handle: backend.gl_create_program()?
                    })
                })?,
                attribute_handles: HashMap::new()
            })
        })
    }

    fn attach_shader(
        &mut self,
        context: &GLContextManager,
        shader: &GLShader
    ) -> Result<(), BacktraceError<ErrorMessage>>
    {
        context.with_gl_backend(|backend| unsafe {
            backend.gl_attach_shader(self.get_handle(), shader.get_handle());
        });

        gl_check_error_always(context)?;

        Ok(())
    }

    fn link(
        context: &GLContextManager,
        vertex_shader: &GLShader,
        fragment_shader: &GLShader,
        attribute_names: impl IntoIterator<Item = &'static &'static str>
    ) -> Result<Self, BacktraceError<ErrorMessage>>
    {
        gl_clear_and_log_old_error(context);

        let mut program = GLProgram::new(context)?;

        program.attach_shader(context, vertex_shader)?;
        program.attach_shader(context, fragment_shader)?;

        context.with_gl_backend(|backend| unsafe {
            backend.gl_link_program(program.get_handle());
        });

        gl_check_error_always(context)?;

        context.with_gl_backend(|backend| unsafe {
            if backend.gl_get_program_link_status(program.get_handle()) {
                Ok(())
            } else {
                let msg = backend.gl_get_program_info_log(program.get_handle())?;
                Err(ErrorMessage::msg(format!(
                    "Program linking failed: '{msg}'"
                )))
            }
        })?;

        gl_check_error_always(context)?;

        for attribute_name in attribute_names.into_iter() {
            program.attribute_handles.insert(
                attribute_name.as_ref(),
                program.get_attribute_handle(attribute_name.as_ref())?
            );
        }

        Ok(program)
    }

    fn enable(&self, context: &GLContextManager)
    {
        context.with_gl_backend(|backend| {
            unsafe {
                backend.gl_use_program(self.get_handle());
            }

            for attribute in self.attribute_handles.values() {
                unsafe {
                    backend.gl_enable_vertex_attrib_array(attribute.handle);
                }
            }
        });
    }

    fn disable(&self, context: &GLContextManager)
    {
        context.with_gl_backend(|backend| {
            for attribute in self.attribute_handles.values() {
                unsafe {
                    backend.gl_disable_vertex_attrib_array(attribute.handle);
                }
            }
        });
    }

    pub fn get_attribute_handle(
        &self,
        name: &str
    ) -> Result<GLAttributeHandle, BacktraceError<ErrorMessage>>
    {
        let context = self
            .handle
            .obtain_context_if_valid()
            .ok_or_else(|| ErrorMessage::msg("GL context no longer valid"))?;

        let handle = context.with_gl_backend(|backend| unsafe {
            backend.gl_get_attrib_location(self.get_handle(), name)
        });

        gl_check_error_always(&context)?;

        match handle {
            None => Err(ErrorMessage::msg(format!(
                "Attribute handle {name} is invalid"
            ))),
            Some(handle) => Ok(GLAttributeHandle { handle })
        }
    }

    pub fn get_uniform_handle(
        &self,
        context: &GLContextManager,
        name: &str
    ) -> Result<GLUniformHandle, BacktraceError<ErrorMessage>>
    {
        if !context.is_valid() {
            return Err(ErrorMessage::msg("GL context no longer valid"));
        }

        let handle = context.with_gl_backend(|backend| unsafe {
            backend.gl_get_uniform_location(self.get_handle(), name)
        });

        gl_check_error_always(context)?;

        match handle {
            None => Err(ErrorMessage::msg(format!(
                "Uniform handle {name} is invalid"
            ))),
            Some(handle) => Ok(GLUniformHandle { handle })
        }
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub enum GLShaderType
{
    Vertex,
    Fragment
}

impl GLShaderType
{
    fn gl_constant(&self) -> GLenum
    {
        match self {
            GLShaderType::Vertex => GL_VERTEX_SHADER,
            GLShaderType::Fragment => GL_FRAGMENT_SHADER
        }
    }
}

pub struct GLShader
{
    handle: GLHandle<GLHandleTypeShader>
}

impl GLHandleOwner<GLHandleTypeShader> for GLShader
{
    fn get_handle(&self) -> <GLHandleTypeShader as GLHandleId>::HandleRawType
    {
        self.handle.handle.handle
    }
}

impl GLShader
{
    fn new(
        context: &GLContextManager,
        shader_type: GLShaderType
    ) -> Result<Self, BacktraceError<ErrorMessage>>
    {
        Ok(GLShader {
            handle: GLHandle::wrap(context, GLHandleType::Shader, || {
                context.with_gl_backend(|backend| unsafe {
                    Ok(GLHandleTypeShader {
                        handle: backend.gl_create_shader(shader_type.gl_constant())?
                    })
                })
            })?
        })
    }

    fn compile(
        context: &GLContextManager,
        shader_type: GLShaderType,
        source: &str
    ) -> Result<Self, BacktraceError<ErrorMessage>>
    {
        gl_clear_and_log_old_error(context);

        let shader = GLShader::new(context, shader_type)?;

        context.with_gl_backend(|backend| unsafe {
            backend.gl_shader_source(shader.get_handle(), source);
            backend.gl_check_error_always()?;

            backend.gl_compile_shader(shader.get_handle());
            backend.gl_check_error_always()?;

            if backend.gl_get_shader_compile_status(shader.get_handle()) {
                Ok(shader)
            } else {
                Err(ErrorMessage::msg(context.with_gl_backend(|backend| {
                    backend.gl_get_shader_info_log(shader.get_handle())
                })?))
            }
        })
    }
}

#[derive(Debug)]
pub struct GLAttributeHandle
{
    handle: GLuint
}

#[derive(Debug)]
pub struct GLUniformHandle
{
    handle: GLTypeUniformLocation
}

impl GLUniformHandle
{
    pub fn set_value_float(&self, context: &GLContextManager, value: f32)
    {
        context.with_gl_backend(|backend| unsafe {
            backend.gl_uniform_1f(&self.handle, value)
        })
    }

    pub fn set_value_int(&self, context: &GLContextManager, value: i32)
    {
        context.with_gl_backend(|backend| unsafe {
            backend.gl_uniform_1i(&self.handle, value)
        })
    }
}

pub enum GLBufferTarget
{
    Array,
    #[allow(dead_code)]
    ElementArray
}

impl GLBufferTarget
{
    fn gl_constant(&self) -> GLenum
    {
        match self {
            GLBufferTarget::Array => GL_ARRAY_BUFFER,
            GLBufferTarget::ElementArray => GL_ELEMENT_ARRAY_BUFFER
        }
    }
}

pub struct GLBuffer
{
    handle: GLHandle<GLHandleTypeBuffer>,
    target: GLBufferTarget,
    components_per_vertex: GLint,
    attrib_index: GLAttributeHandle
}

impl GLHandleOwner<GLHandleTypeBuffer> for GLBuffer
{
    fn get_handle(&self) -> <GLHandleTypeBuffer as GLHandleId>::HandleRawType
    {
        self.handle.handle.handle
    }
}

impl GLBuffer
{
    fn new(
        context: &GLContextManager,
        target: GLBufferTarget,
        components_per_vertex: GLint,
        attrib_index: GLAttributeHandle
    ) -> Result<Self, BacktraceError<ErrorMessage>>
    {
        gl_clear_and_log_old_error(context);

        let handle = GLHandle::wrap(context, GLHandleType::Buffer, || {
            context.with_gl_backend(|backend| unsafe {
                Ok(GLHandleTypeBuffer {
                    handle: backend.gl_gen_buffer()?
                })
            })
        })?;

        Ok(GLBuffer {
            handle,
            target,
            components_per_vertex,
            attrib_index
        })
    }

    pub fn set_data(&mut self, context: &GLContextManager, data: &[f32])
    {
        if !context.is_valid() {
            log::warn!("Ignoring buffer set_data: invalid GL context");
            return;
        }

        context.with_gl_backend(|backend| unsafe {
            backend.gl_bind_buffer(self.target.gl_constant(), self.get_handle());

            backend.gl_buffer_data_f32(self.target.gl_constant(), data, GL_DYNAMIC_DRAW);

            backend.gl_vertex_attrib_pointer_f32(
                self.attrib_index.handle,
                self.components_per_vertex,
                GL_FLOAT,
                false,
                0,
                0
            )
        });
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub enum GLTextureSmoothing
{
    NearestNeighbour,
    Linear
}

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub enum GLTextureImageFormatU8
{
    #[allow(dead_code)]
    Red,
    RGB,
    RGBA
}

impl From<ImageDataType> for GLTextureImageFormatU8
{
    fn from(value: ImageDataType) -> Self
    {
        match value {
            ImageDataType::RGB => Self::RGB,
            ImageDataType::RGBA => Self::RGBA
        }
    }
}

impl GLTextureImageFormatU8
{
    fn get_internal_format(&self) -> GLenum
    {
        match self {
            GLTextureImageFormatU8::Red => GL_R8,
            GLTextureImageFormatU8::RGB => GL_RGB8,
            GLTextureImageFormatU8::RGBA => GL_RGBA8
        }
    }

    fn get_format(&self) -> GLenum
    {
        match self {
            GLTextureImageFormatU8::Red => GL_RED,
            GLTextureImageFormatU8::RGB => GL_RGB,
            GLTextureImageFormatU8::RGBA => GL_RGBA
        }
    }

    fn get_bytes_per_pixel(&self) -> usize
    {
        match self {
            GLTextureImageFormatU8::Red => 1,
            GLTextureImageFormatU8::RGB => 3,
            GLTextureImageFormatU8::RGBA => 4
        }
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct GLTexture
{
    handle: Rc<GLHandle<GLHandleTypeTexture>>
}

impl GLHandleOwner<GLHandleTypeTexture> for GLTexture
{
    fn get_handle(&self) -> <GLHandleTypeTexture as GLHandleId>::HandleRawType
    {
        self.handle.handle.handle
    }
}

impl GLTexture
{
    fn new(context: &GLContextManager) -> Result<Self, BacktraceError<ErrorMessage>>
    {
        let handle = GLHandle::wrap(context, GLHandleType::Texture, || {
            context.with_gl_backend(|backend| unsafe {
                Ok(GLHandleTypeTexture {
                    handle: backend.gl_gen_texture()?
                })
            })
        })?;

        Ok(GLTexture {
            handle: Rc::new(handle)
        })
    }

    pub fn set_image_data(
        &self,
        context: &GLContextManager,
        format: GLTextureImageFormatU8,
        smoothing: GLTextureSmoothing,
        size: &UVec2,
        data: &[u8]
    ) -> Result<(), BacktraceError<ErrorMessage>>
    {
        if !context.is_valid() {
            log::warn!("Ignoring texture set_image_data: invalid GL context");
            return Ok(());
        }

        let smoothing_constant = match smoothing {
            GLTextureSmoothing::NearestNeighbour => GL_NEAREST,
            GLTextureSmoothing::Linear => GL_LINEAR
        } as GLint;

        context.bind_texture(self);

        let width_stride_bytes = size.x as usize * format.get_bytes_per_pixel();

        let unpack_alignment = if width_stride_bytes % 8 == 0 {
            8
        } else if width_stride_bytes % 4 == 0 {
            4
        } else if width_stride_bytes % 2 == 0 {
            2
        } else {
            1
        };

        context.with_gl_backend::<Result<(), BacktraceError<ErrorMessage>>, _>(
            |backend| unsafe {
                backend.gl_pixel_store_i(GL_UNPACK_ALIGNMENT, unpack_alignment);
                backend.gl_tex_parameter_i(
                    GL_TEXTURE_2D,
                    GL_TEXTURE_WRAP_S,
                    GL_CLAMP_TO_EDGE as GLint
                );
                backend.gl_tex_parameter_i(
                    GL_TEXTURE_2D,
                    GL_TEXTURE_WRAP_T,
                    GL_CLAMP_TO_EDGE as GLint
                );
                backend.gl_tex_parameter_i(
                    GL_TEXTURE_2D,
                    GL_TEXTURE_MIN_FILTER,
                    smoothing_constant
                );
                backend.gl_tex_parameter_i(
                    GL_TEXTURE_2D,
                    GL_TEXTURE_MAG_FILTER,
                    smoothing_constant
                );

                backend.gl_tex_image_2d(
                    GL_TEXTURE_2D,
                    0,
                    format
                        .get_internal_format()
                        .try_into()
                        .context("Failed to cast internal format")?,
                    size.x.try_into()?,
                    size.y.try_into()?,
                    0,
                    format.get_format(),
                    GL_UNSIGNED_BYTE,
                    Some(data)
                );

                Ok(())
            }
        )
    }
}

#[must_use]
fn obtain_context_if_valid(
    state: &RefCell<GLContextManagerState>
) -> Option<GLContextManager>
{
    let state = state.borrow_mut();

    if state.is_valid {
        Some(GLContextManager {
            state: state.weak_ref_to_self.upgrade().unwrap()
        })
    } else {
        None
    }
}

#[inline]
#[must_use]
fn obtain_context_from_weak_if_valid(
    state: &Weak<RefCell<GLContextManagerState>>
) -> Option<GLContextManager>
{
    match state.upgrade() {
        None => None,
        Some(state) => obtain_context_if_valid(&state)
    }
}

struct GLContextManagerState
{
    is_valid: bool,
    active_texture: Option<GLTexture>,
    active_program: Option<Rc<GLProgram>>,
    active_blend_mode: Option<GLBlendEnabled>,
    viewport_size: Option<UVec2>,
    scissor_enabled: bool,
    gl_backend: Rc<dyn GLBackend + 'static>,
    gl_version: GLVersion,
    weak_ref_to_self: Weak<RefCell<GLContextManagerState>>
}

impl PartialEq for GLContextManagerState
{
    fn eq(&self, other: &Self) -> bool
    {
        ptr::eq(self, other)
    }
}

#[derive(Clone)]
pub struct GLContextManager
{
    state: Rc<RefCell<GLContextManagerState>>
}

impl GLContextManager
{
    pub fn create(
        gl_backend: Rc<dyn GLBackend>,
        gl_version: GLVersion
    ) -> Result<Self, BacktraceError<ErrorMessage>>
    {
        let manager = GLContextManager {
            state: Rc::new(RefCell::new(GLContextManagerState {
                is_valid: true,
                active_texture: None,
                active_program: None,
                active_blend_mode: None,
                viewport_size: None,
                scissor_enabled: false,
                gl_backend,
                gl_version,
                weak_ref_to_self: Weak::new()
            }))
        };

        RefCell::borrow_mut(&manager.state).weak_ref_to_self =
            Rc::downgrade(&manager.state);

        log::info!("GL context manager created");

        Ok(manager)
    }

    pub fn mark_invalid(&self)
    {
        log::info!("GL context manager is now inactive");
        RefCell::borrow_mut(&self.state).is_valid = false;
    }

    pub fn new_buffer(
        &self,
        target: GLBufferTarget,
        components_per_vertex: GLint,
        attrib_index: GLAttributeHandle
    ) -> Result<GLBuffer, BacktraceError<ErrorMessage>>
    {
        self.ensure_valid()?;
        GLBuffer::new(self, target, components_per_vertex, attrib_index)
    }

    pub fn new_shader(
        &self,
        shader_type: GLShaderType,
        source: &str
    ) -> Result<GLShader, BacktraceError<ErrorMessage>>
    {
        self.ensure_valid()?;
        GLShader::compile(self, shader_type, source)
    }

    pub fn new_program(
        &self,
        vertex_shader: &GLShader,
        fragment_shader: &GLShader,
        attribute_names: impl IntoIterator<Item = &'static &'static str>
    ) -> Result<Rc<GLProgram>, BacktraceError<ErrorMessage>>
    {
        self.ensure_valid()?;

        Ok(Rc::new(GLProgram::link(
            self,
            vertex_shader,
            fragment_shader,
            attribute_names
        )?))
    }

    pub fn new_texture(&self) -> Result<GLTexture, BacktraceError<ErrorMessage>>
    {
        self.ensure_valid()?;
        GLTexture::new(self)
    }

    pub fn set_viewport_size(&self, size: UVec2)
    {
        if !self.is_valid() {
            log::warn!("Ignoring set_viewport_size: invalid GL context");
            return;
        }

        log::info!("Setting viewport size to {}x{}", size.x, size.y);

        self.state.borrow_mut().viewport_size = Some(size);

        self.with_gl_backend(|backend| unsafe {
            backend.gl_viewport(0, 0, size.x as i32, size.y as i32);
        });
    }

    pub fn bind_texture(&self, texture: &GLTexture)
    {
        if !self.is_valid() {
            log::warn!("Ignoring bind_texture: invalid GL context");
            return;
        }

        if RefCell::borrow(&self.state).active_texture.as_ref() == Some(texture) {
            // Already bound
            return;
        }

        // Drop separately to avoid a duplicate borrow of `state`.
        let old_active_texture = RefCell::borrow_mut(&self.state).active_texture.take();
        drop(old_active_texture);

        RefCell::borrow_mut(&self.state).active_texture = Some(texture.clone());

        self.with_gl_backend(|backend| unsafe {
            backend.gl_active_texture(GL_TEXTURE0);
            backend.gl_bind_texture(GL_TEXTURE_2D, texture.get_handle());
        });
    }

    pub fn unbind_texture(&self)
    {
        #[cfg(not(target_arch = "wasm32"))]
        {
            if !self.is_valid() {
                log::warn!("Ignoring unbind_texture: invalid GL context");
                return;
            }

            if RefCell::borrow(&self.state)
                .active_texture
                .as_ref()
                .is_none()
            {
                // Already unbound
                return;
            }

            self.with_gl_backend(|backend| unsafe {
                backend.gl_active_texture(GL_TEXTURE0);
                backend.gl_bind_texture(GL_TEXTURE_2D, 0);
            });

            RefCell::borrow_mut(&self.state).active_texture = None;
        }
    }

    pub fn use_program(&self, program: &Rc<GLProgram>)
    {
        if !self.is_valid() {
            log::warn!("Ignoring use_program: invalid GL context");
            return;
        }

        if RefCell::borrow(&self.state).active_program.as_ref() == Some(program) {
            // Already bound
            return;
        }

        if let Some(existing_program) = &RefCell::borrow_mut(&self.state).active_program {
            existing_program.disable(self);
        }

        RefCell::borrow_mut(&self.state).active_program = Some(program.clone());
        program.enable(self);
    }

    fn set_blend_mode(&self, blend_mode: GLBlendEnabled)
    {
        if RefCell::borrow(&self.state).active_blend_mode == Some(blend_mode.clone()) {
            return;
        }

        RefCell::borrow_mut(&self.state).active_blend_mode = Some(blend_mode.clone());

        match blend_mode {
            GLBlendEnabled::Enabled(mode) => match mode {
                GLBlendMode::OneMinusSrcAlpha => self.with_gl_backend(|backend| unsafe {
                    backend.gl_enable(GL_BLEND);
                    backend.gl_blend_func(GL_SRC_ALPHA, GL_ONE_MINUS_SRC_ALPHA);
                })
            },

            GLBlendEnabled::Disabled => self.with_gl_backend(|backend| unsafe {
                backend.gl_disable(GL_BLEND);
            })
        }
    }

    pub fn set_enable_scissor(&self, enabled: bool)
    {
        if enabled != self.state.borrow().scissor_enabled {
            self.with_gl_backend(|backend| unsafe {
                match enabled {
                    true => backend.gl_enable(GL_SCISSOR_TEST),
                    false => backend.gl_disable(GL_SCISSOR_TEST)
                }
            });
            self.state.borrow_mut().scissor_enabled = enabled;
        }
    }

    pub fn set_clip(&self, x: i32, y: i32, width: i32, height: i32)
    {
        let vp_height = match self.state.borrow().viewport_size {
            None => panic!("Call to set_clip before viewport size set"),
            Some(viewport_size) => viewport_size.y as i32
        };
        self.with_gl_backend(|backend| unsafe {
            backend.gl_scissor(x, vp_height - y - height, width, height);
        });
    }

    pub fn draw_triangles(&self, blend_mode: GLBlendEnabled, vertex_count: usize)
    {
        if !self.is_valid() {
            log::warn!("Ignoring draw_triangles: invalid GL context");
            return;
        }

        self.set_blend_mode(blend_mode);

        self.with_gl_backend(|backend| unsafe {
            backend.gl_draw_arrays(GL_TRIANGLES, 0, vertex_count.try_into().unwrap());
        });
    }

    pub fn clear_screen(&self, color: Color)
    {
        if !self.is_valid() {
            log::warn!("Ignoring clear_screen: invalid GL context");
            return;
        }

        self.with_gl_backend(|backend| unsafe {
            backend.gl_clear_color(color.r(), color.g(), color.b(), color.a());
            backend.gl_clear(GL_COLOR_BUFFER_BIT);
        });
    }

    fn with_gl_backend<Return, F>(&self, callback: F) -> Return
    where
        F: FnOnce(&Rc<dyn GLBackend>) -> Return
    {
        let backend = RefCell::borrow(&self.state).gl_backend.clone();
        callback(&backend)
    }

    fn is_valid(&self) -> bool
    {
        RefCell::borrow(&self.state).is_valid
    }

    fn ensure_valid(&self) -> Result<(), BacktraceError<ErrorMessage>>
    {
        if !self.is_valid() {
            Err(ErrorMessage::msg("GL context no longer valid"))
        } else {
            Ok(())
        }
    }

    pub fn version(&self) -> GLVersion
    {
        self.state.borrow().gl_version
    }

    pub fn capture(&mut self, format: ImageDataType) -> RawBitmapData
    {
        let viewport_size = match self.state.borrow().viewport_size {
            None => return RawBitmapData::new(vec![], (0, 0), format),
            Some(value) => value
        };

        let width: usize = viewport_size.x.try_into().unwrap();
        let height: usize = viewport_size.y.try_into().unwrap();

        let gl_format = GLTextureImageFormatU8::from(format);

        let bpp = gl_format.get_bytes_per_pixel();
        let gl_format = gl_format.get_format();

        let bytes = width * height * bpp;

        let mut buf: Vec<u8> = Vec::with_capacity(bytes);

        self.with_gl_backend(|backend| unsafe {
            backend.gl_read_pixels(
                0,
                0,
                width.try_into().unwrap(),
                height.try_into().unwrap(),
                gl_format,
                GL_UNSIGNED_BYTE,
                buf.spare_capacity_mut()
            );
        });

        unsafe {
            buf.set_len(bytes);
        }

        let row_bytes = width * bpp;

        let buf_ptr = buf.as_mut_ptr();

        for row in 0..(height / 2) {
            let bottom_row = height - row - 1;

            let top_start = row * row_bytes;
            let bottom_start = bottom_row * row_bytes;

            unsafe {
                ptr::swap_nonoverlapping(
                    buf_ptr.add(top_start),
                    buf_ptr.add(bottom_start),
                    row_bytes
                );
            }
        }

        RawBitmapData::new(buf, viewport_size, format)
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub enum GLBlendMode
{
    OneMinusSrcAlpha
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub enum GLBlendEnabled
{
    Enabled(GLBlendMode),
    #[allow(dead_code)]
    Disabled
}
