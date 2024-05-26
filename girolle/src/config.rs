use regex::Captures;
use regex::Regex;
use serde::Deserialize;
use serde_yaml;
use std::borrow::Cow;
use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::Read;

/// # Config
///
/// ## Description
///
/// This struct is used to store the configuration of the AMQP configuration.
/// It is used to store the configuration of the AMQP connection
/// - the web server address, the RPC exchange
/// - the serializer
/// - the serializers
/// - the accept
/// - the heartbeat
/// - the AMQP SSL
/// - the transport options
/// - the login method
/// - the max workers
/// - the prefetch count
/// - the parent calls tracked
/// - the call id stack
/// - the auth token
/// - the language
/// - the user id
/// - the user agent
/// - the header prefix
/// - the non persistent
/// - the persistent.
#[allow(non_snake_case)]
#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    AMQP_URI: Option<String>,
    web_server_address: Option<String>,
    rpc_exchange: Option<String>,
    serializer: Option<String>,
    serializers: Option<String>,
    accept: Option<String>,
    heartbeat: Option<u16>,
    amqp_ssl: Option<String>,
    transport_options: Option<HashMap<String, u32>>,
    login_method: Option<String>,
    max_workers: Option<u32>,
    prefetch_count: Option<u16>,
    parent_calls_tracked: Option<u32>,
    call_id_stack: Option<String>,
    auth_token: Option<String>,
    language: Option<String>,
    user_id: Option<String>,
    user_agent: Option<String>,
    header_prefix: Option<String>,
    non_persistent: Option<u8>,
    persistent: Option<u8>,
}

impl Config {
    // Create a default config
    pub fn default_config() -> Self {
        Self {
            AMQP_URI: Some("amqp://guest:guest@localhost/".to_string()),
            web_server_address: Some("".to_string()),
            rpc_exchange: Some("nameko-rpc".to_string()),
            serializer: Some("json".to_string()),
            serializers: Some("".to_string()),
            accept: Some("".to_string()),
            heartbeat: Some(60),
            amqp_ssl: Some("".to_string()),
            transport_options: Some({
                let mut map = HashMap::new();
                map.insert("max_retries".to_string(), 3);
                map.insert("interval_start".to_string(), 2);
                map.insert("interval_step".to_string(), 1);
                map.insert("interval_max".to_string(), 5);
                map
            }),
            login_method: Some("".to_string()),
            max_workers: Some(10),
            prefetch_count: Some(10),
            parent_calls_tracked: Some(10),
            call_id_stack: Some("call_id_stack".to_string()),
            auth_token: Some("auth_token".to_string()),
            language: Some("language".to_string()),
            user_id: Some("user_id".to_string()),
            user_agent: Some("user_agent".to_string()),
            header_prefix: Some("HEADER_PREFIX".to_string()),
            non_persistent: Some(1),
            persistent: Some(2),
        }
    }

    // Function to load from a YAML file and merge with default config
    pub fn with_yaml_defaults(file_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let default_config = Config::default_config();

        let mut file = File::open(file_path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        contents = expand_var(&contents).to_string();
        let overrides: Config = serde_yaml::from_str(&contents)?;
        Ok(default_config.merge(overrides))
    }

    // Function to merge two configs
    fn merge(self, other: Config) -> Config {
        Config {
            AMQP_URI: other.AMQP_URI.or(self.AMQP_URI),
            web_server_address: other.web_server_address.or(self.web_server_address),
            rpc_exchange: other.rpc_exchange.or(self.rpc_exchange),
            serializer: other.serializer.or(self.serializer),
            serializers: other.serializers.or(self.serializers),
            accept: other.accept.or(self.accept),
            heartbeat: other.heartbeat.or(self.heartbeat),
            amqp_ssl: other.amqp_ssl.or(self.amqp_ssl),
            transport_options: other.transport_options.or(self.transport_options),
            login_method: other.login_method.or(self.login_method),
            max_workers: other.max_workers.or(self.max_workers),
            prefetch_count: other.prefetch_count.or(self.prefetch_count),
            parent_calls_tracked: other.parent_calls_tracked.or(self.parent_calls_tracked),
            call_id_stack: other.call_id_stack.or(self.call_id_stack),
            auth_token: other.auth_token.or(self.auth_token),
            language: other.language.or(self.language),
            user_id: other.user_id.or(self.user_id),
            user_agent: other.user_agent.or(self.user_agent),
            header_prefix: other.header_prefix.or(self.header_prefix),
            non_persistent: other.non_persistent.or(self.non_persistent),
            persistent: other.persistent.or(self.persistent),
        }
    }
    // Get the AMQP URI
    #[allow(non_snake_case)]
    pub fn AMQP_URI(&self) -> String {
        self.AMQP_URI.clone().unwrap()
    }
    // Get the prefetch count
    pub fn prefetch_count(&self) -> u16 {
        self.prefetch_count.unwrap()
    }
    // Get the heartbeat
    pub fn heartbeat(&self) -> u16 {
        self.heartbeat.unwrap()
    }
    // Get the RPC exchange
    pub fn rpc_exchange(&self) -> &str {
        self.rpc_exchange.as_ref().unwrap()
    }
    // Set the AMQP URI
    pub fn with_amqp_uri(&self, amqp_uri: &str) -> Config {
        let mut new_config = self.clone();
        new_config.AMQP_URI = Some(amqp_uri.to_string());
        new_config
    }
    // Set the prefetch count
    pub fn with_prefetch_count(&self, prefetch_count: u16) -> Config {
        let mut new_config = self.clone();
        new_config.prefetch_count = Some(prefetch_count);
        new_config
    }
    // Set the heartbeat
    pub fn with_heartbeat(&self, heartbeat: u16) -> Config {
        let mut new_config = self.clone();
        new_config.heartbeat = Some(heartbeat);
        new_config
    }
    // Set the RPC exchange
    pub fn with_rpc_exchange(&self, rpc_exchange: &str) -> Config {
        let mut new_config = self.clone();
        new_config.rpc_exchange = Some(rpc_exchange.to_string());
        new_config
    }
}

fn expand_var(raw_config: &str) -> Cow<str> {
    let re = Regex::new(r"\$\{([a-zA-Z_][0-9a-zA-Z_]*)\}").unwrap();
    re.replace_all(&raw_config, |caps: &Captures| match env::var(&caps[1]) {
        Ok(val) => val,
        Err(_) => (&caps[0]).to_string(),
    })
}

#[test]
fn test_expand_var() {
    env::set_var("HOME", "/home/user");
    assert_eq!(expand_var("${HOME}/.config/"), "/home/user/.config/");
    env::remove_var("HOME");
    assert_eq!(expand_var("${HOME}/.config"), "${HOME}/.config");
}

#[test]
fn test_config_default() {
    let config = Config::default_config();
    assert_eq!(config.AMQP_URI(), "amqp://guest:guest@localhost/");
    assert_eq!(config.prefetch_count(), 10);
}

#[test]
fn test_config_with_yaml_defaults() {
    let config = Config::with_yaml_defaults("../examples/config.yml").unwrap();
    assert_eq!(config.AMQP_URI(), "amqp://toto:super@$172.16.1.1/");
    assert_eq!(config.prefetch_count(), 10);
    assert_eq!(config.heartbeat(), 60);
    assert_eq!(config.rpc_exchange(), "nameko-rpc");
}
