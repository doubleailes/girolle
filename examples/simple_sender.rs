use girolle::{serde_json, Config, RpcClient, Value};
use std::vec;
use std::{thread, time};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let conf = Config::with_yaml_defaults("staging/config.yml".to_string())?;
    let video_name = "video";
    // Create the rpc call struct
    let rpc_client = RpcClient::new(conf);
    // Send the payload
    let new_result = rpc_client.send(video_name, "fibonacci", vec![30])?;
    let fib_result: u64 = serde_json::from_value(new_result)?;
    // Print the result
    println!("fibonacci :{:?}", fib_result);
    assert_eq!(fib_result, 832040);
    // Create a future result
    let future_result = rpc_client.call_async(video_name, "hello", vec!["Toto"]);
    // Send a message during the previous async process
    let result = rpc_client.send(video_name, "hello", vec!["Girolle"])?;
    // Print the result
    println!("{:?}", result);
    assert_eq!(result, Value::String("Hello, Girolle!".to_string()));
    // Wait for the future result
    let consumer = future_result.await?;
    // wait for it
    let two_sec = time::Duration::from_secs(2);
    thread::sleep(two_sec);
    // Print the result
    let async_result = rpc_client.result(consumer).await;
    println!("{:?}", async_result);
    assert_eq!(async_result, Value::String("Hello, Toto!".to_string()));
    let mut consummers: Vec<_> = Vec::new();
    for n in 1..101 {
        consummers.push((
            n,
            rpc_client
                .call_async(video_name, "hello", vec![n.to_string()])
                .await,
        ));
    }
    // wait for it
    thread::sleep(two_sec);
    for con in consummers {
        let async_result = rpc_client.result(con.1?).await;
        println!("{}-{}", con.0, async_result);
    }
    Ok(())
}
