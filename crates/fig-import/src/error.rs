use thiserror::Error;

#[derive(Error, Debug)]
pub enum FigError {
    #[error("Invalid magic header: expected 'fig-kiwi' or 'fig-jam.', found {0:?}")]
    InvalidMagicHeader(Vec<u8>),

    #[error("File too small: expected at least {expected} bytes, found {actual}")]
    FileTooSmall { expected: usize, actual: usize },

    #[error("Incomplete chunk at offset {offset}: expected {expected} bytes, found {actual}")]
    IncompleteChunk {
        offset: usize,
        expected: usize,
        actual: usize,
    },

    #[error("Not enough chunks: expected at least {expected}, found {actual}")]
    NotEnoughChunks { expected: usize, actual: usize },

    #[error("ZIP error: {0}")]
    ZipError(String),

    #[error("Canvas file not found in ZIP archive")]
    CanvasNotFoundInZip,

    #[error("Decode error: {0}")]
    DecodeError(String),

    #[error("Tree build error: {0}")]
    TreeError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("ZIP library error: {0}")]
    ZipLibraryError(#[from] zip::result::ZipError),
}

pub type Result<T> = std::result::Result<T, FigError>;
