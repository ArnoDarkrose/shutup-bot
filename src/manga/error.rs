#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Lopdf(#[from] lopdf::Error),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Image(#[from] image::error::ImageError),

    #[error("Error while performing image conversion")]
    ImageConvert,
}

pub type Result<T> = std::result::Result<T, Error>;
