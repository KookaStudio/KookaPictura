use std::borrow::Cow;
use std::panic::{catch_unwind, UnwindSafe};

macro_rules! error_codes {
    ($($name:ident = $value:literal => $key:literal),+ $(,)?) => {
        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        #[non_exhaustive]
        #[repr(u32)]
        pub enum ErrorCode {
            $($name = $value),+
        }

        impl ErrorCode {
            pub const ALL: &'static [Self] = &[$(Self::$name),+];

            pub const fn message_key(self) -> &'static str {
                match self {
                    $(Self::$name => $key),+
                }
            }

            pub const fn status(self) -> OpStatus {
                OpStatus(self as u32)
            }
        }

        impl OpStatus {
            pub const fn code(self) -> Option<ErrorCode> {
                match self.0 {
                    $($value => Some(ErrorCode::$name),)+
                    _ => None,
                }
            }
        }
    };
}

error_codes! {
    Ok = 0 => "operation.ok",
    Version = 1 => "error.abi.version",
    Capability = 2 => "error.capability.denied",
    InvalidArg = 3 => "error.argument.invalid",
    Cancelled = 4 => "operation.cancelled",
    Host = 5 => "error.host",
    Panic = 6 => "error.panic",
    Codec = 100 => "error.codec",
    Io = 101 => "error.io",
    Color = 102 => "error.color",
    Gpu = 103 => "error.gpu",
    Scratch = 104 => "error.scratch",
    Script = 105 => "error.script",
    License = 107 => "error.license",
}

/// A status crossing the ABI may contain a code from a newer registry.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(transparent)]
pub struct OpStatus(u32);

impl OpStatus {
    pub const fn from_raw(raw: u32) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u32 {
        self.0
    }

    pub const fn is_ok(self) -> bool {
        self.0 == ErrorCode::Ok as u32
    }
}

#[derive(Debug, thiserror::Error)]
#[error("{detail}")]
pub struct Error {
    code: ErrorCode,
    detail: Cow<'static, str>,
    #[source]
    source: Option<Box<dyn std::error::Error + Send + Sync>>,
}

impl Error {
    pub fn new(code: ErrorCode, detail: impl Into<Cow<'static, str>>) -> Self {
        Self {
            code,
            detail: detail.into(),
            source: None,
        }
    }

    pub fn with_source(
        code: ErrorCode,
        detail: impl Into<Cow<'static, str>>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self {
            source: Some(Box::new(source)),
            ..Self::new(code, detail)
        }
    }

    pub fn code(&self) -> ErrorCode {
        self.code
    }

    pub fn detail(&self) -> &str {
        &self.detail
    }
}

/// Guard Rust work whose staged state can be discarded after an unwind.
pub fn guard<T>(operation: impl FnOnce() -> Result<T, Error> + UnwindSafe) -> Result<T, Error> {
    match catch_unwind(operation) {
        Ok(result) => result,
        Err(payload) => {
            // A custom payload can panic in Drop. Standard panic messages can be freed safely.
            if !payload.is::<&'static str>() && !payload.is::<String>() {
                std::mem::forget(payload);
            }
            Err(Error::new(
                ErrorCode::Panic,
                "the operation stopped unexpectedly",
            ))
        }
    }
}

#[cfg(test)]
mod tests;
