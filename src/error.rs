use miette::Diagnostic;
use thiserror::Error;

#[derive(Debug, Diagnostic, Error)]
pub enum DoitError {
    #[error("I/O error: {msg}")]
    #[diagnostic(code(doit::io))]
    Io {
        msg: String,
        #[source]
        source: std::io::Error,
    },

    #[error("Configuration error: {msg}")]
    #[diagnostic(code(doit::config))]
    Config { msg: String },

    #[error("Session error: {msg}")]
    #[diagnostic(code(doit::session))]
    Session { msg: String },

    #[error("Backend error: {msg}")]
    #[diagnostic(code(doit::backend))]
    Backend { msg: String },

    #[error("Shell error: {msg}")]
    #[diagnostic(code(doit::shell))]
    Shell { msg: String },

    #[error("{msg}")]
    #[diagnostic(code(doit::internal))]
    Internal { msg: String },
}

pub type Result<T> = std::result::Result<T, DoitError>;

impl DoitError {
    pub fn io(source: std::io::Error, msg: impl Into<String>) -> Self {
        Self::Io {
            msg: msg.into(),
            source,
        }
    }

    pub fn config(msg: impl Into<String>) -> Self {
        Self::Config { msg: msg.into() }
    }

    pub fn session(msg: impl Into<String>) -> Self {
        Self::Session { msg: msg.into() }
    }

    pub fn backend(msg: impl Into<String>) -> Self {
        Self::Backend { msg: msg.into() }
    }

    pub fn shell(msg: impl Into<String>) -> Self {
        Self::Shell { msg: msg.into() }
    }

    pub fn internal(msg: impl Into<String>) -> Self {
        Self::Internal { msg: msg.into() }
    }
}
