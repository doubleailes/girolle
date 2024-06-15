use proc_macro2::TokenStream;
use quote::quote;
use syn::fold::Fold;
use syn::parse_quote;
use syn::{parse2, FnArg, ItemFn};

struct Task {
    args: Vec<FnArg>,
    inner_statements: Vec<syn::Stmt>,
}
impl Task {
    fn new() -> Self {
        Task {
            args: Vec::new(),
            inner_statements: Vec::new(),
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
                    stmts.push(
                        parse_quote! {let #pat: #ty = serde_json::from_value(#data_quote.clone())?;},
                    );
                }
                _ => {}
            }
            i += 1;
        }
        let previous_stmts = self.inner_statements.clone();
        stmts.extend(previous_stmts.clone());
        self.inner_statements = stmts.clone();
    }
    fn add_output_serialize(&mut self) {
        let last_imut = self.inner_statements.pop().clone();
        let output_quote: syn::Stmt = parse_quote! {let output = #last_imut;};
        self.inner_statements.push(output_quote);
    }
    fn add_output_final(&mut self) {
        let final_line: syn::Stmt = parse_quote! {return Ok(serde_json::to_value(output)?);};
        self.inner_statements.push(final_line);
    }
    #[allow(dead_code)]
    fn print_stmts(&self) {
        for stmt in &self.inner_statements {
            println!("{}", quote!(#stmt).to_string());
        }
    }
}

impl Fold for Task {
    fn fold_signature(&mut self, i: syn::Signature) -> syn::Signature {
        let mut folded_item = i.clone();
        // Capture the original inputs
        self.args = folded_item.inputs.iter().cloned().collect();
        // Replace inputs by the NamekoFunction inputs
        folded_item.inputs = parse_quote! {
            data: &[Value]
        };
        // Replace the return type by the NamekoFunction return type
        folded_item.output = parse_quote! {-> NamekoResult<Value>};
        folded_item
    }
    fn fold_stmt(&mut self, i: syn::Stmt) -> syn::Stmt {
        let folded_item = i.clone();
        // Capture the original statements
        self.inner_statements.push(folded_item.clone());
        // Replace the statements by the NamekoFunction statements
        folded_item
    }
}

pub(crate) fn function(input: TokenStream) -> TokenStream {
    // proc_marco2 version of "parse_macro_input!(input as ItemFn)"
    let old_item_fn = match parse2::<ItemFn>(input) {
        Ok(syntax_tree) => syntax_tree,
        Err(error) => return error.to_compile_error(),
    };
    let mut task = Task::new();
    let mut new_item_fn = task.fold_item_fn(old_item_fn);
    task.add_input_serialize();
    task.add_output_serialize();
    task.add_output_final();
    new_item_fn.block.stmts = task.inner_statements.clone();
    TokenStream::from(quote!(#new_item_fn))
}

pub(crate) fn girolle_task(input: TokenStream) -> TokenStream {
    let item_fn = parse2::<ItemFn>(input).unwrap();
    let mut task = Task::new();
    let name = &item_fn.sig.ident.clone();
    let mut new_item_fn = task.fold_item_fn(item_fn.clone());
    task.add_input_serialize();
    task.add_output_serialize();
    task.add_output_final();
    new_item_fn.block.stmts = task.inner_statements.clone();

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
    let rpc_task_name = syn::Ident::new(
        &format!("{}_rpc_task", name),
        proc_macro2::Span::call_site(),
    );
    new_item_fn.sig.ident = rpc_task_name.clone();
    let expanded = quote! {
        #new_item_fn
        fn #name() -> girolle::RpcTask {
            girolle::RpcTask::new(#name_fn,vec![#(#args_str),*], #rpc_task_name)
        }
    };
    println!("{}", expanded.to_string());
    TokenStream::from(expanded)
}
