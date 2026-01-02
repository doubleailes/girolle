use crate::types::{AsyncNamekoFunction, NamekoFunction};
/// # RpcTask
///
/// ## Description
///
/// This struct is used to create a RPC task. This task will be used to register
/// a function in the RpcService struct.
///
/// ## Example
///
/// ```rust,no_run
/// use girolle::prelude::*;
/// use std::vec;
///
/// fn hello(s: &[Value]) -> GirolleResult<Value> {
///    // Parse the incomming data
///    let n: String = serde_json::from_value(s[0].clone())?;
///    let hello_str: Value = format!("Hello, {}!, by Girolle", n).into();
///    Ok(hello_str)
/// }
///  
///
/// fn main() {
///     let mut services: RpcService = RpcService::new(Config::default(),"video");
///     let rpc_task = RpcTask::new("hello", vec!["s"], hello);
/// }
///
#[derive(Clone)]
pub struct RpcTask {
    pub name: &'static str,
    pub args: Vec<&'static str>,
    pub handler: RpcTaskHandler,
}

/// Handler types for RpcTask - either sync or async
#[derive(Clone)]
pub enum RpcTaskHandler {
    /// Legacy synchronous handler
    Sync(NamekoFunction),
    /// New async handler with RpcContext
    Async(AsyncNamekoFunction),
}

impl RpcTask {
    /// # new
    ///
    /// ## Description
    ///
    /// This function create a new RpcTask struct with a synchronous handler
    ///
    /// ## Arguments
    ///
    /// * `name` - The name of the function to call
    /// * `inner_function` - The function to call as NamekoFunction
    ///
    /// ## Returns
    ///
    /// This function return a girolle::RpcTask struct
    ///
    /// ## Example
    ///
    /// ```rust,no_run
    /// use girolle::prelude::*;
    /// use std::vec;
    ///
    /// fn hello(s: &[Value]) -> GirolleResult<Value> {
    ///    // Parse the incomming data
    ///    let n: String = serde_json::from_value(s[0].clone())?;
    ///    let hello_str: Value = format!("Hello, {}!, by Girolle", n).into();
    ///    Ok(hello_str)
    /// }
    ///
    /// fn main() {
    ///     let mut services: RpcService = RpcService::new(Config::default(),"video");
    ///     let rpc_task = RpcTask::new("hello", vec!["s"], hello);
    /// }
    ///
    pub fn new(
        name: &'static str,
        args: Vec<&'static str>,
        inner_function: NamekoFunction,
    ) -> Self {
        Self {
            name,
            args,
            handler: RpcTaskHandler::Sync(inner_function),
        }
    }

    /// # new_async
    ///
    /// ## Description
    ///
    /// Create a new RpcTask with an async handler that receives RpcContext
    ///
    /// ## Arguments
    ///
    /// * `name` - The name of the function to call
    /// * `args` - The argument names
    /// * `async_function` - The async function to call
    ///
    /// ## Returns
    ///
    /// This function return a girolle::RpcTask struct
    pub fn new_async(
        name: &'static str,
        args: Vec<&'static str>,
        async_function: AsyncNamekoFunction,
    ) -> Self {
        Self {
            name,
            args,
            handler: RpcTaskHandler::Async(async_function),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};
    use std::pin::Pin;
    use std::sync::Arc;

    #[test]
    fn test_rpc_task_new_sync() {
        fn test_fn(_args: &[Value]) -> crate::types::GirolleResult<Value> {
            Ok(json!("test"))
        }

        let task = RpcTask::new("test", vec!["arg1"], test_fn);
        assert_eq!(task.name, "test");
        assert_eq!(task.args, vec!["arg1"]);
        match task.handler {
            RpcTaskHandler::Sync(_) => {}
            _ => panic!("Expected Sync handler"),
        }
    }

    #[test]
    fn test_rpc_task_new_async() {
        use crate::rpc_context::RpcContext;

        let async_fn = Arc::new(
            |_ctx: Arc<RpcContext>,
             _data: Vec<Value>|
             -> Pin<Box<dyn std::future::Future<Output = crate::types::GirolleResult<Value>> + Send>>
            {
                Box::pin(async { Ok(json!("async_test")) })
            },
        );

        let task = RpcTask::new_async("test_async", vec!["arg1"], async_fn);
        assert_eq!(task.name, "test_async");
        assert_eq!(task.args, vec!["arg1"]);
        match task.handler {
            RpcTaskHandler::Async(_) => {}
            _ => panic!("Expected Async handler"),
        }
    }
}
