use crate::env_reader::{env_or, require_env};
use alloy::primitives::{Address, FixedBytes};
use std::str::FromStr;

pub struct Config {
    pub rpc: String,
    pub inbox: Address,
    pub batch_proposed_topic: FixedBytes<32>,
    pub target_coinbase: Address,
    pub start_block: u64,
    pub end_block: u64,
    pub block_step: u64,
    pub retry_delay_secs: u64,
    pub poll_delay_secs: u64,
    pub initial_batch_id: u64,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            rpc: require_env("PACAYA_RPC")?,
            inbox: Address::from_str(&require_env("PACAYA_INBOX_ADDRESS")?)?,
            batch_proposed_topic: FixedBytes::from_str(&require_env(
                "PACAYA_BATCH_PROPOSED_TOPIC",
            )?)?,
            target_coinbase: Address::from_str(&require_env("PACAYA_TARGET_COINBASE")?)?,
            start_block: require_env("PACAYA_START_BLOCK")?.parse()?,
            end_block: require_env("PACAYA_END_BLOCK")?.parse()?,
            block_step: env_or("PACAYA_BLOCK_STEP", 300)?,
            retry_delay_secs: env_or("PACAYA_RETRY_DELAY_SECS", 10)?,
            poll_delay_secs: env_or("PACAYA_POLL_DELAY_SECS", 1)?,
            initial_batch_id: require_env("PACAYA_INITIAL_BATCH_ID")?.parse()?,
        })
    }
}
