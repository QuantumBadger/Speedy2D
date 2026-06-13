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
use std::fmt::{Debug, Display, Formatter};
use std::rc::Rc;

use backtrace::Backtrace;

/// An error with an associated backtrace, and an optional cause.
#[derive(Clone)]
pub struct BacktraceError<E>
where
    E: Debug + Display + 'static
{
    value: Rc<BacktraceErrorImpl<E>>
}

struct BacktraceErrorImpl<E>
where
    E: Debug + Display
{
    error: E,
    backtrace: Backtrace,
    cause: Option<Box<dyn std::error::Error>>
}

impl<E: Debug + Display> std::error::Error for BacktraceError<E>
{
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)>
    {
        match self.value.cause {
            Some(ref cause) => Some(&**cause),
            None => None
        }
    }

    fn cause(&self) -> Option<&dyn std::error::Error>
    {
        self.source()
    }
}

impl<E: Debug + Display> Display for BacktraceError<E>
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result
    {
        Display::fmt(self.error(), f)
    }
}

impl<E: Debug + Display> Debug for BacktraceError<E>
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result
    {
        f.debug_struct("BacktraceError")
            .field("error", self.error())
            .field("backtrace", self.get_backtrace())
            .field("cause", self.cause())
            .finish()
    }
}

impl<E: Debug + Display> BacktraceError<E>
{
    #[must_use]
    pub(crate) fn new_with_cause<Cause: std::error::Error + 'static>(
        error: E,
        cause: Cause
    ) -> Self
    {
        BacktraceError {
            value: Rc::new(BacktraceErrorImpl {
                backtrace: Backtrace::new(),
                error,
                cause: Some(Box::new(cause))
            })
        }
    }

    #[must_use]
    pub(crate) fn new(error: E) -> Self
    {
        BacktraceError {
            value: Rc::new(BacktraceErrorImpl {
                backtrace: Backtrace::new(),
                error,
                cause: None
            })
        }
    }

    /// Returns the backtrace for this error.
    #[must_use]
    pub fn get_backtrace(&self) -> &Backtrace
    {
        &self.value.backtrace
    }

    /// Returns the error.
    #[must_use]
    pub fn error(&self) -> &E
    {
        &self.value.error
    }

    /// Returns the original cause of the error, if one is present.
    #[must_use]
    pub fn cause(&self) -> &Option<Box<dyn std::error::Error>>
    {
        &self.value.cause
    }

    #[must_use]
    pub(crate) fn context<S: AsRef<str>>(
        self,
        description: S
    ) -> BacktraceError<ErrorMessage>
    {
        BacktraceError::new_with_cause(
            ErrorMessage {
                description: description.as_ref().to_string()
            },
            self
        )
    }
}

/// A human-readable error message.
#[derive(Clone, Debug)]
pub struct ErrorMessage
{
    description: String
}

impl ErrorMessage
{
    pub(crate) fn msg<S: AsRef<str>>(description: S) -> BacktraceError<Self>
    {
        BacktraceError::new(Self {
            description: description.as_ref().to_string()
        })
    }

    pub(crate) fn msg_with_cause<S, Cause>(
        description: S,
        cause: Cause
    ) -> BacktraceError<Self>
    where
        S: AsRef<str>,
        Cause: std::error::Error + 'static
    {
        BacktraceError::new_with_cause(
            Self {
                description: description.as_ref().to_string()
            },
            cause
        )
    }
}

impl Display for ErrorMessage
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result
    {
        Display::fmt(&self.description, f)
    }
}

pub(crate) trait Context<R>
{
    fn context<S: AsRef<str>>(
        self,
        description: S
    ) -> Result<R, BacktraceError<ErrorMessage>>;
}

impl<R, E: std::error::Error + 'static> Context<R> for Result<R, E>
{
    fn context<S: AsRef<str>>(
        self,
        description: S
    ) -> Result<R, BacktraceError<ErrorMessage>>
    {
        match self {
            Ok(result) => Ok(result),
            Err(err) => Err(BacktraceError::new_with_cause(
                ErrorMessage {
                    description: description.as_ref().to_string()
                },
                err
            ))
        }
    }
}
