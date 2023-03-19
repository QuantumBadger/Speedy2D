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

use std::mem::MaybeUninit;

use glow::{HasContext, PixelPackData};
#[cfg(not(target_arch = "wasm32"))]
use {std::convert::TryInto, std::ffi::CStr, std::os::raw::c_void};

use crate::error::{BacktraceError, ErrorMessage};
use crate::glbackend::constants::*;
use crate::glbackend::types::*;

pub mod types
{
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    pub type GLenum = u32;
    pub type GLuint = u32;
    pub type GLint = i32;
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    pub type GLchar = std::os::raw::c_char;
    pub type GLsizei = i32;

    pub type GLTypeShader = glow::Shader;
    pub type GLTypeProgram = glow::Program;
    pub type GLTypeBuffer = glow::Buffer;
    pub type GLTypeTexture = glow::Texture;
    pub type GLTypeUniformLocation = glow::UniformLocation;
}

pub mod constants
{
    use crate::glbackend::types::GLenum;

    #[allow(dead_code)]
    pub const GL_VERSION: GLenum = glow::VERSION;

    pub const GL_TEXTURE0: GLenum = glow::TEXTURE0;

    pub const GL_TEXTURE_2D: GLenum = glow::TEXTURE_2D;

    pub const GL_BLEND: GLenum = glow::BLEND;

    pub const GL_SCISSOR_TEST: GLenum = glow::SCISSOR_TEST;

    pub const GL_ONE: GLenum = glow::ONE;
    pub const GL_SRC_ALPHA: GLenum = glow::SRC_ALPHA;
    pub const GL_ONE_MINUS_SRC_ALPHA: GLenum = glow::ONE_MINUS_SRC_ALPHA;

    pub const GL_NEAREST: GLenum = glow::NEAREST;
    pub const GL_LINEAR: GLenum = glow::LINEAR;

    pub const GL_ARRAY_BUFFER: GLenum = glow::ARRAY_BUFFER;
    pub const GL_ELEMENT_ARRAY_BUFFER: GLenum = glow::ELEMENT_ARRAY_BUFFER;

    pub const GL_DYNAMIC_DRAW: GLenum = glow::DYNAMIC_DRAW;

    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    pub const GL_FALSE: u8 = glow::FALSE;
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    pub const GL_TRUE: u8 = glow::TRUE;

    pub const GL_FLOAT: GLenum = glow::FLOAT;
    pub const GL_UNSIGNED_BYTE: GLenum = glow::UNSIGNED_BYTE;

    pub const GL_R8: GLenum = glow::R8;
    pub const GL_RGB8: GLenum = glow::RGB8;
    pub const GL_RGBA8: GLenum = glow::RGBA8;

    pub const GL_RED: GLenum = glow::RED;
    pub const GL_RGB: GLenum = glow::RGB;
    pub const GL_RGBA: GLenum = glow::RGBA;

    pub const GL_TEXTURE_WRAP_S: GLenum = glow::TEXTURE_WRAP_S;
    pub const GL_TEXTURE_WRAP_T: GLenum = glow::TEXTURE_WRAP_T;
    pub const GL_TEXTURE_MIN_FILTER: GLenum = glow::TEXTURE_MIN_FILTER;
    pub const GL_TEXTURE_MAG_FILTER: GLenum = glow::TEXTURE_MAG_FILTER;
    pub const GL_CLAMP_TO_EDGE: GLenum = glow::CLAMP_TO_EDGE;

    pub const GL_TRIANGLES: GLenum = glow::TRIANGLES;

    pub const GL_COLOR_BUFFER_BIT: GLenum = glow::COLOR_BUFFER_BIT;

    pub const GL_NO_ERROR: GLenum = glow::NO_ERROR;
    pub const GL_INVALID_ENUM: GLenum = glow::INVALID_ENUM;
    pub const GL_INVALID_VALUE: GLenum = glow::INVALID_VALUE;
    pub const GL_INVALID_OPERATION: GLenum = glow::INVALID_OPERATION;
    pub const GL_INVALID_FRAMEBUFFER_OPERATION: GLenum =
        glow::INVALID_FRAMEBUFFER_OPERATION;
    pub const GL_OUT_OF_MEMORY: GLenum = glow::OUT_OF_MEMORY;
    pub const GL_STACK_UNDERFLOW: GLenum = glow::STACK_UNDERFLOW;
    pub const GL_STACK_OVERFLOW: GLenum = glow::STACK_OVERFLOW;

