use crate::types::NamekoFunction;
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
///
/// fn hello(s: &[Value]) -> NamekoResult<Value> {
///    // Parse the incomming data
///    let n: String = serde_json::from_value(s[0].clone())?;
///    let hello_str: Value = format!("Hello, {}!, by Girolle", n).into();
///    Ok(hello_str)
/// }
///  
///
/// fn main() {
///     let mut services: RpcService = RpcService::new(Config::default_config(),"video");
///     let rpc_task = RpcTask::new("hello", hello);
///     services.register(rpc_task).start();
/// }
///
#[derive(Clone)]
pub struct RpcTask {
    pub name: &'static str,
    pub inner_function: NamekoFunction,
}
impl RpcTask {
    /// # new
    ///
    /// ## Description
    ///
    /// This function create a new RpcTask struct
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
    ///
    /// fn hello(s: &[Value]) -> NamekoResult<Value> {
    ///    // Parse the incomming data
    ///    let n: String = serde_json::from_value(s[0].clone())?;
    ///    let hello_str: Value = format!("Hello, {}!, by Girolle", n).into();
    ///    Ok(hello_str)
    /// }
    ///
    /// fn main() {
    ///     let mut services: RpcService = RpcService::new(Config::default_config(),"video");
    ///     let rpc_task = RpcTask::new("hello", hello);
    ///     services.register(rpc_task).start();
    /// }
    ///
    pub fn new(name: &'static str, inner_function: NamekoFunction) -> Self {
        Self {
            name,
            inner_function,
        }
    }
}