use girolle::prelude::Payload;
use girolle::{serde_json, Config, RpcClient, Value};
use std::time::Instant;
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
    let p = Payload::new().arg(30);
    let new_result = rpc_client.send(service_name, "fibonacci", p)?;
    let fib_result: u64 = serde_json::from_value(new_result.get_result())?;
    // Print the result
    println!("fibonacci :{:?}", fib_result);
    assert_eq!(fib_result, 832040);
    let sub_result = rpc_client.send(service_name, "sub", Payload::new().arg(10).arg(5))?;
    assert_eq!(sub_result.get_result(), Value::Number(serde_json::Number::from(5)));
    // Create a future result
    let future_result = rpc_client.call_async(service_name, "hello", Payload::new().arg("Toto"))?;
    // Send a message during the previous async process
    let result = rpc_client.send(service_name, "hello", Payload::new().arg("Girolle"))?;
    // Print the result
    println!("{:?}", result.get_result());
    assert_eq!(result.get_result(), Value::String("Hello, Girolle!".to_string()));
    // wait for it
    let tempo: time::Duration = time::Duration::from_secs(4);
    thread::sleep(tempo);
    println!("exit sleep");
    // Print the result
    let async_result = rpc_client.result(&future_result)?;
    println!("{:?}", async_result.get_result());
    assert_eq!(async_result.get_result(), Value::String("Hello, Toto!".to_string()));
    println!("Time elapsed in for the previous call is: {:?}", async_result.get_elapsed_time()-tempo);
    let start = Instant::now();
    let mut consummers: Vec<_> = Vec::new();
    for n in 1..1001 {
        consummers.push((
            n,
            rpc_client.call_async(service_name, "hello", Payload::new().arg(n.to_string())),
        ));
    }
    // wait for it
    thread::sleep(tempo);
    for con in consummers {
        let _async_result = rpc_client.result(&con.1?)?;
    }
    let duration = start.elapsed() - tempo;
    println!("Time elapsed in expensive_function() is: {:?}", duration);
    rpc_client.unregister_service("video")?;
    rpc_client.close().await?;
    Ok(())
}
