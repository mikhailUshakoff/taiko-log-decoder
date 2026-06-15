use crate::env_reader::require_env;
use alloy::primitives::Address;
use std::str::FromStr;
pub struct Config {
    pub rpc: String,
    pub beacon_rpc: String,
    pub inbox: Address,
    pub start_block: u64,
    pub end_block: u64,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            rpc: require_env("SHASTA_RPC")?,
            beacon_rpc: require_env("SHASTA_BEACON_RPC")?,
            inbox: Address::from_str(&require_env("SHASTA_INBOX_ADDRESS")?)?,
            start_block: require_env("SHASTA_START_BLOCK")?.parse()?,
            end_block: require_env("SHASTA_END_BLOCK")?.parse()?,
        })
    }
}
