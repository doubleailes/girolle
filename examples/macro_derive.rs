use girolle::girolle_macro::girolle;


#[girolle("video")]
fn hello(s: String) -> String {
    // Parse the incomming data
    format!("Hello, {}!, by Girolle", s)
}

fn main() {
    run();
}
