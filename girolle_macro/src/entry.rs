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
    #[allow(dead_code)]
    is_async: bool,
    has_context: bool,
}
impl Task {
    fn new(name: syn::Ident, is_async: bool, has_context: bool) -> Self {
        Task {
            name,
            args: Vec::new(),
            deser_wrapper: Vec::new(),
            args_input_core: Vec::new(),
            is_async,
            has_context,
        }
    }
    /// # add_input_serialize
    ///
    /// ## Description
    ///
    /// This function is used to add the input serialization to the task.
    /// It use for the second function to deserialize the input.
    ///
    /// ## Arguments
    ///
    /// - `self` : &mut Task : The task to modify.
    fn add_input_serialize(&mut self) {
        let mut stmts: Vec<syn::Stmt> = Vec::new();
        let start_idx = if self.has_context { 1 } else { 0 }; // Skip context if present
        for (i, arg) in self.args.iter().enumerate().skip(start_idx) {
            let data_idx = i - start_idx; // Adjust index for data array
            let data_quote = quote! {
                data[#data_idx]
            };
            if let FnArg::Typed(pat_type) = arg {
                let pat = &pat_type.pat;
                let ty = &pat_type.ty;
                self.args_input_core.push(*pat.clone());
                stmts.push(
                    parse_quote! {let #pat: #ty = serde_json::from_value(#data_quote.clone())?;},
                );
            }
        }
        self.deser_wrapper.clone_from(&stmts);
    }
}

impl Fold for Task {
    /// # fold_ident
    ///
    /// ## Description
    ///
    /// This function is used to fold the ident of the function by replacing the
    /// original name by the name suffixed by `_core`.
    ///
    /// ## Arguments
    ///
    /// - `i` : proc_macro2::Ident : The ident to fold.
    ///
    /// ## Returns
    ///
    /// - `proc_macro2::Ident` : The folded ident.
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
    let name = item_fn.sig.ident.clone();
    let is_async = item_fn.sig.asyncness.is_some();
    
    // Check if first argument is RpcContext - use more robust type checking
    let has_context = if let Some(FnArg::Typed(pat_type)) = item_fn.sig.inputs.first() {
        // Check the type more robustly
        matches_rpc_context_type(&pat_type.ty)
    } else {
        false
    };

    let mut task = Task::new(name.clone(), is_async, has_context);
    task.args = item_fn.sig.inputs.iter().cloned().collect();
    let new_item_fn = task.fold_item_fn(item_fn);
    task.add_input_serialize();
    
    let args_str: Vec<String> = task
        .args
        .iter()
        .skip(if has_context { 1 } else { 0 }) // Skip context in args list
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

    let expanded = if is_async && has_context {
        // Generate async handler with RpcContext
        quote! {
            #new_item_fn
            
            fn #name() -> girolle::RpcTask {
                use std::sync::Arc;
                use std::pin::Pin;
                use std::future::Future;
                
                let async_fn = Arc::new(
                    move |ctx: Arc<girolle::RpcContext>, data: Vec<Value>| -> Pin<Box<dyn Future<Output = GirolleResult<Value>> + Send>> {
                        Box::pin(async move {
                            #(#wrap)*
                            Ok(serde_json::to_value(#fn_core_name(ctx, #(#args_input_core),*).await)?)
                        })
                    }
                );
                girolle::RpcTask::new_async(#name_fn, vec![#(#args_str),*], async_fn)
            }
        }
    } else {
        // Generate sync handler (backward compatible)
        quote! {
            #new_item_fn
            fn #fn_wrap_name(data : & [Value]) -> GirolleResult<Value>{
                #(#wrap)*
                Ok(serde_json :: to_value(#fn_core_name(#(#args_input_core),*)) ?)
            }
            fn #name() -> girolle::RpcTask {
                girolle::RpcTask::new(#name_fn,vec![#(#args_str),*], #fn_wrap_name)
            }
        }
    };
    expanded
}

/// Check if a type matches RpcContext more robustly
fn matches_rpc_context_type(ty: &syn::Type) -> bool {
    use syn::{GenericArgument, PathArguments, Type};
    
    match ty {
        // Check for Arc<RpcContext>
        Type::Path(type_path) => {
            // Check if last segment is Arc
            if let Some(last_seg) = type_path.path.segments.last() {
                if last_seg.ident == "Arc" {
                    // Check the generic argument
                    if let PathArguments::AngleBracketed(args) = &last_seg.arguments {
                        if let Some(GenericArgument::Type(Type::Path(inner_path))) = args.args.first() {
                            // Check if inner type ends with RpcContext
                            if let Some(inner_seg) = inner_path.path.segments.last() {
                                return inner_seg.ident == "RpcContext";
                            }
                        }
                    }
                }
            }
            false
        }
        _ => false,
    }
}
