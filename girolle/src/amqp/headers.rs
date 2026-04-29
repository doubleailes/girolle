//! Manipulation of Nameko-specific AMQP headers.
//!
//! In particular `nameko.call_id_stack`: every hop appends
//! `<routing_key>.<id>`, and the stack is truncated to a configurable
//! window so it doesn't grow unbounded across long call chains.

use crate::error::GirolleError;
use crate::protocol::NAMEKO_CALL_ID_STACK;
use lapin::message::Delivery;
use lapin::types::{AMQPValue, FieldTable, LongString, ShortString};
use lapin::BasicProperties;
use tracing::error;
use uuid::Uuid;

fn set_current_call_id(function_name: &str, id: &str) -> AMQPValue {
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

/// Append a fresh call id to `nameko.call_id_stack` and truncate the
/// stack to `parent_calls_tracked` entries (Nameko semantics).
pub(crate) fn insert_new_id_to_call_id(
    mut headers: FieldTable,
    function_name: &str,
    id: &str,
    parent_calls_tracked: usize,
) -> FieldTable {
    let inner_headers = headers.inner();
    let call_id_stack_slice = inner_headers
        .get(NAMEKO_CALL_ID_STACK)
        .unwrap()
        .as_array()
        .unwrap()
        .as_slice();
    let mut call_id_stack = call_id_stack_slice.to_vec();
    call_id_stack.push(set_current_call_id(function_name, id));

    if call_id_stack.len() > parent_calls_tracked {
        call_id_stack = call_id_stack[call_id_stack.len() - parent_calls_tracked..].to_vec();
    }

    let to_amqp = AMQPValue::FieldArray(call_id_stack.into());
    let key_field = ShortString::from(NAMEKO_CALL_ID_STACK);
    headers.insert(key_field, to_amqp);
    headers
}

/// Read a required `ShortString` header into an owned `String`,
/// panicking if the header is absent.
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

/// Build the `BasicProperties` of a reply message from an inbound
/// delivery: copies the correlation id, sets reply-to to the caller's
/// reply queue, and updates `nameko.call_id_stack` for the next hop.
pub(crate) fn delivery_to_message_properties(
    delivery: &Delivery,
    id: &Uuid,
    rpc_queue: &str,
    parent_calls_tracked: usize,
) -> Result<BasicProperties, GirolleError> {
    let opt_routing_key = delivery.routing_key.to_string();
    let correlation_id = get_id(delivery.properties.correlation_id(), "correlation_id");
    let opt_headers = delivery.properties.headers();
    let headers = match opt_headers {
        Some(h) => insert_new_id_to_call_id(
            h.clone(),
            &opt_routing_key,
            &id.to_string(),
            parent_calls_tracked,
        ),
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