    pub const GL_VERTEX_SHADER: GLenum = glow::VERTEX_SHADER;
    pub const GL_FRAGMENT_SHADER: GLenum = glow::FRAGMENT_SHADER;

    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    pub const GL_LINK_STATUS: GLenum = glow::LINK_STATUS;
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    pub const GL_COMPILE_STATUS: GLenum = glow::COMPILE_STATUS;
    #[allow(dead_code)]
    pub const GL_INFO_LOG_LENGTH: GLenum = glow::INFO_LOG_LENGTH;

    pub const GL_DEBUG_SEVERITY_HIGH: GLenum = glow::DEBUG_SEVERITY_HIGH;
    pub const GL_DEBUG_SEVERITY_MEDIUM: GLenum = glow::DEBUG_SEVERITY_MEDIUM;
    pub const GL_DEBUG_SEVERITY_LOW: GLenum = glow::DEBUG_SEVERITY_LOW;
    pub const GL_DEBUG_OUTPUT: GLenum = glow::DEBUG_OUTPUT;
    pub const GL_DEBUG_OUTPUT_SYNCHRONOUS: GLenum = glow::DEBUG_OUTPUT_SYNCHRONOUS;

    pub const GL_UNPACK_ALIGNMENT: GLenum = glow::UNPACK_ALIGNMENT;
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
enum GLErrorCode
{
    InvalidEnum,
    InvalidValue,
    InvalidOperation,
    InvalidFramebufferOperation,
    OutOfMemory,
    StackUnderflow,
    StackOverflow,
    Other(GLenum)
}

impl From<GLenum> for GLErrorCode
{
    fn from(constant: GLenum) -> Self
    {
        match constant {
            GL_INVALID_ENUM => GLErrorCode::InvalidEnum,
            GL_INVALID_VALUE => GLErrorCode::InvalidValue,
            GL_INVALID_OPERATION => GLErrorCode::InvalidOperation,
            GL_INVALID_FRAMEBUFFER_OPERATION => GLErrorCode::InvalidFramebufferOperation,
            GL_OUT_OF_MEMORY => GLErrorCode::OutOfMemory,
            GL_STACK_UNDERFLOW => GLErrorCode::StackUnderflow,
            GL_STACK_OVERFLOW => GLErrorCode::StackOverflow,
            _ => GLErrorCode::Other(constant)
        }
    }
}

impl From<GLErrorCode> for BacktraceError<ErrorMessage>
{
    fn from(err: GLErrorCode) -> Self
    {
        ErrorMessage::msg(format!("Got GL error code {err:?}"))
    }
}

pub trait GLBackend
{
    unsafe fn gl_delete_program(&self, handle: GLTypeProgram);
    unsafe fn gl_delete_shader(&self, handle: GLTypeShader);
    unsafe fn gl_delete_buffer(&self, handle: GLTypeBuffer);
    unsafe fn gl_delete_texture(&self, handle: GLTypeTexture);
    unsafe fn gl_active_texture(&self, unit: GLenum);
    unsafe fn gl_bind_texture(&self, target: GLenum, handle: GLTypeTexture);
    unsafe fn gl_enable(&self, cap: GLenum);
    unsafe fn gl_disable(&self, cap: GLenum);
    unsafe fn gl_blend_func(&self, sfactor: GLenum, dfactor: GLenum);
    unsafe fn gl_blend_func_separate(
        &self,
        sfactor: GLenum,
        dfactor: GLenum,
        sfactor_alpha: GLenum,
        dfactor_alpha: GLenum
    );
    unsafe fn gl_use_program(&self, handle: GLTypeProgram);
    unsafe fn gl_enable_vertex_attrib_array(&self, handle: GLuint);
    unsafe fn gl_disable_vertex_attrib_array(&self, handle: GLuint);
    unsafe fn gl_uniform_1f(&self, handle: &GLTypeUniformLocation, value: f32);
    unsafe fn gl_uniform_1i(&self, handle: &GLTypeUniformLocation, value: GLint);
    unsafe fn gl_attach_shader(&self, program: GLTypeProgram, shader: GLTypeShader);
    unsafe fn gl_link_program(&self, program: GLTypeProgram);
    unsafe fn gl_shader_source(&self, handle: GLTypeShader, source: &str);
    unsafe fn gl_compile_shader(&self, handle: GLTypeShader);
    unsafe fn gl_tex_parameter_i(&self, target: GLenum, parameter: GLenum, value: GLint);
    unsafe fn gl_bind_buffer(&self, target: GLenum, handle: GLTypeBuffer);
    unsafe fn gl_buffer_data(&self, target: GLenum, data: &[u8], usage: GLenum);
    unsafe fn gl_draw_arrays(&self, mode: GLenum, first: GLint, count: GLsizei);
    unsafe fn gl_clear_color(&self, r: f32, g: f32, b: f32, a: f32);
    unsafe fn gl_clear(&self, mask: GLenum);
    unsafe fn gl_enable_debug_message_callback(&self);
    unsafe fn gl_get_string(&self, parameter: GLenum) -> String;
    unsafe fn gl_viewport(&self, x: i32, y: i32, width: i32, height: i32);
    unsafe fn gl_scissor(&self, x: GLint, y: GLint, width: GLsizei, height: GLsizei);
    unsafe fn gl_pixel_store_i(&self, param: GLenum, value: GLint);

