use proc_macro::TokenStream;
mod entry;

#[proc_macro_attribute]
pub fn girolle(_metadata: TokenStream, input: TokenStream) -> TokenStream {
    entry::main(input.into()).into()
}
