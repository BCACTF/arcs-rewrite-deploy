use std::{fmt::{Display, Formatter}, sync::PoisonError};
use log::SetLoggerError;

#[derive(Debug)]
pub struct PoisonErrorWrapper(String);

impl Display for PoisonErrorWrapper {


    fn fmt(&self, fmt: &mut Formatter) -> std::fmt::Result {
        self.0.fmt(fmt)
    }
}
impl std::error::Error for PoisonErrorWrapper {}


impl<T> From<PoisonError<T>> for PoisonErrorWrapper
where
    T: std::fmt::Debug {
    fn from(error: PoisonError<T>) -> Self {
        Self(format!("{:?}", error.to_string()))
    }
}


#[derive(Debug)]
pub struct ErrorWrapper(SetLoggerError);

impl Display for ErrorWrapper {
    fn fmt(&self, fmt: &mut Formatter) -> std::fmt::Result {
        self.0.fmt(fmt)
    }
}
impl std::error::Error for ErrorWrapper {}

impl From<SetLoggerError> for ErrorWrapper {
    fn from(error: SetLoggerError) -> Self {
        Self(error)
    }
}

pub mod shortcuts {
    pub use std::io::{
        Error as IOError,
        Result as IOResult,
    };
}
