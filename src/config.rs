use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub private_key: String,
}

pub fn load(filename: &str) -> Config {
    let yaml =
        std::fs::read_to_string(filename).unwrap_or_else(|err| panic!("{} {}", filename, err));
    return serde_yaml::from_str(&yaml).unwrap_or_else(|err| panic!("{} {}", filename, err));
}
