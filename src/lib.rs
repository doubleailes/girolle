use bson::Document;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ReponsePayload {
    result: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}
impl ReponsePayload {
    pub fn new(result: String) -> Self {
        Self {
            result,
            error: None,
        }
    }
}
#[derive(Debug, Serialize, Deserialize)]
pub struct IncommingData {
    args: Vec<String>,
    kwargs: Document,
}
impl IncommingData {
    pub fn get_args(&self) -> &Vec<String> {
        &self.args
    }
}
