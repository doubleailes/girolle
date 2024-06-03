use serde_json::Value;
/// # Result
///
/// ## Description
///
/// This type is used to return a Result<Value> in the RPC call
pub type NamekoResult<T> = std::result::Result<T, serde_json::Error>;
/// # NamekoFunction
///
/// ## Description
///
/// This type is used to define the function to call in the RPC service it
/// mainly simplify the code to manipulate a complexe type.
pub type NamekoFunction = fn(&[Value]) -> NamekoResult<Value>;
