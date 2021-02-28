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
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::num::TryFromIntError;
use std::rc::{Rc, Weak};

use gl::types::*;

use crate::color::Color;
use crate::dimen::Vector2;
use crate::error::BacktraceError;

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
            gl::INVALID_ENUM => GLErrorCode::InvalidEnum,
            gl::INVALID_VALUE => GLErrorCode::InvalidValue,
            gl::INVALID_OPERATION => GLErrorCode::InvalidOperation,
            gl::INVALID_FRAMEBUFFER_OPERATION => GLErrorCode::InvalidFramebufferOperation,
            gl::OUT_OF_MEMORY => GLErrorCode::OutOfMemory,
            gl::STACK_UNDERFLOW => GLErrorCode::StackUnderflow,
            gl::STACK_OVERFLOW => GLErrorCode::StackOverflow,
            _ => GLErrorCode::Other(constant)
        }
    }
}

#[derive(Debug, Clone)]
pub struct GLError
{
    description: String
}

impl Display for GLError
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result
    {
        Display::fmt("GL error: ", f)?;
        Display::fmt(&self.description, f)
    }
}

impl From<GLErrorCode> for BacktraceError<GLError>
{
    fn from(err: GLErrorCode) -> Self
    {
        GLError::msg(format!("Got GL error code {:?}", err))
    }
}

impl From<TryFromIntError> for BacktraceError<GLError>
{
    fn from(_: TryFromIntError) -> Self
    {
        GLError::msg("Integer conversion failed/out of bounds")
    }
}

impl Error for GLError {}

impl GLError
{
    fn for_zero_handle(alloc_type: GLHandleType) -> BacktraceError<Self>
    {
        GLError::msg(format!("GL allocation returned zero for {:?}", alloc_type))
    }

    fn msg<S: Into<String>>(description: S) -> BacktraceError<Self>
    {
        BacktraceError::new(GLError {
            description: description.into()
        })
    }
}

fn gl_check_error_always() -> Result<(), BacktraceError<GLError>>
{
    let err = unsafe { gl::GetError() };

    if err != gl::NO_ERROR {
        return Err(BacktraceError::<GLError>::from(GLErrorCode::from(err)));
    }

    Ok(())
}

fn gl_clear_and_log_old_error()
{
    if let Err(err) = gl_check_error_always() {
        log::error!("Ignoring GL error from previous command: {:?}", err);
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
enum GLHandleType
{
    Program,
    Shader,
    Buffer,
    Texture
}

struct GLHandle
{
    context: Weak<RefCell<GLContextManagerState>>,
    handle: GLuint,
    handle_type: GLHandleType
}

impl Debug for GLHandle
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result
    {
        f.debug_struct("GLHandle")
            .field("handle", &self.handle)
            .field("handle_type", &self.handle_type)
            .finish()
    }
}

impl std::hash::Hash for GLHandle
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H)
    {
        self.handle.hash(state);
        self.handle_type.hash(state);
    }
}

impl PartialEq for GLHandle
{
    fn eq(&self, other: &Self) -> bool
    {
        match self.context.upgrade() {
            None => return false,
            Some(self_context) => match other.context.upgrade() {
                None => return false,
                Some(other_context) => {
                    if *self_context.borrow() != *other_context.borrow() {
                        return false;
                    }
                }
            }
        }

        self.handle == other.handle && self.handle_type == other.handle_type
    }
}

impl Eq for GLHandle {}

impl GLHandle
{
    fn wrap<F: FnOnce() -> GLuint>(
        context: &Rc<RefCell<GLContextManagerState>>,
        handle_type: GLHandleType,
        handle_creator: F
    ) -> Result<Self, BacktraceError<GLError>>
    {
        match handle_type {
            GLHandleType::Program => gl_clear_and_log_old_error(),
            GLHandleType::Shader => gl_clear_and_log_old_error(),
            GLHandleType::Buffer => {}
            GLHandleType::Texture => {}
        }

        let handle = handle_creator();

        match handle_type {
            GLHandleType::Program => gl_check_error_always()?,
            GLHandleType::Shader => gl_check_error_always()?,
            GLHandleType::Buffer => {}
            GLHandleType::Texture => {}
        }

        if handle == 0 {
            return Err(GLError::for_zero_handle(handle_type));
        }

        Ok(GLHandle {
            context: Rc::downgrade(&context),
            handle,
            handle_type
        })
    }

