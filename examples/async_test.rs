use girolle::prelude::*;
use std::time::Instant;
use std::{thread, time};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    let conf: Config = Config::with_yaml_defaults("staging/config.yml".to_string())?;
    let service_name = "video";
    // Create the rpc call struct
    let mut rpc_client = RpcClient::new(conf);
    rpc_client.register_service(service_name).await?;
    rpc_client.start().await?;
    let tempo: time::Duration = time::Duration::from_secs(2);
    let start = Instant::now();
    let mut consummers: Vec<_> = Vec::new();
    println!("Start expensive function");
    for n in 0..1000000 {
        consummers.push((
            n,
            rpc_client.call_async(service_name, "hello", Payload::new().arg(n.to_string()))?,
        ));
    }
    println!("Enter sleep");
    thread::sleep(tempo);
    println!("exit sleep");
    for con in consummers {
        let _async_result = rpc_client.result(&con.1)?;
        println!("{:?}", _async_result.get_result());
    }
    let duration = start.elapsed() - tempo;
    println!("Time elapsed in expensive_function() is: {:?}", duration);
    rpc_client.unregister_service("video")?;
    rpc_client.close().await?;
    Ok(())
}