    unsafe fn gl_vertex_attrib_pointer_f32(
        &self,
        index: GLuint,
        size: GLsizei,
        data_type: GLenum,
        normalized: bool,
        stride: GLsizei,
        offset: GLsizei
    );

    #[allow(clippy::too_many_arguments)]
    unsafe fn gl_tex_image_2d(
        &self,
        target: GLenum,
        level: GLint,
        internal_format: GLint,
        width: GLsizei,
        height: GLsizei,
        border: GLint,
        format: GLenum,
        data_type: GLenum,
        pixels: Option<&[u8]>
    );

    #[allow(clippy::too_many_arguments)]
    unsafe fn gl_tex_sub_image_2d(
        &self,
        target: GLenum,
        level: GLint,
        x: GLint,
        y: GLint,
        width: GLsizei,
        height: GLsizei,
        format: GLenum,
        data_type: GLenum,
        pixels: &[u8]
    );

    unsafe fn gl_create_program(
        &self
    ) -> Result<GLTypeProgram, BacktraceError<ErrorMessage>>;

    unsafe fn gl_create_shader(
        &self,
        shader_type: GLenum
    ) -> Result<GLTypeShader, BacktraceError<ErrorMessage>>;

    unsafe fn gl_gen_buffer(&self) -> Result<GLTypeBuffer, BacktraceError<ErrorMessage>>;

    unsafe fn gl_gen_texture(
        &self
    ) -> Result<GLTypeTexture, BacktraceError<ErrorMessage>>;

    #[must_use]
    unsafe fn gl_get_error(&self) -> GLenum;

    #[must_use]
    unsafe fn gl_get_attrib_location(
        &self,
        program: GLTypeProgram,
        name: &str
    ) -> Option<GLuint>;

    #[must_use]
    unsafe fn gl_get_uniform_location(
        &self,
        program: GLTypeProgram,
        name: &str
    ) -> Option<GLTypeUniformLocation>;

    #[must_use]
    unsafe fn gl_get_program_link_status(&self, program: GLTypeProgram) -> bool;

    #[must_use]
    unsafe fn gl_get_shader_compile_status(&self, shader: GLTypeShader) -> bool;

    unsafe fn gl_get_program_info_log(
        &self,
        program: GLTypeProgram
    ) -> Result<String, BacktraceError<ErrorMessage>>;

    unsafe fn gl_get_shader_info_log(
        &self,
        shader: GLTypeShader
    ) -> Result<String, BacktraceError<ErrorMessage>>;

    fn gl_check_error_always(&self) -> Result<(), BacktraceError<ErrorMessage>>
    {
        let err = unsafe { self.gl_get_error() };

        if err != GL_NO_ERROR {
            return Err(BacktraceError::<ErrorMessage>::from(GLErrorCode::from(err)));
        }

        Ok(())
    }

    fn gl_get_error_name(&self) -> Option<String>
    {
        let err = unsafe { self.gl_get_error() };

        if err != GL_NO_ERROR {
            return Some(format!("{:?}", GLErrorCode::from(err)));
        }

        None
    }

    fn gl_clear_and_log_old_error(&self)
    {
        if let Err(err) = self.gl_check_error_always() {
            log::error!("Ignoring GL error from previous command: {:?}", err);
        }
    }

    unsafe fn gl_buffer_data_f32(&self, target: GLenum, data: &[f32], usage: GLenum)
    {
        let data = std::slice::from_raw_parts(
            data.as_ptr() as *const u8,
            data.len() * std::mem::size_of::<f32>()
        );

        self.gl_buffer_data(target, data, usage)
    }

