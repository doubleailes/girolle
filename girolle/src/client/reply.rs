use serde_json::Value;
use std::time::{Duration, SystemTime, SystemTimeError};
use uuid::Uuid;

/// Handle returned by [`super::RpcClient::call_async`].
///
/// Carries the correlation id used to match the eventual reply and a
/// timestamp captured at send time.
pub struct RpcReply {
    correlation_id: Uuid,
    time_stamp: SystemTime,
}

impl RpcReply {
    pub(crate) fn new(correlation_id: Uuid) -> Self {
        Self {
            correlation_id,
            time_stamp: SystemTime::now(),
        }
    }

    pub fn get_correlation_id(&self) -> String {
        self.correlation_id.to_string()
    }

    pub fn get_elapsed_time(&self) -> Result<Duration, SystemTimeError> {
        self.time_stamp.elapsed()
    }
}

/// The decoded result of an RPC call plus the wall-clock time it took.
pub struct RpcResult {
    result: Value,
    elapsed_time: Duration,
}

impl RpcResult {
    pub(crate) fn new(result: Value, elapsed_time: Duration) -> Self {
        Self {
            result,
            elapsed_time,
        }
    }

    /// The decoded JSON value returned by the remote service.
    pub fn get_value(&self) -> Value {
        self.result.clone()
    }

    /// Time elapsed between the call being sent and the reply being
    /// matched (or wrapped, in `send`).
    pub fn get_elapsed_time(&self) -> Duration {
        self.elapsed_time
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_rpc_result_new() {
        let result = json!({"success": true});
        let elapsed_time = Duration::from_secs(5);
        let rpc_result = RpcResult::new(result.clone(), elapsed_time);

        assert_eq!(rpc_result.get_value(), result);
        assert_eq!(rpc_result.get_elapsed_time(), elapsed_time);
    }

    #[test]
    fn test_rpc_result_get_value() {
        let result = json!({"success": true});
        let rpc_result = RpcResult::new(result.clone(), Duration::from_secs(5));
        assert_eq!(rpc_result.get_value(), result);
    }

    #[test]
    fn test_rpc_result_get_elapsed_time() {
        let elapsed_time = Duration::from_secs(5);
        let rpc_result = RpcResult::new(json!({"success": true}), elapsed_time);
        assert_eq!(rpc_result.get_elapsed_time(), elapsed_time);
    }

    #[test]
    fn test_rpc_reply_new() {
        let correlation_id = Uuid::new_v4();
        let rpc_reply = RpcReply::new(correlation_id);
        assert_eq!(rpc_reply.get_correlation_id(), correlation_id.to_string());
    }

    #[test]
    fn test_rpc_reply_get_correlation_id() {
        let correlation_id = Uuid::new_v4();
        let rpc_reply = RpcReply::new(correlation_id);
        assert_eq!(rpc_reply.get_correlation_id(), correlation_id.to_string());
    }

    #[test]
    fn test_rpc_reply_get_elapsed_time() {
        let rpc_reply = RpcReply::new(Uuid::new_v4());
        let elapsed_time = rpc_reply.get_elapsed_time().unwrap();
        assert!(elapsed_time < Duration::from_secs(1));
    }
}
