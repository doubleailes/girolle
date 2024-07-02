use crate::error::GirolleError;
use crate::payload::{Payload, PayloadResult};
use crate::rpc_task::RpcTask;
use lapin::options::BasicPublishOptions;
/// # nameko_utils
///
/// This module contains functions to manipulate the headers of the AMQP messages.
use lapin::types::{AMQPValue, FieldTable, LongString, ShortString};
use lapin::Channel;
use lapin::{message::Delivery, BasicProperties};
use serde_json::Value;
use std::collections::HashMap;
use tracing::error;
use uuid::Uuid;

fn set_current_call_id(function_name: &str, id: &str) -> AMQPValue {
    // package_cg_asset.get_filepaths_from_tags.4c5615e2-9367-46aa-8f90-b87e89723fa0
    let rpc_id = format!("{}.{}", function_name, id);
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

pub(crate) fn insert_new_id_to_call_id(
    mut headers: FieldTable,
    function_name: &str,
    id: &str,
    parent_calls_tracked: usize,
) -> FieldTable {
    let inner_headers = headers.inner();
    let call_id_stack_slice = inner_headers
        .get("nameko.call_id_stack")
        .unwrap()
        .as_array()
        .unwrap()
        .as_slice();
    let mut call_id_stack = call_id_stack_slice.to_vec();
    call_id_stack.push(set_current_call_id(function_name, &id.to_string()));

    // Keep only the last 10 elements in a simpler way
    if call_id_stack.len() > parent_calls_tracked {
        call_id_stack = call_id_stack[call_id_stack.len() - parent_calls_tracked..].to_vec();
    }

    let to_amqp = AMQPValue::FieldArray(call_id_stack.into());
    let key_field = ShortString::from("nameko.call_id_stack");
    headers.insert(key_field, to_amqp);
    headers
}

pub(crate) fn get_id(opt_id: &Option<ShortString>, id_name: &str) -> String {
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

pub(crate) fn delivery_to_message_properties(
    delivery: &Delivery,
    id: &Uuid,
    rpc_queue: &str,
    parent_calls_tracked: usize,
) -> Result<lapin::protocol::basic::AMQPProperties, GirolleError> {
    let opt_routing_key = delivery.routing_key.to_string();
    // Get the correlation_id and reply_to_id
    let correlation_id = get_id(delivery.properties.correlation_id(), "correlation_id");
    //need to clone to modify the headers
    let opt_headers = delivery.properties.headers();
    let headers = match opt_headers {
        Some(h) => insert_new_id_to_call_id(h.clone(), &opt_routing_key, &id.to_string(),parent_calls_tracked),
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

pub(crate) async fn publish(
    rpc_channel: &Channel,
    payload: PayloadResult,
    properties: BasicProperties,
    reply_to_id: String,
    rpc_exchange: &str,
) -> lapin::Result<()> {
    // Need to clone the rpc_channel to be able to use it in the tokio::spawn
    let rpc_channel_clone = rpc_channel.clone();
    let rpc_exchange_clone = rpc_exchange.to_string();
    tokio::spawn(async move {
        rpc_channel_clone
            .basic_publish(
                &rpc_exchange_clone,
                &reply_to_id.to_string(),
                BasicPublishOptions::default(),
                payload
                    .to_string()
                    .expect("can't serialize payload")
                    .as_bytes(),
                properties,
            )
            .await
            .unwrap()
            .await
            .unwrap();
    });
    // The message was correctly published
    Ok(())
}

fn push_values_to_result(
    service_args: &[&str],
    kwargs: &HashMap<String, Value>,
    start: usize,
    end: usize,
) -> Result<Vec<Value>, GirolleError> {
    service_args.iter()
    .take(end)
    .skip(start)
    .map(|arg| {
        kwargs
            .get(&arg.to_string())
            .cloned()
            .ok_or_else(|| GirolleError::IncorrectSignature("Key is missing in kwargs".to_string()))
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use serde_json::Value;

    #[test]
    fn test_push_values_to_result_success() {
        let service_args = vec!["arg1", "arg2"];
        let mut kwargs = HashMap::new();
        kwargs.insert("arg1".to_string(), Value::String("value1".to_string()));
        kwargs.insert("arg2".to_string(), Value::String("value2".to_string()));

        let result = push_values_to_result(&service_args, &kwargs, 0, 2);
        assert!(result.is_ok());
        let values = result.unwrap();
        assert_eq!(values, vec![Value::String("value1".to_string()), Value::String("value2".to_string())]);
    }

    #[test]
    fn test_push_values_to_result_missing_key() {
        let service_args = vec!["arg1", "arg3"]; // arg3 does not exist in kwargs
        let mut kwargs = HashMap::new();
        kwargs.insert("arg1".to_string(), Value::String("value1".to_string()));

        let result = push_values_to_result(&service_args, &kwargs, 0, 2);
        assert!(result.is_err());
        match result {
            Err(GirolleError::IncorrectSignature(msg)) => {
                assert_eq!(msg, "Key is missing in kwargs".to_string());
            },
            _ => panic!("Expected GirolleError::IncorrectSignature"),
        }
    }
}

fn build_inputs_fn_service(
    service_args: &[&str],
    data_delivery: Payload,
) -> Result<Vec<Value>, GirolleError> {
    let args_size: usize = data_delivery.args.len();
    let kwargs_size: usize = data_delivery.kwargs.len();
    let service_args_size: usize = service_args.len();

    match (
        data_delivery.kwargs.is_empty(),
        data_delivery.args.is_empty(),
        service_args_size == args_size + kwargs_size,
    ) {
        (true, _, _) if service_args_size == args_size => Ok(data_delivery.args),
        (_, true, _) if service_args_size == kwargs_size => {
            push_values_to_result(service_args, &data_delivery.kwargs, 0, kwargs_size)
        }
        (_, _, true) => {
            let mut result = data_delivery.args;
            result.extend(push_values_to_result(
                service_args,
                &data_delivery.kwargs,
                args_size,
                args_size + kwargs_size,
            )?);
            Ok(result)
        }
        _ => Err(GirolleError::IncorrectSignature(format!(
            "takes {} positional arguments but {} were given",
            service_args_size,
            args_size + kwargs_size
        ))),
    }
}

#[test]
fn test_build_inputs_fn_service() {
    let service_args = vec!["a", "b", "c"];
    let data_delivery = Payload {
        args: vec![
            Value::String("1".to_string()),
            Value::String("2".to_string()),
        ],
        kwargs: HashMap::from([("c".to_string(), Value::String("3".to_string()))]),
    };
    let result = build_inputs_fn_service(&service_args, data_delivery);
    assert_eq!(result.is_ok(), true);
    let result = result.unwrap();
    assert_eq!(result.len(), 3);
    assert_eq!(result[0], Value::String("1".to_string()));
    assert_eq!(result[1], Value::String("2".to_string()));
    assert_eq!(result[2], Value::String("3".to_string()));
}

/// Execute the delivery
pub(crate) async fn compute_deliver(
    incomming_data: Payload,
    properties: BasicProperties,
    rpc_task_struct: &RpcTask,
    rpc_channel: &Channel,
    rpc_exchange: &str,
    reply_to_id: String,
) {
    // Publish the response
    let fn_service = rpc_task_struct.inner_function;
    let buildted_args = match build_inputs_fn_service(&rpc_task_struct.args, incomming_data) {
        Ok(result) => result,
        Err(error) => {
            publish(
                rpc_channel,
                PayloadResult::from_error(error.convert()),
                properties,
                reply_to_id,
                rpc_exchange,
            )
            .await
            .expect("Error publishing");
            return;
        }
    };
    match fn_service(&buildted_args) {
        Ok(result) => {
            publish(
                rpc_channel,
                PayloadResult::from_result_value(result),
                properties,
                reply_to_id,
                rpc_exchange,
            )
            .await
            .expect("Error publishing");
        }
        Err(error) => {
            publish(
                rpc_channel,
                PayloadResult::from_error(error.convert()),
                properties,
                reply_to_id,
                rpc_exchange,
            )
            .await
            .expect("Error publishing");
        }
    }
}