    #[inline]
    #[must_use]
    fn is_context_still_valid(&self) -> bool
    {
        is_gl_context_valid_weak(&self.context)
    }
}

impl Drop for GLHandle
{
    fn drop(&mut self)
    {
        if !self.is_context_still_valid() {
            // No need to drop, the context is gone
            return;
        }

        match self.handle_type {
            GLHandleType::Program => {
                unsafe { gl::DeleteProgram(self.handle) };
            }

            GLHandleType::Shader => {
                unsafe { gl::DeleteShader(self.handle) };
            }

            GLHandleType::Buffer => {
                unsafe { gl::DeleteBuffers(1, &self.handle) };
            }

            GLHandleType::Texture => {
                unsafe { gl::DeleteTextures(1, &self.handle) };
            }
        }
    }
}

#[derive(Debug)]
pub struct GLProgram
{
    handle: GLHandle,
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
        std::ptr::eq(self, other)
    }
}

impl Eq for GLProgram {}

impl GLProgram
{
    fn new(
        context: &Rc<RefCell<GLContextManagerState>>
    ) -> Result<Self, BacktraceError<GLError>>
    {
        Ok(GLProgram {
            handle: GLHandle::wrap(context, GLHandleType::Program, || unsafe {
                gl::CreateProgram()
            })?,
            attribute_handles: HashMap::new()
        })
    }

    fn attach_shader(&mut self, shader: &GLShader)
        -> Result<(), BacktraceError<GLError>>
    {
        unsafe {
            gl::AttachShader(self.handle.handle, shader.handle.handle);
        }

        gl_check_error_always()?;

        Ok(())
    }

    fn link(
        context: &Rc<RefCell<GLContextManagerState>>,
        vertex_shader: &GLShader,
        fragment_shader: &GLShader,
        attribute_names: impl IntoIterator<Item = &'static &'static str>
    ) -> Result<Self, BacktraceError<GLError>>
    {
        gl_clear_and_log_old_error();

        let mut program = GLProgram::new(context)?;

        program.attach_shader(vertex_shader)?;
        program.attach_shader(fragment_shader)?;

        unsafe {
            gl::LinkProgram(program.handle.handle);
        }

        gl_check_error_always()?;

        let mut link_status: GLint = 0;

        unsafe {
            gl::GetProgramiv(program.handle.handle, gl::LINK_STATUS, &mut link_status);
        }

        if link_status == 0 {
            let msg = gl_get_program_info_log(&program)?;
            return Err(GLError::msg(format!("Program linking failed: '{}'", msg)));
        }

        unsafe {
            gl::ValidateProgram(program.handle.handle);
        }

        gl_check_error_always()?;

        {
            let validate_msg = gl_get_program_info_log(&program)?;

            if !validate_msg.is_empty() {
                return Err(GLError::msg(format!(
                    "Program validate log was not empty: '{}'",
                    validate_msg
                )));
            }
        }

        for attribute_name in attribute_names.into_iter() {
            program.attribute_handles.insert(
                attribute_name.as_ref(),
                program.get_attribute_handle(attribute_name.as_ref())?
            );
        }

        Ok(program)
    }

    fn enable(&self)
    {
        unsafe {
            gl::UseProgram(self.handle.handle);
        }

        for attribute in self.attribute_handles.values() {
            unsafe {
                gl::EnableVertexAttribArray(attribute.handle);
            }
        }
    }

    fn disable(&self)
    {
        for attribute in self.attribute_handles.values() {
            unsafe {
                gl::DisableVertexAttribArray(attribute.handle);
            }
        }
    }

    pub fn get_attribute_handle(
        &self,
        name: &str
    ) -> Result<GLAttributeHandle, BacktraceError<GLError>>
    {
        if !is_gl_context_valid_weak(&self.handle.context) {
            return Err(GLError::msg("GL context no longer valid"));
        }

        let name_cstr = std::ffi::CString::new(name)
            .map_err(|_| GLError::msg("Attribute name contained NUL"))?;

        let handle =
            unsafe { gl::GetAttribLocation(self.handle.handle, name_cstr.as_ptr()) };

        gl_check_error_always()?;

        if handle < 0 {
            return Err(GLError::msg(format!(
                "Attribute handle {} is invalid",
                name
            )));
        }

        let handle: u32 = handle.try_into()?;

        Ok(GLAttributeHandle { handle })
    }

