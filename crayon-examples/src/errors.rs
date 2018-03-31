use crayon::{graphics, resource};
use crayon_3d;

#[derive(Debug, Fail)]
pub enum Error {
    #[fail(display = "{}", _0)] Graphics(graphics::errors::Error),
    #[fail(display = "{}", _0)] Resource(resource::errors::Error),
    #[fail(display = "{}", _0)] Scene(crayon_3d::errors::Error),
}

pub type Result<T> = ::std::result::Result<T, Error>;

impl From<graphics::errors::Error> for Error {
    fn from(err: graphics::errors::Error) -> Self {
        Error::Graphics(err)
    }
}

impl From<resource::errors::Error> for Error {
    fn from(err: resource::errors::Error) -> Self {
        Error::Resource(err)
    }
}

impl From<crayon_3d::errors::Error> for Error {
    fn from(err: crayon_3d::errors::Error) -> Self {
        Error::Scene(err)
    }
}