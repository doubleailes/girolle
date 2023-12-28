use girolle_macro::girolle;

#[girolle("video")]
fn hello(s: String) -> String {
    // Parse the incoming data
    format!("Hello, {}!, by Girolle", s)
}

fn main() {
    // Start the server
    hello("toto".to_string());
}