    pub fn get_uniform_handle(
        &self,
        name: &str
    ) -> Result<GLUniformHandle, BacktraceError<GLError>>
    {
        if !is_gl_context_valid_weak(&self.handle.context) {
            return Err(GLError::msg("GL context no longer valid"));
        }

        let name_cstr = std::ffi::CString::new(name)
            .map_err(|_| GLError::msg("Uniform name contained NUL"))?;

        let handle =
            unsafe { gl::GetUniformLocation(self.handle.handle, name_cstr.as_ptr()) };

        gl_check_error_always()?;

        if handle < 0 {
            return Err(GLError::msg(format!(
                "Attribute handle {} is invalid",
                name
            )));
        }

        Ok(GLUniformHandle { handle })
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
            GLShaderType::Vertex => gl::VERTEX_SHADER,
            GLShaderType::Fragment => gl::FRAGMENT_SHADER
        }
    }
}

pub struct GLShader
{
    handle: GLHandle
}

impl GLShader
{
    fn new(
        context: &Rc<RefCell<GLContextManagerState>>,
        shader_type: GLShaderType
    ) -> Result<Self, BacktraceError<GLError>>
    {
        Ok(GLShader {
            handle: GLHandle::wrap(context, GLHandleType::Shader, || unsafe {
                gl::CreateShader(shader_type.gl_constant())
            })?
        })
    }

    fn compile(
        context: &Rc<RefCell<GLContextManagerState>>,
        shader_type: GLShaderType,
        source: &str
    ) -> Result<Self, BacktraceError<GLError>>
    {
        gl_clear_and_log_old_error();

        let shader = GLShader::new(context, shader_type)?;

        let source_str: *const GLchar = source.as_ptr() as *const GLchar;
        let source_len: i32 = source.len().try_into()?;

        unsafe { gl::ShaderSource(shader.handle.handle, 1, &source_str, &source_len) };

        gl_check_error_always()?;

        unsafe { gl::CompileShader(shader.handle.handle) };

        gl_check_error_always()?;

        let mut compile_status: GLint = 0;

        unsafe {
            gl::GetShaderiv(
                shader.handle.handle,
                gl::COMPILE_STATUS,
                &mut compile_status
            );
        };

        if compile_status == 0 {
            let msg = gl_call_get_info_log(|buf_capacity, out_buf_len, buf| unsafe {
                gl::GetShaderInfoLog(
                    shader.handle.handle,
                    buf_capacity,
                    out_buf_len,
                    buf
                );

                gl_check_error_always()?;
                Ok(())
            })?;

            return Err(GLError::msg(format!(
                "Shader compilation failed: '{}'",
                msg
            )));
        }

        Ok(shader)
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
    handle: GLint
}

impl GLUniformHandle
{
    pub fn set_value_float(&self, value: f32)
    {
        unsafe {
            gl::Uniform1f(self.handle, value);
        }
    }

    pub fn set_value_int(&self, value: i32)
    {
        unsafe {
            gl::Uniform1i(self.handle, value);
        }
    }
}

fn gl_get_program_info_log(program: &GLProgram)
    -> Result<String, BacktraceError<GLError>>
{
    gl_call_get_info_log(|buf_capacity, out_buf_len, buf| unsafe {
        gl::GetProgramInfoLog(program.handle.handle, buf_capacity, out_buf_len, buf);

        gl_check_error_always()?;
        Ok(())
    })
}

fn gl_call_get_info_log<F>(callback: F) -> Result<String, BacktraceError<GLError>>
where
    F: FnOnce(GLsizei, *mut GLsizei, *mut GLchar) -> Result<(), BacktraceError<GLError>>
{
    gl_clear_and_log_old_error();

    let log_buf_capacity: GLsizei = 16384;
    let mut log_buf: Vec<u8> = vec![0; log_buf_capacity as usize];

    let mut log_buf_length: GLsizei = -1;

    callback(
        log_buf_capacity,
        &mut log_buf_length,
        log_buf.as_mut_ptr() as *mut GLchar
    )?;

    gl_clear_and_log_old_error();

    if log_buf_length < 0 || log_buf_length > log_buf_capacity {
        return Err(GLError::msg(format!(
            "GL info log failed, log had invalid length {}",
            log_buf_length
        )));
    }

    unsafe { log_buf.set_len(log_buf_length as usize) };

    let msg = String::from_utf8_lossy(log_buf.as_slice());

    Ok(String::from(msg))
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
            GLBufferTarget::Array => gl::ARRAY_BUFFER,
            GLBufferTarget::ElementArray => gl::ELEMENT_ARRAY_BUFFER
        }
    }
}

