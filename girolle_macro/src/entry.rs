use proc_macro2::TokenStream;
use quote::quote;
use syn::fold::Fold;
use syn::parse_quote;
use syn::{parse2, FnArg, ItemFn};

struct Task {
    name: syn::Ident,
    args: Vec<FnArg>,
    deser_wrapper: Vec<syn::Stmt>,
    args_input_core: Vec<syn::Pat>,
}
impl Task {
    fn new(name: syn::Ident) -> Self {
        Task {
            name,
            args: Vec::new(),
            deser_wrapper: Vec::new(),
            args_input_core: Vec::new(),
        }
    }
    fn add_input_serialize(&mut self) {
        let mut stmts: Vec<syn::Stmt> = Vec::new();
        let mut i: usize = 0;
        for arg in &self.args {
            let data_quote = quote! {
                data[#i]
            };
            match arg {
                FnArg::Typed(pat_type) => {
                    let pat = &pat_type.pat;
                    let ty = &pat_type.ty;
                    self.args_input_core.push(*pat.clone());
                    stmts.push(
                        parse_quote! {let #pat: #ty = serde_json::from_value(#data_quote.clone())?;},
                    );
                }
                _ => {}
            }
            i += 1;
        }
        self.deser_wrapper = stmts.clone();
    }
}

impl Fold for Task {
    fn fold_ident(&mut self, i: proc_macro2::Ident) -> proc_macro2::Ident {
        let folded_item = i.clone();
        // Capture the original statements
        if folded_item == self.name {
            return syn::Ident::new(
                &format!("{}_core", folded_item),
                proc_macro2::Span::call_site(),
            );
        }
        folded_item
    }
}

pub(crate) fn girolle_task(input: TokenStream) -> TokenStream {
    let item_fn = parse2::<ItemFn>(input).unwrap();
    let name = &item_fn.sig.ident.clone();
    let mut task = Task::new(name.clone());
    task.args = item_fn.sig.inputs.iter().cloned().collect();
    let new_item_fn = task.fold_item_fn(item_fn.clone());
    task.add_input_serialize();
    let _inputs = &item_fn.sig.inputs;
    let args_str: Vec<String> = task
        .args
        .iter()
        .map(|arg| match arg {
            FnArg::Typed(pat_type) => {
                let pat = &pat_type.pat;
                quote! {#pat}.to_string()
            }
            _ => "".to_string(),
        })
        .collect();
    let name_fn = quote! {#name}.to_string();
    let fn_wrap_name = syn::Ident::new(&format!("{}_wrap", name), proc_macro2::Span::call_site());
    let fn_core_name: syn::Ident =
        syn::Ident::new(&format!("{}_core", name), proc_macro2::Span::call_site());
    let wrap = task.deser_wrapper;
    let args_input_core: Vec<syn::Pat> = task.args_input_core.clone();
    let expanded = quote! {
        #new_item_fn
        fn #fn_wrap_name(data : & [Value]) -> GirolleResult<Value>{
            #(#wrap)*
            Ok(serde_json :: to_value(#fn_core_name(#(#args_input_core),*)) ?)
        }
        fn #name() -> girolle::RpcTask {
            girolle::RpcTask::new(#name_fn,vec![#(#args_str),*], #fn_wrap_name)
        }
    };
    TokenStream::from(expanded)
}
