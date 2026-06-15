use std::{error::Error as std_error, time::Duration};

use alloy::eips::eip4844::Blob;
use alloy::primitives::B256;
use alloy::rpc::types::beacon::sidecar::{BeaconBlobBundle, GetBlobsResponse};
use anyhow::Error;
use reqwest;

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

    #[deprecated(
        note = "This method is deprecated in favor of get_blobs, which allows fetching only specific blobs."
    )]
    pub async fn get_blob_sidecars(&self, slot: u64) -> Result<BeaconBlobBundle, Error> {
        let res = self
            .get(format!("eth/v1/beacon/blob_sidecars/{slot}").as_str())
            .await?;
        let sidecar: BeaconBlobBundle = serde_json::from_value(res)?;
        Ok(sidecar)
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

    pub async fn get_head_slot_number(&self) -> Result<u64, Error> {
        let headers = self.get("eth/v1/beacon/headers/head").await?;

        let slot = headers
            .get("data")
            .and_then(|data| data.get("header"))
            .and_then(|header| header.get("message"))
            .and_then(|message| message.get("slot"))
            .and_then(|slot| slot.as_str())
            .ok_or(anyhow::anyhow!(
                "get_head_slot_number error: {}",
                "slot is not a string"
            ))?
            .parse::<u64>()
            .map_err(|err| anyhow::anyhow!("get_head_slot_number error: {}", err))?;
        Ok(slot)
    }

    pub async fn get_validators_for_epoch(&self, epoch: u64) -> Result<Vec<String>, Error> {
        let response = self
            .get(format!("eth/v1/validator/duties/proposer/{epoch}").as_str())
            .await?;

        let validators_response = response
            .get("data")
            .ok_or(anyhow::anyhow!(
                "get_validators_for_epoch invalid response body: {}",
                "`data` not found"
            ))?
            .as_array()
            .ok_or(anyhow::anyhow!(
                "get_validators_for_epoch invalid response body: {}",
                "`data` is not an array"
            ))?;

        let mut validators = Vec::with_capacity(32);
        for validator_response in validators_response {
            // This public key is received in the compressed form
            let pubkey = validator_response
                .get("pubkey")
                .and_then(|pubkey| pubkey.as_str())
                .ok_or(anyhow::anyhow!(
                    "get_validators_for_epoch invalid response body: {}",
                    "array element does not contain `pubkey`"
                ))?;

            validators.push(pubkey.to_string());
        }

        Ok(validators)
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
