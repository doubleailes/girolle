use proc_macro2::TokenStream;
use quote::quote;
use syn::fold::Fold;
use syn::parse_quote;
use syn::{parse2, FnArg, ItemFn};

struct Task {
    args: Vec<FnArg>,
    inner_statements: Vec<syn::Stmt>,
    return_stmt: bool,
}
impl Task {
    fn new() -> Self {
        Task {
            args: Vec::new(),
            inner_statements: Vec::new(),
            return_stmt: false,
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
    fn replace_return(&mut self){
        for stmt in &mut self.inner_statements {
            if let syn::Stmt::Expr(expr, _) = stmt {
                // If the statement is an expression, check if it's a return statement
                if let syn::Expr::Return(_) = &*expr {
                    // Replace return with let output =
                    *stmt = parse_quote! {
                        let output = 
                    };
                    self.return_stmt = true;
                }
            }
        }
    }
    fn add_output_serialize(&mut self) {
        if self.return_stmt == false {
            let last_imut = self.inner_statements.pop().clone();
            let output_quote: syn::Stmt = parse_quote! {let output = #last_imut;};
            self.inner_statements.push(output_quote);
        } else{
            println!("return_stmt is true");
        }
    }
    fn add_output_final(&mut self) {
        let final_line: syn::Stmt = parse_quote! {return Ok(serde_json::to_value(output)?);};
        self.inner_statements.push(final_line);
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

pub(crate) fn main(input: TokenStream) -> TokenStream {
    // proc_marco2 version of "parse_macro_input!(input as ItemFn)"
    let old_item_fn = match parse2::<ItemFn>(input) {
        Ok(syntax_tree) => syntax_tree,
        Err(error) => return error.to_compile_error(),
    };
    let mut task = Task::new();
    let mut new_item_fn = task.fold_item_fn(old_item_fn);
    task.add_input_serialize();
    task.replace_return();
    task.add_output_serialize();
    task.add_output_final();
    new_item_fn.block.stmts = task.inner_statements.clone();
    println!("new_item_fn: {:?}", quote!(#new_item_fn).to_string());
    TokenStream::from(quote!(#new_item_fn))
}
