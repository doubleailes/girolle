use std::vec;
use std::{thread, time};
use girolle::{JsonValue::Value, RpcCall};
use serde_json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create the rpc call struct
    let rpc_call = RpcCall::new();
    // Transform the number into a JsonValue
    let t: serde_json::Number = serde_json::from_str("30").unwrap();
    // Create the payload
    let new_payload = vec![t.into()];
    // Send the payload
    let new_result = rpc_call
        .send("video".to_string(), "fibonacci".to_string(), new_payload)
        .await?;
    let fib_result: u64 = serde_json::from_value(new_result).unwrap();
    // Print the result
    println!("fibonacci :{:?}", fib_result);
    assert_eq!(fib_result, 832040);
    // Create a future result
    let future_result = rpc_call.call_async(
        "video".to_string(),
        "hello".to_string(),
        vec![Value::String("Toto".to_string())],
    );
    // Send a message
    let result = rpc_call
        .send(
            "video".to_string(),
            "hello".to_string(),
            vec![Value::String("Girolle".to_string())],
        )
        .await?;
    // Print the result
    println!("{:?}", result);
    assert_eq!(
        result,
        Value::String("Hello, Girolle!, by nameko".to_string())
    );
    // Wait for the future result
    let mut consumer = future_result.await?;
    // wait for it

    let two_sec = time::Duration::from_secs(20);
    thread::sleep(two_sec);
    // Print the result
    let async_result = rpc_call.result(&mut consumer).await;
    println!("{:?}", async_result);
    assert_eq!(
        async_result,
        Value::String("Hello, Toto!, by nameko".to_string())
    );
    Ok(())
}
