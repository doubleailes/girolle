use crate::error::GirolleError;
/// # nameko_utils
///
/// This module contains functions to manipulate the headers of the AMQP messages.
use lapin::types::{AMQPValue, FieldTable, LongString, ShortString};
use lapin::{message::Delivery, BasicProperties};
use tracing::error;
use uuid::Uuid;

fn set_current_call_id(function_name: &str, id: &str) -> AMQPValue {
    // package_cg_asset.get_filepaths_from_tags.4c5615e2-9367-46aa-8f90-b87e89723fa0
    let rpc_id = format!("{}.{}", function_name.to_string(), id.to_string());
    AMQPValue::LongString(LongString::from(rpc_id.as_bytes()))
}
#[test]
fn test_set_current_call_id() {
    let function_name = "package_cg_asset.get_filepaths_from_tags";
    let id = "4c5615e2-9367-46aa-8f90-b87e89723fa0";
    let rpc_id = set_current_call_id(function_name, id);
    assert_eq!(
        rpc_id,
        AMQPValue::LongString(LongString::from(
            "package_cg_asset.get_filepaths_from_tags.4c5615e2-9367-46aa-8f90-b87e89723fa0"
                .as_bytes()
        ))
    );
}

pub fn insert_new_id_to_call_id(
    mut headers: FieldTable,
    function_name: &str,
    id: &str,
) -> FieldTable {
    let inner_headers = headers.inner();
    let mut call_id_stack = inner_headers
        .get("nameko.call_id_stack")
        .unwrap()
        .as_array()
        .unwrap()
        .clone();
    call_id_stack.push(set_current_call_id(function_name, &id.to_string()));
    let to_amqp = AMQPValue::FieldArray(call_id_stack);
    let key_field = ShortString::from("nameko.call_id_stack");
    headers.insert(key_field, to_amqp);
    headers
}

pub fn get_id(opt_id: &Option<ShortString>, id_name: &str) -> String {
    match opt_id {
        Some(id) => id.to_string(),
        None => {
            error!("{}: None", id_name);
            panic!("{}: None", id_name)
        }
    }
}
#[test]
fn test_get_id() {
    let id = get_id(&Some(ShortString::from("id")), "id");
    assert_eq!(id, "id".to_string());
}

pub fn delivery_to_message_properties(
    delivery: &Delivery,
    id: &Uuid,
    rpc_queue: &str,
) -> Result<lapin::protocol::basic::AMQPProperties, GirolleError> {
    let opt_routing_key = delivery.routing_key.to_string();
    // Get the correlation_id and reply_to_id
    let correlation_id = get_id(delivery.properties.correlation_id(), "correlation_id");
    //need to clone to modify the headers
    let opt_headers = delivery.properties.headers();
    let headers = match opt_headers {
        Some(h) => insert_new_id_to_call_id(h.clone(), &opt_routing_key, &id.to_string()),
        None => {
            error!("No headers found in delivery properties");
            return Err(GirolleError::MissingHeader);
        }
    };
    Ok(BasicProperties::default()
        .with_correlation_id(correlation_id.into())
        .with_content_type("application/json".into())
        .with_reply_to(rpc_queue.into())
        .with_content_encoding("utf-8".into())
        .with_headers(headers)
        .with_delivery_mode(2)
        .with_priority(0))
}