pub struct GLBuffer
{
    handle: GLHandle,
    target: GLBufferTarget,
    components_per_vertex: GLint,
    attrib_index: GLAttributeHandle
}

impl GLBuffer
{
    fn new(
        context: &Rc<RefCell<GLContextManagerState>>,
        target: GLBufferTarget,
        components_per_vertex: GLint,
        attrib_index: GLAttributeHandle
    ) -> Result<Self, BacktraceError<GLError>>
    {
        gl_clear_and_log_old_error();

        let handle = GLHandle::wrap(context, GLHandleType::Buffer, || unsafe {
            let mut handle: GLuint = 0;
            gl::GenBuffers(1, &mut handle);
            handle
        })?;

        Ok(GLBuffer {
            handle,
            target,
            components_per_vertex,
            attrib_index
        })
    }

    pub fn set_data(&mut self, data: &[GLfloat])
    {
        if !is_gl_context_valid_weak(&self.handle.context) {
            log::warn!("Ignoring buffer set_data: invalid GL context");
            return;
        }

        self.gl_bind_buffer();
        self.gl_buffer_data_dynamic_draw(data);
        self.gl_set_attrib_pointer_data();
    }

    fn gl_bind_buffer(&mut self)
    {
        unsafe {
            gl::BindBuffer(self.target.gl_constant(), self.handle.handle);
        }
    }

    fn gl_buffer_data_dynamic_draw(&mut self, data: &[GLfloat])
    {
        unsafe {
            gl::BufferData(
                self.target.gl_constant(),
                (data.len() * std::mem::size_of::<GLfloat>())
                    .try_into()
                    .unwrap(),
                data.as_ptr() as *const GLvoid,
                gl::DYNAMIC_DRAW
            );
        }
    }

