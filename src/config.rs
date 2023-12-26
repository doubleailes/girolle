extern crate confy;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfyConfig  {
    rabbitmq_user: String,
    api_key: String,
    rabbitmq_password: String,
    rabitmq_host: String,
    rabbitmq_port: String,
    prefetch_count: u16,
    queue_ttl: u32,
}

impl Default for ConfyConfig {
    fn default() -> Self {
        Self {
            rabbitmq_user: "guest".to_string(),
            api_key: "guest".to_string(),
            rabbitmq_password: "guest".to_string(),
            rabitmq_host: "localhost".to_string(),
            rabbitmq_port: "5672".to_string(),
            prefetch_count: 10,
            queue_ttl: 300000,
        }
    }
}
impl ConfyConfig {
    pub fn get_rabbitmq_user(&self) -> String {
        self.rabbitmq_user.clone()
    }
    pub fn get_rabbitmq_password(&self) -> String {
        self.rabbitmq_password.clone()
    }
    pub fn get_rabitmq_host(&self) -> String {
        self.rabitmq_host.clone()
    }
    pub fn get_rabbitmq_port(&self) -> String {
        self.rabbitmq_port.clone()
    }
    pub fn get_prefetch_count(&self) -> u16 {
        self.prefetch_count
    }
    pub fn get_queue_ttl(&self) -> u32 {
        self.queue_ttl
    }
}

pub fn get_config() -> ConfyConfig {
    confy::load("girolle_config", None).unwrap()
}