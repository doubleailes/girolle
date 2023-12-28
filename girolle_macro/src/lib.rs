extern crate proc_macro;

use proc_macro::*;

#[proc_macro_attribute]
pub fn girolle(args: TokenStream, input: TokenStream) -> TokenStream {
    let x = format!(
        r#"
    fn run() {{
        println!("entering");
        println!("args tokens: {{}}", {args});
        println!("input tokens: {{}}", {input});
        println!("exiting");
    }}
"#,
        args = args.into_iter().count(),
        input = input.into_iter().count(),
    );

    x.parse().expect("Generated invalid tokens")
}
