pub mod cfg;

pub use cfg::*;

#[derive(thiserror::Error, Debug)]
pub enum BuildError {
    #[error("invalid input: {}", reason.as_ref().map(String::as_str).unwrap_or("??"))]
    InvalidInput { reason: Option<String> },
}

impl BuildError {
    fn invalid_input<T: ToString>(reason: T) -> Self {
        Self::InvalidInput {
            reason: Some(reason.to_string()),
        }
    }
}

pub type BuildResult<T> = std::result::Result<T, BuildError>;
