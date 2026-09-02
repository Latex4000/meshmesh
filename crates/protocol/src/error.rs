use thiserror::Error;

use crate::format::Response;

#[derive(Error, Debug)]
pub enum Error {
    // own
    #[error("Missing peer")]
    MissingPeerError,

    #[error("Can't connect to yourself")]
    SelfConnectingError,

    #[error("Unexpected response: {0:?}")]
    UnexpectedResponseError(Response),

    #[error("No response found")]
    NoResponseError,

    #[error("Unknown error")]
    Unknown,

    // iroh
    #[error("Could not bind via iroh")]
    BindError(#[from] iroh::endpoint::BindError),

    #[error("Could not parse endpoint ticket")]
    TicketParseError(#[from] iroh_tickets::ParseError),

    #[error("Could not form a connection")]
    ConnectError(#[from] iroh::endpoint::ConnectError),

    #[error("Could not use the connection")]
    ConnectionError(#[from] iroh::endpoint::ConnectionError),

    // postcard
    #[error("Error from postcard")]
    PostcardError(#[from] postcard::Error),

    // others
    #[error("IO error")]
    IOError(#[from] std::io::Error),
}
