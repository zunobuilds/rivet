use thiserror::Error;

/// Deliberately excludes source text, SQL, provider content and OS path details.
#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid input or configuration")]
    Invalid,
    #[error("capability unsupported or not ready")]
    Unsupported,
    #[error("state unavailable or unsafe")]
    State,
    #[error("state busy: active ownership")]
    Busy,
    #[error("state requires supervised recovery")]
    Recovery,
    #[error("permission denied or stale grant")]
    Denied,
    #[error("transition conflict or stale owner")]
    Conflict,
    #[error("interrupted")]
    Interrupted,
}
impl Error {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Invalid => "invalid-input", Self::Unsupported => "unsupported", Self::Busy => "busy",
            Self::Recovery => "recovery-required", Self::Denied => "denied", Self::Conflict => "conflict",
            Self::Interrupted => "interrupted", Self::State => "state-failure",
        }
    }
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::Invalid => 2,
            Self::Unsupported | Self::Denied => 3,
            Self::Interrupted => 130,
            _ => 4,
        }
    }
}
pub type Result<T> = std::result::Result<T, Error>;
impl From<std::io::Error> for Error {
    fn from(_: std::io::Error) -> Self { Self::State }
}
impl From<rusqlite::Error> for Error {
    fn from(_: rusqlite::Error) -> Self { Self::State }
}