    #[allow(clippy::too_many_arguments)]
    unsafe fn gl_read_pixels(
        &self,
        x: GLint,
        y: GLint,
        width: GLsizei,
        height: GLsizei,
        format: GLenum,
        data_type: GLenum,
        data: &mut [MaybeUninit<u8>]
    );
}

pub struct GLBackendGlow
{
    context: glow::Context
}

impl GLBackendGlow
{
    #[must_use]
    pub fn new(context: glow::Context) -> Self
    {
        GLBackendGlow { context }
    }
}

impl GLBackend for GLBackendGlow
{
    unsafe fn gl_delete_program(&self, handle: GLTypeProgram)
    {
        self.context.delete_program(handle)
    }

    unsafe fn gl_delete_shader(&self, handle: GLTypeShader)
    {
        self.context.delete_shader(handle)
    }

    unsafe fn gl_delete_buffer(&self, handle: GLTypeBuffer)
    {
        self.context.delete_buffer(handle)
    }

    unsafe fn gl_delete_texture(&self, handle: GLTypeTexture)
    {
        self.context.delete_texture(handle)
    }

    unsafe fn gl_active_texture(&self, unit: GLenum)
    {
        self.context.active_texture(unit)
    }

    unsafe fn gl_bind_texture(&self, target: GLenum, handle: GLTypeTexture)
    {
        self.context.bind_texture(target, Some(handle))
    }

    unsafe fn gl_enable(&self, cap: GLenum)
    {
        self.context.enable(cap)
    }

    unsafe fn gl_disable(&self, cap: GLenum)
    {
        self.context.disable(cap)
    }

    unsafe fn gl_blend_func(&self, sfactor: GLenum, dfactor: GLenum)
    {
        self.context.blend_func(sfactor, dfactor)
    }

    unsafe fn gl_blend_func_separate(
        &self,
        sfactor: GLenum,
        dfactor: GLenum,
        sfactor_alpha: GLenum,
        dfactor_alpha: GLenum
    )
    {
        self.context
            .blend_func_separate(sfactor, dfactor, sfactor_alpha, dfactor_alpha)
    }

    unsafe fn gl_use_program(&self, handle: GLTypeProgram)
    {
        self.context.use_program(Some(handle))
    }

    unsafe fn gl_enable_vertex_attrib_array(&self, handle: GLuint)
    {
        self.context.enable_vertex_attrib_array(handle)
    }

    unsafe fn gl_disable_vertex_attrib_array(&self, handle: GLuint)
    {
        self.context.disable_vertex_attrib_array(handle)
    }

    unsafe fn gl_uniform_1f(&self, handle: &GLTypeUniformLocation, value: f32)
    {
        self.context.uniform_1_f32(Some(handle), value)
    }

    unsafe fn gl_uniform_1i(&self, handle: &GLTypeUniformLocation, value: GLint)
    {
        self.context.uniform_1_i32(Some(handle), value)
    }

    unsafe fn gl_attach_shader(&self, program: GLTypeProgram, shader: GLTypeShader)
    {
        self.context.attach_shader(program, shader)
    }

    unsafe fn gl_link_program(&self, program: GLTypeProgram)
    {
        self.context.link_program(program)
    }

    unsafe fn gl_shader_source(&self, handle: GLTypeShader, source: &str)
    {
        self.context.shader_source(handle, source)
    }

    unsafe fn gl_compile_shader(&self, handle: GLTypeShader)
    {
        self.context.compile_shader(handle)
    }

    unsafe fn gl_tex_parameter_i(&self, target: u32, parameter: u32, value: i32)
    {
        self.context.tex_parameter_i32(target, parameter, value)
    }

    unsafe fn gl_bind_buffer(&self, target: u32, handle: GLTypeBuffer)
    {
        self.context.bind_buffer(target, Some(handle))
    }

    unsafe fn gl_buffer_data(&self, target: u32, data: &[u8], usage: u32)
    {
        self.context.buffer_data_u8_slice(target, data, usage)
    }

    unsafe fn gl_draw_arrays(&self, mode: u32, first: i32, count: i32)
    {
        self.context.draw_arrays(mode, first, count)
    }

    unsafe fn gl_clear_color(&self, r: f32, g: f32, b: f32, a: f32)
    {
        self.context.clear_color(r, g, b, a)
    }

    unsafe fn gl_clear(&self, mask: u32)
    {
        self.context.clear(mask)
    }

