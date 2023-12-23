use thiserror::Error;

/// Result type for convenience.
pub type Result<T> = std::result::Result<T, Error>;

/// Error type. Mostly comprised of other crates' errors.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// <https://docs.rs/winit/0.29.5/winit/error/enum.EventLoopError.html>
    #[error(transparent)]
    EventLoopError(#[from] winit::error::EventLoopError),
    /// <https://docs.rs/winit/0.29.5/winit/error/struct.OsError.html>
    #[error(transparent)]
    OsError(#[from] winit::error::OsError),
    /// <https://docs.rs/pixels/0.13.0/pixels/enum.Error.html>
    #[error(transparent)]
    PixelsError(#[from] pixels::Error),
    /// <https://docs.rs/pixels/0.13.0/pixels/enum.TextureError.html>
    #[error(transparent)]
    TextureError(#[from] pixels::TextureError),
    /// Generic error.
    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync + 'static>),
}

impl Error {
    /// Construct an [`Error::Other`] from a string.
    #[inline]
    pub fn new(msg: impl ToString) -> Self {
        Error::Other(msg.to_string().into())
    }
}
