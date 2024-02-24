use girolle::prelude::*;

#[girolle]
fn simple_operation(a: i32, b: i32) -> i32 {
    a + b
}

#[test]
fn test_simple_operation() {
    let expected = serde_json::json!(3);
    let input = vec![serde_json::json!(1), serde_json::json!(2)];
    assert_eq!(simple_operation(&input).unwrap(), expected);
}

#[girolle]
fn return_operation(a: i32, b: i32) -> i32{
    return a + b;
}

#[test]
fn test_return_operation() {
    let expected = serde_json::json!(3);
    let input = vec![serde_json::json!(1), serde_json::json!(2)];
    assert_eq!(return_operation(&input).unwrap(), expected);
}