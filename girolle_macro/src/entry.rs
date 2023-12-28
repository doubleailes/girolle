use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

pub(crate) fn main(args: TokenStream, item: TokenStream) -> TokenStream {
        // Parse the function
        let input = syn::parse2(item.clone());

        // Extract the function name
        let fn_name = &input.sig.ident;
    
        // Generate the new function
        let expanded = quote! {
    
        };
    
        // Return the generated code as a TokenStream
        TokenStream::from(expanded)
}