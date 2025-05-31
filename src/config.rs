use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

pub static CONFIG: OnceLock<Config> = OnceLock::new();

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub private_key: String,
    pub geth_url: String,
    pub psql: String,
    pub etherscan_key: String,
}

pub fn load(filename: &str) -> Config {
    let yaml =
        std::fs::read_to_string(filename).unwrap_or_else(|err| panic!("{} {}", filename, err));
    return serde_yaml::from_str(&yaml).unwrap_or_else(|err| panic!("{} {}", filename, err));
}
