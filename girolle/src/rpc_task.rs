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
///     let mut services: RpcService = RpcService::new(Config::default_config(),"video");
///     let rpc_task = RpcTask::new("hello", vec!["s"], hello);
/// }
///
#[derive(Clone)]
pub struct RpcTask {
    pub name: &'static str,
    pub args: Vec<&'static str>,
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
    ///     let mut services: RpcService = RpcService::new(Config::default_config(),"video");
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
            inner_function,
        }
    }
}
