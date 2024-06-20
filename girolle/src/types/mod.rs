use serde_json;
use serde_json::Value;
use crate::error::GirolleError;

/// # Result
///
/// ## Description
///
/// This type is used to return a `Result<Value>` in the RPC call
pub type GirolleResult<T> = std::result::Result<T, GirolleError>;
/// # NamekoFunction
///
/// ## Description
///
/// This type is used to define the function to call in the RPC service it
/// mainly simplify the code to manipulate a complexe type.
pub type NamekoFunction = fn(&[Value]) -> GirolleResult<Value>;