    fn gl_set_attrib_pointer_data(&mut self)
    {
        unsafe {
            gl::VertexAttribPointer(
                self.attrib_index.handle,
                self.components_per_vertex,
                gl::FLOAT,
                gl::FALSE,
                0,
                std::ptr::null()
            );
        }
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub enum GLTextureSmoothing
{
    NearestNeighbour,
    Linear
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub enum GLTextureImageFormatU8
{
    #[allow(dead_code)]
    Red,
    RGB,
    RGBA
}

impl GLTextureImageFormatU8
{
    fn get_internal_format(&self) -> GLenum
    {
        match self {
            GLTextureImageFormatU8::Red => gl::R8,
            GLTextureImageFormatU8::RGB => gl::RGB8,
            GLTextureImageFormatU8::RGBA => gl::RGBA8
        }
    }

    fn get_format(&self) -> GLenum
    {
        match self {
            GLTextureImageFormatU8::Red => gl::RED,
            GLTextureImageFormatU8::RGB => gl::RGB,
            GLTextureImageFormatU8::RGBA => gl::RGBA
        }
    }
}

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct GLTexture
{
    handle: GLHandle
}

impl GLTexture
{
    fn new(
        context: &Rc<RefCell<GLContextManagerState>>
    ) -> Result<Rc<Self>, BacktraceError<GLError>>
    {
        let handle = GLHandle::wrap(context, GLHandleType::Texture, || unsafe {
            let mut handle: GLuint = 0;
            gl::GenTextures(1, &mut handle);
            handle
        })?;

        Ok(Rc::new(GLTexture { handle }))
    }

    pub fn set_image_data(
        self: &Rc<Self>,
        context: &Rc<GLContextManager>,
        format: GLTextureImageFormatU8,
        smoothing: GLTextureSmoothing,
        size: &Vector2<u32>,
        data: &[u8]
    ) -> Result<(), BacktraceError<GLError>>
    {
        if !is_gl_context_valid_weak(&self.handle.context) {
            log::warn!("Ignoring texture set_image_data: invalid GL context");
            return Ok(());
        }

        let smoothing_constant = match smoothing {
            GLTextureSmoothing::NearestNeighbour => gl::NEAREST,
            GLTextureSmoothing::Linear => gl::LINEAR
        } as GLint;

        context.bind_texture(self);

        unsafe {
            gl::BindTexture(gl::TEXTURE_2D, self.handle.handle);
            gl::TexParameteri(
                gl::TEXTURE_2D,
                gl::TEXTURE_WRAP_S,
                gl::CLAMP_TO_EDGE as GLint
            );
            gl::TexParameteri(
                gl::TEXTURE_2D,
                gl::TEXTURE_WRAP_T,
                gl::CLAMP_TO_EDGE as GLint
            );
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, smoothing_constant);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, smoothing_constant);
        }

        unsafe {
            gl::TexImage2D(
                gl::TEXTURE_2D,
                0,
                format.get_internal_format().try_into()?,
                size.x.try_into()?,
                size.y.try_into()?,
                0,
                format.get_format(),
                gl::UNSIGNED_BYTE,
                data.as_ptr() as *const std::os::raw::c_void
            );
        }

        Ok(())
    }

    #[allow(dead_code)]
    pub fn set_sub_image_data(
        &self,
        format: GLTextureImageFormatU8,
        x: usize,
        y: usize,
        width: usize,
        height: usize,
        data: &[u8]
    ) -> Result<(), BacktraceError<GLError>>
    {
        if !is_gl_context_valid_weak(&self.handle.context) {
            log::warn!("Ignoring texture set_sub_image_data: invalid GL context");
            return Ok(());
        }

        unsafe {
            gl::TexSubImage2D(
                gl::TEXTURE_2D,
                0,
                x.try_into()?,
                y.try_into()?,
                width.try_into()?,
                height.try_into()?,
                format.get_format(),
                gl::UNSIGNED_BYTE,
                data.as_ptr() as *const std::os::raw::c_void
            );
        }

        Ok(())
    }

    fn unbind_texture()
    {
        unsafe {
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, 0);
        }
    }
}

#[must_use]
fn is_gl_context_valid(state: &RefCell<GLContextManagerState>) -> bool
{
    state.borrow().is_valid
}

#[inline]
#[must_use]
fn is_gl_context_valid_weak(state: &Weak<RefCell<GLContextManagerState>>) -> bool
{
    match state.upgrade() {
        None => false,
        Some(state) => is_gl_context_valid(&state)
    }
}

#[derive(Eq)]
struct GLContextManagerState
{
    is_valid: bool,
    active_texture: Option<Rc<GLTexture>>,
    active_program: Option<Rc<GLProgram>>,
    active_blend_mode: Option<GLBlendEnabled>
}

impl PartialEq for GLContextManagerState
{
    fn eq(&self, other: &Self) -> bool
    {
        std::ptr::eq(self, other)
    }
}

pub struct GLContextManager
{
    state: Rc<RefCell<GLContextManagerState>>
}

impl GLContextManager
{
    pub fn create() -> Result<Rc<Self>, BacktraceError<GLError>>
    {
        let manager = Rc::new(GLContextManager {
            state: Rc::new(RefCell::new(GLContextManagerState {
                is_valid: true,
                active_texture: None,
                active_program: None,
                active_blend_mode: None
            }))
        });

        log::info!("GL context manager created");

        Ok(manager)
    }

    pub fn mark_invalid(&self)
    {
        log::info!("GL context manager is now inactive");
        self.state.borrow_mut().is_valid = false;
    }

