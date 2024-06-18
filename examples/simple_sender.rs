use girolle::{serde_json, Config, RpcClient, Value};
use std::time::Instant;
use std::vec;
use std::{thread, time};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let conf: Config = Config::with_yaml_defaults("staging/config.yml".to_string())?;
    let service_name = "video";
    // Create the rpc call struct
    let mut rpc_client = RpcClient::new(conf);
    rpc_client.register_service(service_name).await?;
    rpc_client.start().await?;
    // Send the payload
    let new_result = rpc_client.send(service_name, "fibonacci", vec![30])?;
    let fib_result: u64 = serde_json::from_value(new_result)?;
    // Print the result
    println!("fibonacci :{:?}", fib_result);
    assert_eq!(fib_result, 832040);
    let sub_result = rpc_client.send(service_name, "sub", vec![10, 5])?;
    assert_eq!(sub_result, Value::Number(serde_json::Number::from(5)));
    // Create a future result
    let future_result = rpc_client.call_async(service_name, "hello", vec!["Toto"])?;
    // Send a message during the previous async process
    let result = rpc_client.send(service_name, "hello", vec!["Girolle"])?;
    // Print the result
    println!("{:?}", result);
    assert_eq!(result, Value::String("Hello, Girolle!".to_string()));
    // wait for it
    let tempo: time::Duration = time::Duration::from_secs(4);
    thread::sleep(tempo);
    println!("exit sleep");
    // Print the result
    let async_result = rpc_client.result(future_result)?;
    println!("{:?}", async_result);
    assert_eq!(async_result, Value::String("Hello, Toto!".to_string()));
    let start = Instant::now();
    let mut consummers: Vec<_> = Vec::new();
    for n in 1..1001 {
        consummers.push((
            n,
            rpc_client.call_async(service_name, "hello", vec![n.to_string()]),
        ));
    }
    // wait for it
    thread::sleep(tempo);
    for con in consummers {
        let _async_result = rpc_client.result(con.1?)?;
    }
    let duration = start.elapsed() - tempo;
    println!("Time elapsed in expensive_function() is: {:?}", duration);
    rpc_client.unregister_service("video")?;
    rpc_client.close().await?;
    Ok(())
}
