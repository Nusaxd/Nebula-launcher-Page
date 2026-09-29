use serde::{Serialize, Serializer};
use std::fmt;

/// A simple string based error that can be sent to the frontend.
#[derive(Debug)]
pub struct Error(pub String);

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    pub fn msg(m: impl Into<String>) -> Self {
        Error(m.into())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

// Note: `Error` intentionally does not implement `std::error::Error`,
// otherwise this blanket impl would conflict with `impl<T> From<T> for T`.
impl<E: std::error::Error> From<E> for Error {
    fn from(e: E) -> Self {
        let mut msg = e.to_string();
        let mut source = e.source();
        while let Some(s) = source {
            let text = s.to_string();
            if !msg.contains(&text) {
                msg.push_str(": ");
                msg.push_str(&text);
            }
            source = s.source();
        }
        Error(msg)
    }
}

impl Serialize for Error {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

/// `bail!("message {}", x)` returns early with an [`Error`].
#[macro_export]
macro_rules! bail {
    ($($arg:tt)*) => {
        return Err($crate::error::Error::msg(format!($($arg)*)))
    };
}
