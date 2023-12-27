use girolle::{JsonValue::Value, RpcClient};
use std::vec;
use std::{thread, time};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let video_name = "video".to_string();
    // Create the rpc call struct
    let rpc_client = RpcClient::new();
    // Create a future result
    let future_result = rpc_client.call_async(
        video_name.clone(),
        "hello".to_string(),
        vec![Value::String("Toto".to_string())],
    );
    // Send a message during the previous async process
    let result = rpc_client.send(
        video_name.clone(),
        "hello".to_string(),
        vec![Value::String("Girolle".to_string())],
    )?;
    // Print the result
    println!("{:?}", result);
    assert_eq!(
        result,
        Value::String("Hello, Girolle!, by nameko".to_string())
    );
    // Wait for the future result
    let consumer = future_result.await?;
    // wait for it
    let two_sec = time::Duration::from_secs(2);
    thread::sleep(two_sec);
    // Print the result
    let async_result = rpc_client.result(consumer).await;
    println!("{:?}", async_result);
    assert_eq!(
        async_result,
        Value::String("Hello, Toto!, by nameko".to_string())
    );
    let mut consummers: Vec<_> = Vec::new();
    for n in 1..101 {
        consummers.push(rpc_client.call_async(
            video_name.clone(),
            "hello".to_string(),
            vec![Value::String(n.to_string())],
        ));
    }
    // wait for it
    thread::sleep(two_sec);
    for con in consummers {
        let async_result = rpc_client.result(con.await?).await;
        println!("{}", async_result.as_str().unwrap());
    }
    Ok(())
}
