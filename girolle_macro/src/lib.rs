use proc_macro::TokenStream;
mod entry;

/// #girolle macro
///
/// ## Description
///
/// girolle macro is used to generate a complexe tree function.
/// - The first function is a copy of the original function with a suffix `_core`.
/// - The second one is just a wrapper to deserialize the input and serialize the output
/// with serde_json.
/// - The thrid one is the creation of the RpcTask.
///
/// `type NamekoFunction = fn(&[Value]) -> Result<Value>`
///
/// It support the serde serialization types.
/// ```rust,no_run
/// use serde_json::{Number, Map};
///
/// enum Value {
///    Null,
///    Bool(bool),
///    Number(Number),
///    String(String),
///    Array(Vec<Value>),
///    Object(Map<String, Value>),
///}
/// ```
/// The function must be deterministic, which means that it must always return a serializable result.

#[proc_macro_attribute]
pub fn girolle(_metadata: TokenStream, input: TokenStream) -> TokenStream {
    entry::girolle_task(input.into()).into()
}
