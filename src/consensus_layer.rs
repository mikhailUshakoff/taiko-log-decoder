use std::{error::Error as std_error, time::Duration};

use alloy::eips::eip4844::Blob;
use alloy::primitives::B256;
use alloy::rpc::types::beacon::sidecar::GetBlobsResponse;
use anyhow::Error;

pub struct ConsensusLayer {
    client: reqwest::Client,
    url: reqwest::Url,
}

impl ConsensusLayer {
    pub fn new(rpc_url: &str, timeout: Duration) -> Result<Self, Error> {
        if !rpc_url.ends_with('/') {
            return Err(anyhow::anyhow!("Consensus layer URL must end with '/'"));
        }
        let client = reqwest::Client::builder().timeout(timeout).build()?;
        Ok(Self {
            client,
            url: reqwest::Url::parse(rpc_url)?,
        })
    }

    pub async fn get_blobs(
        &self,
        slot: u64,
        versioned_hashes: &[B256],
    ) -> Result<Vec<Blob>, Error> {
        let mut path = format!("eth/v1/beacon/blobs/{slot}");
        if !versioned_hashes.is_empty() {
            let hashes = versioned_hashes
                .iter()
                .map(|hash| hash.to_string())
                .collect::<Vec<_>>()
                .join(",");
            path = format!("{path}?versioned_hashes={hashes}");
        }

        let res = self.get(path.as_str()).await?;
        let blobs: GetBlobsResponse = serde_json::from_value(res)?;
        Ok(blobs.data)
    }

    pub async fn get_genesis_time(&self) -> Result<u64, Error> {
        let genesis = self.get("eth/v1/beacon/genesis").await?;
        let genesis_time = genesis
            .get("data")
            .and_then(|data| data.get("genesis_time"))
            .and_then(|genesis_time| genesis_time.as_str())
            .ok_or_else(|| {
                anyhow::anyhow!("get_genesis_time error: missing or invalid 'genesis_time' field")
            })?
            .parse::<u64>()
            .map_err(|err| anyhow::anyhow!("get_genesis_time error: {}", err))?;
        Ok(genesis_time)
    }

    async fn get(&self, path: &str) -> Result<serde_json::Value, Error> {
        let start = std::time::Instant::now();
        let response = self
            .client
            .get(self.url.join(path)?)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    anyhow::anyhow!("Consensus layer request timed out: {}", path)
                } else {
                    anyhow::anyhow!(
                        "Consensus layer request failed with error: {}. Source: {:?}",
                        e,
                        e.source()
                    )
                }
            })?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Consensus layer request ({}) failed with status: {}",
                path,
                response.status()
            ));
        }

        let body = response.text().await?;
        let v: serde_json::Value = serde_json::from_str(&body)?;
        println!(
            "ConsensusLayer ({}) took {} ms",
            path,
            start.elapsed().as_millis()
        );
        Ok(v)
    }
}
