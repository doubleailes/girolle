use std::fmt;
use serde::{Deserialize,Serialize};
use lapin;

pub enum GirolleError {
    SerdeJsonError(serde_json::Error),
    LapinError(lapin::Error),
    ArgumentsError(String),
    IncorrectSignature(String),
    MethodNotFound(String),
    UnknownService(String),
    RemoteError(RemoteError),
    ServiceMissingError(String),
    SystemTimeError(std::time::SystemTimeError),
}

impl fmt::Display for GirolleError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            GirolleError::SerdeJsonError(e) => write!(f, "Serde JSON error: {}", e),
            GirolleError::LapinError(e) => write!(f, "Lapin error: {}", e),
            GirolleError::ArgumentsError(e) => write!(f, "Arguments error: {}", e),
            GirolleError::IncorrectSignature(e) => write!(f, "Incorrect signature: {}", e),
            GirolleError::MethodNotFound(e) => write!(f, "Method not found: {}", e),
            GirolleError::UnknownService(e) => write!(f, "Unknown service: {}", e),
            GirolleError::ServiceMissingError(e) => write!(f, "Service missing error: {}", e),
            GirolleError::SystemTimeError(e) => write!(f, "System time error: {}", e),
            GirolleError::RemoteError(e) => write!(f, "Remote error: {:?}", e),
        }
    }
}

impl fmt::Debug for GirolleError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            GirolleError::SerdeJsonError(e) => write!(f, "Serde JSON error: {:?}", e),
            GirolleError::LapinError(e) => write!(f, "Lapin error: {:?}", e),
            GirolleError::ArgumentsError(e) => write!(f, "Arguments error: {}", e),
            GirolleError::IncorrectSignature(e) => write!(f, "Incorrect signature: {}", e),
            GirolleError::MethodNotFound(e) => write!(f, "Method not found: {}", e),
            GirolleError::UnknownService(e) => write!(f, "Unknown service: {}", e),
            GirolleError::ServiceMissingError(e) => write!(f, "Service missing error: {:?}", e),
            GirolleError::SystemTimeError(e) => write!(f, "System time error: {:?}", e),
            GirolleError::RemoteError(e) => write!(f, "Remote error: {:?}", e),
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
impl From<std::time::SystemTimeError> for GirolleError {
    fn from(error: std::time::SystemTimeError) -> Self {
        GirolleError::SystemTimeError(error)
    }
    
}
impl From<RemoteError> for GirolleError {
    fn from(error: RemoteError) -> Self {
        GirolleError::RemoteError(error)
    }
}
impl GirolleError{
    pub fn convert(&self)->RemoteError {
        match self {
            GirolleError::RemoteError(e) => e.clone(),
            GirolleError::IncorrectSignature(e) => RemoteError {
                exc_path: "nameko.exceptions.IncorrectSignature".to_string(),
                exc_type: "IncorrectSignature".to_string(),
                value: e.clone(),
                exc_args: vec![e.clone()]
            },
            GirolleError::MethodNotFound(e) => RemoteError {
                exc_path: "nameko.exceptions.MethodNotFound".to_string(),
                exc_type: "MethodNotFound".to_string(),
                value: e.clone(),
                exc_args: vec![e.clone()]
            },
            GirolleError::UnknownService(e) => RemoteError {
                exc_path: "nameko.exceptions.UnknownService".to_string(),
                exc_type: "UnknownService".to_string(),
                value: e.clone(),
                exc_args: vec![e.clone()]
            },
            _ => RemoteError {
                exc_path: "".to_string(),
                exc_type: "".to_string(),
                value: "".to_string(),
                exc_args: vec![]
            }
        }
    }
}

impl std::error::Error for GirolleError {}

#[derive(Debug,Deserialize,Serialize,Clone)]
pub struct RemoteError {
    exc_path: String,
    exc_type: String,
    value: String,
    exc_args: Vec<String>,
}
impl RemoteError {
    pub fn convert_to_girolle_error(&self)-> GirolleError {
        match self.exc_type.as_str() {
            "IncorrectSignature" => {
                GirolleError::IncorrectSignature(self.value.clone())
            },
            "MethodNotFound" => {
                GirolleError::MethodNotFound(self.value.clone())
            },
            "UnknownService" => {
                GirolleError::UnknownService(self.value.clone())
            },
            _ => {
                GirolleError::RemoteError(self.clone())
            }
        }
    }
}