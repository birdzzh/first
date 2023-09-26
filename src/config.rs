use std::{error::Error, fs};

use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Config {
    pub follows_address_config: Vec<FollowsAddressConfig>,
    pub base_mainnet: BaseMainnetConfig,
    pub account: AccountConfig,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FollowsAddressConfig {
    pub address: String,
    pub amount: u32,
    pub balance: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct BaseMainnetConfig {
    pub ws: String,
    pub https: String,
    pub chain_id: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AccountConfig {
    pub private_key: String,
    pub address: String,
}

impl Config {
    pub fn load(path: &str) -> Result<Config, Box<dyn Error>> {
        match fs::read_to_string(path) {
            Ok(config) => {
                let config: Self = toml::from_str(&config).unwrap();
                Ok(config)
            }
            Err(e) => {
                info!("***** 加载配置文件异常 *****");
                Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    e,
                )))
            }
        }
    }
}