    unsafe fn gl_enable_debug_message_callback(&self)
    {
        if !self.context.supports_debug() {
            log::info!("Context does not support debug message callbacks");
            return;
        }

        fn gl_log_callback(
            _source: GLenum,
            _gltype: GLenum,
            _id: GLuint,
            severity: GLenum,
            msg: &str
        )
        {
            match severity {
                GL_DEBUG_SEVERITY_HIGH => log::error!("GL debug log: {}", msg),
                GL_DEBUG_SEVERITY_MEDIUM => log::warn!("GL debug log: {}", msg),
                GL_DEBUG_SEVERITY_LOW => log::info!("GL debug log: {}", msg),
                _ => log::debug!("GL debug log: {}", msg)
            }
        }

        self.context.debug_message_callback(gl_log_callback);
        self.gl_enable(GL_DEBUG_OUTPUT);
        self.gl_enable(GL_DEBUG_OUTPUT_SYNCHRONOUS);

        log::info!("GL debug log enabled for glow backend");
    }

    unsafe fn gl_get_string(&self, parameter: u32) -> String
    {
        self.context.get_parameter_string(parameter)
    }

    unsafe fn gl_viewport(&self, x: i32, y: i32, width: i32, height: i32)
    {
        self.context.viewport(x, y, width, height)
    }

    unsafe fn gl_scissor(&self, x: GLint, y: GLint, width: GLsizei, height: GLsizei)
    {
        self.context.scissor(x, y, width, height);
    }

    unsafe fn gl_pixel_store_i(&self, param: u32, value: i32)
    {
        self.context.pixel_store_i32(param, value)
    }

    unsafe fn gl_vertex_attrib_pointer_f32(
        &self,
        index: u32,
        size: i32,
        data_type: u32,
        normalized: bool,
        stride: i32,
        offset: i32
    )
    {
        self.context
            .vertex_attrib_pointer_f32(index, size, data_type, normalized, stride, offset)
    }

    unsafe fn gl_tex_image_2d(
        &self,
        target: u32,
        level: i32,
        internal_format: i32,
        width: i32,
        height: i32,
        border: i32,
        format: u32,
        data_type: u32,
        pixels: Option<&[u8]>
    )
    {
        self.context.tex_image_2d(
            target,
            level,
            internal_format,
            width,
            height,
            border,
            format,
            data_type,
            pixels
        )
    }

    unsafe fn gl_tex_sub_image_2d(
        &self,
        target: u32,
        level: i32,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        format: u32,
        data_type: u32,
        pixels: &[u8]
    )
    {
        self.context.tex_sub_image_2d(
            target,
            level,
            x,
            y,
            width,
            height,
            format,
            data_type,
            glow::PixelUnpackData::Slice(pixels)
        )
    }

    unsafe fn gl_create_program(
        &self
    ) -> Result<GLTypeProgram, BacktraceError<ErrorMessage>>
    {
        let handle = self.context.create_program().map_err(|err| {
            ErrorMessage::msg(format!("Failed to create program: {err}"))
        })?;

        Ok(handle)
    }

    unsafe fn gl_create_shader(
        &self,
        shader_type: GLenum
    ) -> Result<GLTypeShader, BacktraceError<ErrorMessage>>
    {
        let handle = self.context.create_shader(shader_type).map_err(|err| {
            ErrorMessage::msg(format!("Failed to create shader: {err}"))
        })?;

        Ok(handle)
    }

    unsafe fn gl_gen_buffer(&self) -> Result<GLTypeBuffer, BacktraceError<ErrorMessage>>
    {
        let handle = self.context.create_buffer().map_err(|err| {
            ErrorMessage::msg(format!("Failed to create buffer: {err}"))
        })?;

        Ok(handle)
    }

    unsafe fn gl_gen_texture(&self)
        -> Result<GLTypeTexture, BacktraceError<ErrorMessage>>
    {
        let handle = self.context.create_texture().map_err(|err| {
            ErrorMessage::msg(format!("Failed to create texture: {err}"))
        })?;

        Ok(handle)
    }

    unsafe fn gl_get_error(&self) -> GLenum
    {
        self.context.get_error()
    }

    unsafe fn gl_get_attrib_location(
        &self,
        program: GLTypeProgram,
        name: &str
    ) -> Option<GLuint>
    {
        self.context.get_attrib_location(program, name)
    }

    unsafe fn gl_get_uniform_location(
        &self,
        program: GLTypeProgram,
        name: &str
    ) -> Option<GLTypeUniformLocation>
    {
        self.context.get_uniform_location(program, name)
    }

    unsafe fn gl_get_program_link_status(&self, program: GLTypeProgram) -> bool
    {
        self.context.get_program_link_status(program)
    }

    unsafe fn gl_get_shader_compile_status(&self, shader: GLTypeShader) -> bool
    {
        self.context.get_shader_compile_status(shader)
    }