    pub fn new_buffer(
        &self,
        target: GLBufferTarget,
        components_per_vertex: GLint,
        attrib_index: GLAttributeHandle
    ) -> Result<GLBuffer, BacktraceError<GLError>>
    {
        if !is_gl_context_valid(&self.state) {
            return Err(GLError::msg("GL context no longer valid"));
        }

        GLBuffer::new(&self.state, target, components_per_vertex, attrib_index)
    }

    pub fn new_shader(
        &self,
        shader_type: GLShaderType,
        source: &str
    ) -> Result<GLShader, BacktraceError<GLError>>
    {
        if !is_gl_context_valid(&self.state) {
            return Err(GLError::msg("GL context no longer valid"));
        }

        GLShader::compile(&self.state, shader_type, source)
    }

    pub fn new_program(
        &self,
        vertex_shader: &GLShader,
        fragment_shader: &GLShader,
        attribute_names: impl IntoIterator<Item = &'static &'static str>
    ) -> Result<Rc<GLProgram>, BacktraceError<GLError>>
    {
        if !is_gl_context_valid(&self.state) {
            return Err(GLError::msg("GL context no longer valid"));
        }

        Ok(Rc::new(GLProgram::link(
            &self.state,
            vertex_shader,
            fragment_shader,
            attribute_names
        )?))
    }

    pub fn new_texture(&self) -> Result<Rc<GLTexture>, BacktraceError<GLError>>
    {
        if !is_gl_context_valid(&self.state) {
            return Err(GLError::msg("GL context no longer valid"));
        }

        GLTexture::new(&self.state)
    }

    pub fn bind_texture(&self, texture: &Rc<GLTexture>)
    {
        if !is_gl_context_valid(&self.state) {
            log::warn!("Ignoring bind_texture: invalid GL context");
            return;
        }

        if self.state.borrow().active_texture.as_ref() == Some(texture) {
            // Already bound
            return;
        }

        // Drop separately to avoid a duplicate borrow of `state`.
        let old_active_texture = self.state.borrow_mut().active_texture.take();
        std::mem::drop(old_active_texture);

        self.state.borrow_mut().active_texture = Some(texture.clone());

        unsafe {
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, texture.handle.handle);
        }
    }

    pub fn unbind_texture(&self)
    {
        if !is_gl_context_valid(&self.state) {
            log::warn!("Ignoring unbind_texture: invalid GL context");
            return;
        }

        GLTexture::unbind_texture();
    }

    pub fn use_program(&self, program: &Rc<GLProgram>)
    {
        if !is_gl_context_valid(&self.state) {
            log::warn!("Ignoring use_program: invalid GL context");
            return;
        }

        if self.state.borrow().active_program.as_ref() == Some(program) {
            // Already bound
            return;
        }

        if let Some(existing_program) = &self.state.borrow_mut().active_program {
            existing_program.disable();
        }

        self.state.borrow_mut().active_program = Some(program.clone());
        program.enable();
    }

    fn set_blend_mode(&self, blend_mode: GLBlendEnabled)
    {
        if self.state.borrow().active_blend_mode == Some(blend_mode.clone()) {
            return;
        }

        self.state.borrow_mut().active_blend_mode = Some(blend_mode.clone());

        match blend_mode {
            GLBlendEnabled::Enabled(mode) => match mode {
                GLBlendMode::OneMinusSrcAlpha => unsafe {
                    gl::Enable(gl::BLEND);
                    gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
                }
            },

            GLBlendEnabled::Disabled => unsafe {
                gl::Disable(gl::BLEND);
            }
        }
    }

    pub fn draw_triangles(&self, blend_mode: GLBlendEnabled, vertex_count: usize)
    {
        if !is_gl_context_valid(&self.state) {
            log::warn!("Ignoring draw_triangles: invalid GL context");
            return;
        }

        self.set_blend_mode(blend_mode);

        unsafe {
            gl::DrawArrays(gl::TRIANGLES, 0, vertex_count.try_into().unwrap());
        }
    }

    pub fn clear_screen(&self, color: Color)
    {
        if !is_gl_context_valid(&self.state) {
            log::warn!("Ignoring clear_screen: invalid GL context");
            return;
        }

        unsafe {
            gl::ClearColor(color.r(), color.g(), color.b(), color.a());
            gl::Clear(gl::COLOR_BUFFER_BIT);
        }
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
