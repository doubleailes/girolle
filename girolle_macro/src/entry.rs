use girolle::{JsonValue::Value, Result};
use proc_macro2::TokenStream;
use quote::quote;
use syn::fold::Fold;
use syn::parse_quote;
use syn::{parse2, FnArg, ItemFn};

struct Task {
    args: Vec<FnArg>,
}
impl Task {
    fn new() -> Self {
        Task { args: Vec::new() }
    }
}

impl Fold for Task {
    fn fold_fn_arg(&mut self, i: FnArg) -> FnArg {
        self.args.push(i.clone());
        i
    }
    fn fold_signature(&mut self, i: syn::Signature) -> syn::Signature {
        let mut folded_item = i.clone();
        folded_item.inputs = parse_quote! {
            data: Vec<&Value>
        };
        folded_item.output = parse_quote! {
            -> girolle::Result<Value>
        };
        folded_item
    }
}

pub(crate) fn main(input: TokenStream) -> TokenStream {
    // proc_marco2 version of "parse_macro_input!(input as ItemFn)"
    let old_item_fn = match parse2::<ItemFn>(input) {
        Ok(syntax_tree) => syntax_tree,
        Err(error) => return error.to_compile_error(),
    };
    let mut task = Task::new();
    let new_item_fn = task.fold_item_fn(old_item_fn);
    println!("numbers of args {}", task.args.len());
    println!("{:?}", quote!(#new_item_fn).to_string());
    quote!(#new_item_fn)
}
