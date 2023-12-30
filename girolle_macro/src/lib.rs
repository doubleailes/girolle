use proc_macro::TokenStream;
use quote::quote;
use std::collections::HashSet;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::Token;
use syn::{parse_macro_input, Ident, ItemFn, Result};

#[proc_macro_attribute]
pub fn girolle(_metadata: TokenStream, tokens: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(tokens as ItemFn);
    let fn_name = &input_fn.sig.ident;
    let fn_params = &input_fn.sig.inputs;
    let fn_output = &input_fn.sig.output;
    let expanded = quote! {
        fn #fn_name(s: Vec<&Value>) -> Result<Value> {
            let n: u64 = serde_json::from_value(s[0].clone())?;
            let result: Value = serde_json::to_value(n)?;
            Ok(result)
        }
    };
    TokenStream::from(expanded)
}
