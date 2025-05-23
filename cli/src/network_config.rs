use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone)]
pub struct NetworkConfigs {
    pub mainnet: NetworkConfig,
    pub testnet: NetworkConfig,
    pub localnet: NetworkConfig,
}

impl NetworkConfigs {
    fn path() -> PathBuf {
        PathBuf::from("./config.json")
    }

    pub fn load() -> Result<Self, anyhow::Error> {
        let path = Self::path();
        Ok(if let Ok(file) = std::fs::File::open(path) {
            serde_json::from_reader(file)?
        } else {
            let new = Self {
                mainnet: NetworkConfig::new("https://sui-rpc.publicnode.com"),
                testnet: NetworkConfig::new("https://fullnode.testnet.sui.io:443"),
                localnet: NetworkConfig::new("http://127.0.0.1:9000"),
            };
            new.update()?;
            new
        })
    }

    pub fn update(&self) -> Result<(), anyhow::Error> {
        serde_json::to_writer_pretty(std::fs::File::create(Self::path())?, &self)
            .map_err(anyhow::Error::from)
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct NetworkConfig {
    pub rpc: String,
    pub keys: Vec<String>,
    pub deployment: Option<Deployment>,
}

impl NetworkConfig {
    pub fn new(rpc: &str) -> Self {
        Self {
            rpc: rpc.to_string(),
            keys: vec!["admin".to_string()],
            deployment: None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Deployment {
    pub package: String,
    pub admin_cap: String,
    pub registry: String,
    pub monster_minter: String,
    pub drive_minter: String,
    pub distribution_table: String,
}
