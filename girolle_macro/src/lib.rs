use proc_macro::TokenStream;
mod entry;

/// #girolle macro
///
/// ## Description
///
/// girolle macro is used to generate a serializable function.
/// The function must be a pure function, which means that it must not have any side effects.
/// The function must be serializable, which means that it must not have any reference to a type that is not serializable.
/// The function match the type NamekoFunction which is defined as follow:
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
    entry::main(input.into()).into()
}
