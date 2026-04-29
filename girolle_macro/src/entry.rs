use proc_macro2::TokenStream;
use quote::quote;
use syn::fold::Fold;
use syn::parse_quote;
use syn::{parse2, FnArg, ItemFn, Pat, Stmt};

/// Renames the user-written function's identifier to `<name>_core`, both at
/// the definition site and at any recursive call site inside the body.
struct CoreRenamer {
    from: syn::Ident,
}

impl Fold for CoreRenamer {
    fn fold_ident(&mut self, i: proc_macro2::Ident) -> proc_macro2::Ident {
        if i == self.from {
            syn::Ident::new(
                &format!("{}_core", i),
                proc_macro2::Span::call_site(),
            )
        } else {
            i
        }
    }
}

/// Returns true if the FnArg is a typed argument whose type path ends in
/// `RpcContext`. We use this to detect the optional first context arg.
fn is_ctx_arg(arg: &FnArg) -> bool {
    match arg {
        FnArg::Typed(pt) => match &*pt.ty {
            syn::Type::Path(tp) => tp
                .path
                .segments
                .last()
                .map(|s| s.ident == "RpcContext")
                .unwrap_or(false),
            _ => false,
        },
        _ => false,
    }
}

pub(crate) fn girolle_task(input: TokenStream) -> TokenStream {
    let item_fn = parse2::<ItemFn>(input).expect("girolle: expected a free function");
    let name = item_fn.sig.ident.clone();
    let is_async = item_fn.sig.asyncness.is_some();

    let inputs: Vec<FnArg> = item_fn.sig.inputs.iter().cloned().collect();
    let (has_ctx, real_args): (bool, Vec<FnArg>) = match inputs.first() {
        Some(first) if is_ctx_arg(first) => (true, inputs[1..].to_vec()),
        _ => (false, inputs),
    };

    // Rename the original function to `<name>_core`.
    let mut renamer = CoreRenamer { from: name.clone() };
    let new_item_fn = renamer.fold_item_fn(item_fn);

    // Build the per-arg name list used for kwargs->args dispatch and the
    // deserialization statements unpacking each arg from the Payload.
    let mut args_str: Vec<String> = Vec::new();
    let mut deser_stmts: Vec<Stmt> = Vec::new();
    let mut args_input_core: Vec<Pat> = Vec::new();
    for (i, arg) in real_args.iter().enumerate() {
        if let FnArg::Typed(pat_type) = arg {
            let pat = &pat_type.pat;
            let ty = &pat_type.ty;
            args_str.push(quote! { #pat }.to_string());
            args_input_core.push(*pat.clone());
            deser_stmts.push(parse_quote! {
                let #pat: #ty = ::girolle::serde_json::from_value(__args[#i].clone())?;
            });
        }
    }

    let name_fn = quote! { #name }.to_string();
    let fn_core_name =
        syn::Ident::new(&format!("{}_core", name), proc_macro2::Span::call_site());

    let core_call = match (has_ctx, is_async) {
        (true, true) => quote! { #fn_core_name(__ctx, #(#args_input_core),*).await },
        (true, false) => quote! { #fn_core_name(__ctx, #(#args_input_core),*) },
        (false, true) => quote! { #fn_core_name(#(#args_input_core),*).await },
        (false, false) => quote! { #fn_core_name(#(#args_input_core),*) },
    };

    let ctx_binding = if has_ctx {
        quote! { let __ctx = ctx; }
    } else {
        quote! { let _ = ctx; }
    };

    quote! {
        #new_item_fn

        fn #name() -> ::girolle::RpcTask {
            ::girolle::RpcTask::new(
                #name_fn,
                ::std::sync::Arc::new(
                    |ctx: ::girolle::RpcContext, payload: ::girolle::Payload|
                        -> ::girolle::BoxFuture<::girolle::GirolleResult<::girolle::Value>>
                    {
                        ::std::boxed::Box::pin(async move {
                            #ctx_binding
                            let __args = ::girolle::nameko_utils::build_inputs_fn_service(
                                &[#(#args_str),*],
                                payload,
                            )?;
                            #(#deser_stmts)*
                            let __result = #core_call;
                            Ok(::girolle::serde_json::to_value(__result)?)
                        })
                    },
                ),
            )
        }
    }
}