    unsafe fn gl_get_program_info_log(
        &self,
        program: GLTypeProgram
    ) -> Result<String, BacktraceError<ErrorMessage>>
    {
        Ok(self.context.get_program_info_log(program))
    }

    unsafe fn gl_get_shader_info_log(
        &self,
        shader: GLTypeShader
    ) -> Result<String, BacktraceError<ErrorMessage>>
    {
        Ok(self.context.get_shader_info_log(shader))
    }

    unsafe fn gl_read_pixels(
        &self,
        x: GLint,
        y: GLint,
        width: GLsizei,
        height: GLsizei,
        format: GLenum,
        data_type: GLenum,
        data: &mut [MaybeUninit<u8>]
    )
    {
        let data =
            std::slice::from_raw_parts_mut(data.as_mut_ptr() as *mut u8, data.len());

        self.context.read_pixels(
            x,
            y,
            width,
            height,
            format,
            data_type,
            PixelPackData::Slice(data)
        )
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub struct GLBackendGLRS {}

#[cfg(not(target_arch = "wasm32"))]
impl GLBackendGLRS
{
    #[allow(dead_code)]
    pub fn new() -> Self
    {
        GLBackendGLRS {}
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl GLBackend for GLBackendGLRS
{
    unsafe fn gl_delete_program(&self, handle: u32)
    {
        gl::DeleteProgram(handle)
    }

    unsafe fn gl_delete_shader(&self, handle: u32)
    {
        gl::DeleteShader(handle)
    }

    unsafe fn gl_delete_buffer(&self, handle: u32)
    {
        gl::DeleteBuffers(1, &handle)
    }

    unsafe fn gl_delete_texture(&self, handle: u32)
    {
        gl::DeleteTextures(1, &handle)
    }

    unsafe fn gl_active_texture(&self, texture: GLenum)
    {
        gl::ActiveTexture(texture);
    }

    unsafe fn gl_bind_texture(&self, target: u32, handle: u32)
    {
        gl::BindTexture(target, handle);
    }

    unsafe fn gl_enable(&self, cap: u32)
    {
        gl::Enable(cap)
    }

    unsafe fn gl_disable(&self, cap: u32)
    {
        gl::Disable(cap)
    }

    unsafe fn gl_blend_func(&self, sfactor: u32, dfactor: u32)
    {
        gl::BlendFunc(sfactor, dfactor)
    }

    unsafe fn gl_blend_func_separate(
        &self,
        sfactor: GLenum,
        dfactor: GLenum,
        sfactor_alpha: GLenum,
        dfactor_alpha: GLenum
    )
    {
        gl::BlendFuncSeparate(sfactor, dfactor, sfactor_alpha, dfactor_alpha)
    }

    unsafe fn gl_use_program(&self, handle: u32)
    {
        gl::UseProgram(handle)
    }

    unsafe fn gl_enable_vertex_attrib_array(&self, handle: u32)
    {
        gl::EnableVertexAttribArray(handle)
    }

    unsafe fn gl_disable_vertex_attrib_array(&self, handle: u32)
    {
        gl::DisableVertexAttribArray(handle)
    }

    unsafe fn gl_uniform_1f(&self, handle: &u32, value: f32)
    {
        gl::Uniform1f(*handle as i32, value)
    }

    unsafe fn gl_uniform_1i(&self, handle: &u32, value: i32)
    {
        gl::Uniform1i(*handle as i32, value)
    }

    unsafe fn gl_attach_shader(&self, program: u32, shader: u32)
    {
        gl::AttachShader(program, shader)
    }

    unsafe fn gl_link_program(&self, program: u32)
    {
        gl::LinkProgram(program)
    }

    unsafe fn gl_shader_source(&self, handle: u32, source: &str)
    {
        let source_str: *const GLchar = source.as_ptr() as *const GLchar;
        let source_len: i32 = source.len().try_into().unwrap();

        gl::ShaderSource(handle, 1, &source_str, &source_len);
    }

    unsafe fn gl_compile_shader(&self, handle: u32)
    {
        gl::CompileShader(handle)
    }

    unsafe fn gl_tex_parameter_i(&self, target: u32, parameter: u32, value: i32)
    {
        gl::TexParameteri(target, parameter, value)
    }

    unsafe fn gl_bind_buffer(&self, target: u32, handle: u32)
    {
        gl::BindBuffer(target, handle)
    }

    unsafe fn gl_buffer_data(&self, target: u32, data: &[u8], usage: u32)
    {
        gl::BufferData(
            target,
            data.len().try_into().unwrap(),
            data.as_ptr() as *const c_void,
            usage
        )
    }

    unsafe fn gl_draw_arrays(&self, mode: u32, first: i32, count: i32)
    {
        gl::DrawArrays(mode, first, count)
    }

    unsafe fn gl_clear_color(&self, r: f32, g: f32, b: f32, a: f32)
    {
        gl::ClearColor(r, g, b, a)
    }

    unsafe fn gl_clear(&self, mask: u32)
    {
        gl::Clear(mask)
    }

    unsafe fn gl_enable_debug_message_callback(&self)
    {
        if !gl::DebugMessageCallback::is_loaded() {
            log::error!("Cannot register GL debug log: function not loaded");
            return;
        }

        extern "system" fn gl_log_callback(
            _source: GLenum,
            _gltype: GLenum,
            _id: GLuint,
            severity: GLenum,
            length: GLsizei,
            message: *const GLchar,
            _user_param: *mut c_void
        )
        {
            let msg = if length < 0 {
                unsafe { String::from_utf8_lossy(CStr::from_ptr(message).to_bytes()) }
            } else {
                unsafe {
                    String::from_utf8_lossy(std::slice::from_raw_parts(
                        message as *const u8,
                        length as usize
                    ))
                }
            };

            match severity {
                GL_DEBUG_SEVERITY_HIGH => log::error!("GL debug log: {}", msg),
                GL_DEBUG_SEVERITY_MEDIUM => log::warn!("GL debug log: {}", msg),
                GL_DEBUG_SEVERITY_LOW => log::info!("GL debug log: {}", msg),
                _ => log::debug!("GL debug log: {}", msg)
            }
        }

        gl::DebugMessageCallback(Some(gl_log_callback), std::ptr::null());
        gl::Enable(GL_DEBUG_OUTPUT);
        gl::Enable(GL_DEBUG_OUTPUT_SYNCHRONOUS);

        log::info!("GL debug log enabled for gl backend");
    }

    unsafe fn gl_get_string(&self, parameter: u32) -> String
    {
        String::from_utf8_lossy(
            CStr::from_ptr(gl::GetString(parameter) as *const _).to_bytes()
        )
        .to_string()
    }

    unsafe fn gl_viewport(&self, x: i32, y: i32, width: i32, height: i32)
    {
        gl::Viewport(x, y, width, height)
    }

    unsafe fn gl_scissor(&self, x: GLint, y: GLint, width: GLsizei, height: GLsizei)
    {
        gl::Scissor(x, y, width, height);
    }

    unsafe fn gl_pixel_store_i(&self, param: u32, value: i32)
    {
        gl::PixelStorei(param, value)
    }

    unsafe fn gl_vertex_attrib_pointer_f32(
        &self,
        index: u32,
        size: i32,
        data_type: u32,
        normalized: bool,
        stride: i32,
        offset: i32
    )
    {
        gl::VertexAttribPointer(
            index,
            size,
            data_type,
            if normalized { GL_TRUE } else { GL_FALSE },
            stride,
            offset as *const c_void
        );
    }

    unsafe fn gl_tex_image_2d(
        &self,
        target: u32,
        level: i32,
        internal_format: i32,
        width: i32,
        height: i32,
        border: i32,
        format: u32,
        data_type: u32,
        pixels: Option<&[u8]>
    )
    {
        gl::TexImage2D(
            target,
            level,
            internal_format,
            width,
            height,
            border,
            format,
            data_type,
            match pixels {
                None => std::ptr::null(),
                Some(pixels) => pixels.as_ptr() as *const c_void
            }
        );
    }

    unsafe fn gl_tex_sub_image_2d(
        &self,
        target: u32,
        level: i32,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        format: u32,
        data_type: u32,
        pixels: &[u8]
    )
    {
        gl::TexSubImage2D(
            target,
            level,
            x,
            y,
            width,
            height,
            format,
            data_type,
            pixels.as_ptr() as *const c_void
        )
    }

    unsafe fn gl_create_program(&self) -> Result<GLuint, BacktraceError<ErrorMessage>>
    {
        let handle = gl::CreateProgram();

        if handle == 0 {
            Err(ErrorMessage::msg("Got program with zero handle"))
        } else {
            Ok(handle)
        }
    }

    unsafe fn gl_create_shader(
        &self,
        shader_type: GLenum
    ) -> Result<GLuint, BacktraceError<ErrorMessage>>
    {
        let handle = gl::CreateShader(shader_type);

        if handle == 0 {
            Err(ErrorMessage::msg("Got shader with zero handle"))
        } else {
            Ok(handle)
        }
    }

    unsafe fn gl_gen_buffer(&self) -> Result<u32, BacktraceError<ErrorMessage>>
    {
        let mut handle: GLuint = 0;
        gl::GenBuffers(1, &mut handle);

        if handle == 0 {
            Err(ErrorMessage::msg("Got buffer with zero handle"))
        } else {
            Ok(handle)
        }
    }

    unsafe fn gl_gen_texture(&self) -> Result<u32, BacktraceError<ErrorMessage>>
    {
        let mut handle: GLuint = 0;
        gl::GenTextures(1, &mut handle);

        if handle == 0 {
            Err(ErrorMessage::msg("Got texture with zero handle"))
        } else {
            Ok(handle)
        }
    }

    unsafe fn gl_get_error(&self) -> GLenum
    {
        gl::GetError()
    }

    unsafe fn gl_get_attrib_location(&self, program: u32, name: &str) -> Option<u32>
    {
        let name_cstr = match std::ffi::CString::new(name) {
            Ok(name_cstr) => name_cstr,
            Err(_) => return None
        };

        let result = gl::GetAttribLocation(program, name_cstr.as_ptr());

        if result < 0 {
            None
        } else {
            Some(result as u32)
        }
    }

    unsafe fn gl_get_uniform_location(&self, program: u32, name: &str) -> Option<u32>
    {
        let name_cstr = match std::ffi::CString::new(name) {
            Ok(name_cstr) => name_cstr,
            Err(_) => return None
        };

        let result = gl::GetUniformLocation(program, name_cstr.as_ptr());

        if result < 0 {
            None
        } else {
            Some(result as u32)
        }
    }

    unsafe fn gl_get_program_link_status(&self, program: u32) -> bool
    {
        let mut link_status: GLint = 0;
        gl::GetProgramiv(program, GL_LINK_STATUS, &mut link_status);

        link_status == 1
    }

    unsafe fn gl_get_shader_compile_status(&self, shader: u32) -> bool
    {
        let mut compile_status: GLint = 0;
        gl::GetShaderiv(shader, GL_COMPILE_STATUS, &mut compile_status);

        compile_status == 1
    }

    unsafe fn gl_get_program_info_log(
        &self,
        program: u32
    ) -> Result<String, BacktraceError<ErrorMessage>>
    {
        self.gl_call_get_info_log(|buf_capacity, out_buf_len, buf| {
            gl::GetProgramInfoLog(program, buf_capacity, out_buf_len, buf);
            self.gl_check_error_always()
        })
    }

    unsafe fn gl_get_shader_info_log(
        &self,
        shader: u32
    ) -> Result<String, BacktraceError<ErrorMessage>>
    {
        self.gl_call_get_info_log(|buf_capacity, out_buf_len, buf| {
            gl::GetShaderInfoLog(shader, buf_capacity, out_buf_len, buf);
            self.gl_check_error_always()
        })
    }

    unsafe fn gl_read_pixels(
        &self,
        x: GLint,
        y: GLint,
        width: GLsizei,
        height: GLsizei,
        format: GLenum,
        data_type: GLenum,
        data: &mut [MaybeUninit<u8>]
    )
    {
        gl::ReadPixels(
            x,
            y,
            width,
            height,
            format,
            data_type,
            data.as_mut_ptr() as *mut c_void
        )
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl GLBackendGLRS
{
    fn gl_call_get_info_log<F>(
        &self,
        callback: F
    ) -> Result<String, BacktraceError<ErrorMessage>>
    where
        F: FnOnce(
            GLsizei,
            *mut GLsizei,
            *mut GLchar
        ) -> Result<(), BacktraceError<ErrorMessage>>
    {
        self.gl_clear_and_log_old_error();

        let log_buf_capacity: GLsizei = 16384;
        let mut log_buf: Vec<u8> = vec![0; log_buf_capacity as usize];

        let mut log_buf_length: GLsizei = -1;

        callback(
            log_buf_capacity,
            &mut log_buf_length,
            log_buf.as_mut_ptr() as *mut GLchar
        )?;

        self.gl_clear_and_log_old_error();

        if log_buf_length < 0 || log_buf_length > log_buf_capacity {
            return Err(ErrorMessage::msg(format!(
                "GL info log failed, log had invalid length {log_buf_length}"
            )));
        }

        unsafe { log_buf.set_len(log_buf_length as usize) };

        let msg = String::from_utf8_lossy(log_buf.as_slice());

        Ok(String::from(msg))
    }
}
