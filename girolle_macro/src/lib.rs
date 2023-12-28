use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};
use girolle::RpcService;
use girolle::{JsonValue::Value, Result};

fn hello(s: Vec<&Value>) -> Result<Value> {
    // Parse the incomming data
    let n: String = serde_json::from_value(s[0].clone())?;
    let hello_str: Value = format!("Hello, {}!, by Girolle", n).into();
    Ok(hello_str)
}

#[proc_macro_attribute]
pub fn girolle(args: TokenStream, item: TokenStream) -> TokenStream {
    // Parse the function
    let input = parse_macro_input!(item as ItemFn);
    let signa = &input.sig;
    // Extract the function name
    let fn_name = &input.sig.ident;
    let service_name = args.to_string();
    let mut service = RpcService::new(service_name);
    // Generate the new function
    let expanded = quote! {
        #input
    };
    service.insert(fn_name.to_string(), hello);
    let _ = service.start();
    // Return the generated code as a TokenStream
    TokenStream::from(expanded)
}