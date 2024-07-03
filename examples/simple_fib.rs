use girolle::prelude::*;
use serde_json;
use std::fs::File;
use std::io::BufReader;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let conf: Config = Config::with_yaml_defaults("staging/config.yml".to_string())?;
    let service_name = "video";
    // Create the rpc call struct
    let mut rpc_client = RpcClient::new(conf);
    rpc_client.register_service(service_name).await?;
    rpc_client.start().await?;
    let file = File::open("examples/data_set.json")?;
    let reader = BufReader::new(file);
    let batch_payload: Vec<u8> = serde_json::from_reader(reader)?;
    let res: Vec<RpcReply> = batch_payload
        .into_iter()
        .map(|fib_input| {
            rpc_client
                .call_async(service_name, "fibonacci", Payload::new().arg(fib_input))
                .unwrap()
        })
        .collect();
    let _r: Vec<f64> = res
        .into_iter()
        .map(|reply| rpc_client.result(&reply).unwrap().get_value())
        .map(|v| serde_json::from_value(v).unwrap())
        .collect();
    Ok(())
}
