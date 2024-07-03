use girolle::prelude::*;
#[macro_use]
extern crate rocket;

async fn simple_client(n: u64) -> Result<u64, Box<dyn std::error::Error>> {
    let conf: Config = Config::with_yaml_defaults("staging/config.yml".to_string())?;
    let service_name = "video";
    // Create the rpc call struct
    let mut rpc_client = RpcClient::new(conf);
    rpc_client.register_service(service_name).await?;
    rpc_client.start().await?;
    // Send the payload
    let new_result = rpc_client.send(service_name, "fibonacci", Payload::new().arg(n))?;
    let fib_result: u64 = serde_json::from_value(new_result.get_value())?;
    Ok(fib_result)
}
#[get("/<name>/<age>")]
fn hello(name: &str, age: u8) -> String {
    format!("Hello, {} year old named {}!", age, name)
}

#[get("/fibonacci/<n>")]
fn fibonacci(n: u64) -> String {
    match simple_client(n) {
        Ok(fib_result) => format!("fibonacci :{:?}", fib_result),
        Err(e) => format!("Error: {:?}", e),
    }
}



#[launch]
fn rocket() -> _ {
    rocket::build().mount("/hello", routes![hello])
}
