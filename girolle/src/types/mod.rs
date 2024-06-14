use lapin;
use serde_json;
use serde_json::Value;
use std::fmt;

/// # Result
///
/// ## Description
///
/// This type is used to return a `Result<Value>` in the RPC call
pub type NamekoResult<T> = std::result::Result<T, GirolleError>;
/// # NamekoFunction
///
/// ## Description
///
/// This type is used to define the function to call in the RPC service it
/// mainly simplify the code to manipulate a complexe type.
pub type NamekoFunction = fn(&[Value]) -> NamekoResult<Value>;

pub enum GirolleError {
    SerdeJsonError(serde_json::Error),
    LapinError(lapin::Error),
    ArgumentsError,
}

impl fmt::Display for GirolleError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            GirolleError::SerdeJsonError(e) => write!(f, "Serde JSON error: {}", e),
            GirolleError::LapinError(e) => write!(f, "Lapin error: {}", e),
            GirolleError::ArgumentsError => write!(f, "Arguments error"),
        }
    }
}

impl fmt::Debug for GirolleError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            GirolleError::SerdeJsonError(e) => write!(f, "Serde JSON error: {:?}", e),
            GirolleError::LapinError(e) => write!(f, "Lapin error: {:?}", e),
            GirolleError::ArgumentsError => write!(f, "Arguments error"),
        }
    }
}

impl From<serde_json::Error> for GirolleError {
    fn from(error: serde_json::Error) -> Self {
        GirolleError::SerdeJsonError(error)
    }
}

impl From<lapin::Error> for GirolleError {
    fn from(error: lapin::Error) -> Self {
        GirolleError::LapinError(error)
    }
}

impl std::error::Error for GirolleError {}
