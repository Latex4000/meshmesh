use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("could not bind via iroh")]
    BindError(#[from] iroh::endpoint::BindError),

    #[error("unknown error")]
    Unknown,
}
