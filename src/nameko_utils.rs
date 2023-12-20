use lapin::types::AMQPValue;

fn nameko_call_id(raw_call_id_stack: &AMQPValue) -> Vec<String> {
    let mut call_id_stack: Vec<String> = Vec::new();
    let var_1 = raw_call_id_stack.as_array().expect("call_id_stack"); // First filed Array
    let var_2 = var_1.as_slice();
    for var in var_2 {
        let var_4 = var.as_long_string().expect("call_id_stack").clone();
        let var_5 = var_4.as_bytes();
        let var_6 = String::from_utf8(var_5.to_vec()).expect("call_id_stack");
        call_id_stack.push(var_6);
    }
    call_id_stack
}

fn set_current_call_id(function_name: &str, id: &str) -> String {
    // package_cg_asset.get_filepaths_from_tags.4c5615e2-9367-46aa-8f90-b87e89723fa0
    format!("{}.{}", function_name.to_string(), id.to_string())
}

pub fn push_call_id(call_id_stack: &AMQPValue, function_name: &str, id: &str) -> Vec<String> {
    let mut new_stack = nameko_call_id(call_id_stack);
    new_stack.push(set_current_call_id(function_name, id));
    new_stack
}
