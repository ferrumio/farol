use std::path::PathBuf;

use miette::{Diagnostic, NamedSource, SourceSpan};
use thiserror::Error;

/// Top-level error type for farol-core.
#[derive(Debug, Error, Diagnostic)]
pub enum FarolError {
    #[error("failed to read `{path}`")]
    #[diagnostic(code(farol::io))]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error(transparent)]
    #[diagnostic(transparent)]
    ConfigParse(#[from] Box<ConfigParseError>),

    #[error("config value out of range: {message}")]
    #[diagnostic(code(farol::config::invalid))]
    ConfigInvalid { message: String },

    #[error("invalid frontmatter in `{path}`")]
    #[diagnostic(code(farol::frontmatter))]
    Frontmatter { path: PathBuf, message: String },

    #[error("project already exists at `{path}`")]
    #[diagnostic(code(farol::scaffold::exists))]
    ScaffoldExists { path: PathBuf },
}

pub type Result<T> = std::result::Result<T, FarolError>;

#[derive(Debug, Error, Diagnostic)]
#[error("invalid config in `{}`", src.name())]
#[diagnostic(code(farol::config::parse))]
pub struct ConfigParseError {
    #[source_code]
    pub src: NamedSource<String>,
    #[label("here")]
    pub span: SourceSpan,
    #[help]
    pub help: Option<String>,
    pub message: String,
}

impl FarolError {
    pub(crate) fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        FarolError::Io { path: path.into(), source }
    }
}
