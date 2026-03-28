//! Project-wide error type.

/// All errors in lettre-cat.
#[derive(Debug)]
pub enum EmailError {
    /// SMTP transport error.
    Smtp(lettre::transport::smtp::Error),
    /// Message construction error.
    Message(lettre::error::Error),
    /// Address parsing error.
    Address(lettre::address::AddressError),
    /// Missing required configuration.
    Config { field: String },
}

impl std::fmt::Display for EmailError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Smtp(e) => write!(f, "SMTP error: {e}"),
            Self::Message(e) => write!(f, "message error: {e}"),
            Self::Address(e) => write!(f, "address error: {e}"),
            Self::Config { field } => write!(f, "missing config: {field}"),
        }
    }
}

impl std::error::Error for EmailError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Smtp(e) => Some(e),
            Self::Message(e) => Some(e),
            Self::Address(e) => Some(e),
            Self::Config { .. } => None,
        }
    }
}

impl From<lettre::transport::smtp::Error> for EmailError {
    fn from(e: lettre::transport::smtp::Error) -> Self { Self::Smtp(e) }
}

impl From<lettre::error::Error> for EmailError {
    fn from(e: lettre::error::Error) -> Self { Self::Message(e) }
}

impl From<lettre::address::AddressError> for EmailError {
    fn from(e: lettre::address::AddressError) -> Self { Self::Address(e) }
}
