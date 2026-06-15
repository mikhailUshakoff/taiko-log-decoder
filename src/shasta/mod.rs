use alloy::{
    providers::{Provider, ProviderBuilder},
    rpc::types::Filter,
    sol_types::SolEvent,
};

use taiko_bindings::inbox::Inbox::Proposed;
use taiko_protocol::shasta::{BlobCoder, manifest::DerivationSourceManifest};

use crate::consensus_layer::ConsensusLayer;

mod config;
use config::Config;

pub async fn run() -> anyhow::Result<()> {
    let cfg = Config::from_env()?;

    let provider = ProviderBuilder::new()
        .connect(&cfg.rpc)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to connect to {}: {e}", cfg.rpc))?;

    let filter = Filter::new()
        .address(cfg.inbox)
        .event(Proposed::SIGNATURE)
        .from_block(cfg.start_block)
        .to_block(cfg.end_block);

    let logs = provider.get_logs(&filter).await.map_err(|e| {
        anyhow::anyhow!(
            "Failed to fetch logs [{} .. {}]: {e}",
            cfg.start_block,
            cfg.end_block
        )
    })?;

    println!("Fetched {} log(s)", logs.len());

    let cl = ConsensusLayer::new(&cfg.beacon_rpc, std::time::Duration::from_secs(10))?;
    let genesis_ts = cl.get_genesis_time().await?;

    for log in logs {
        let block_number = log.block_number.unwrap_or(0);
        let event = Proposed::decode_log(&log.inner)?;

        println!(
            "Block: {block_number}  proposal id: {}  proposer: {}",
            event.data.id, event.data.proposer
        );

        for source in &event.sources {
            println!("  forced inclusion: {}", source.isForcedInclusion);
            println!(
                "  blob slice  offset: {}  timestamp: {}  hashes: {:?}",
                source.blobSlice.offset, source.blobSlice.timestamp, source.blobSlice.blobHashes,
            );
        }

        let source = event
            .sources
            .first()
            .ok_or_else(|| anyhow::anyhow!("Proposal has no sources"))?;

        if source.blobSlice.blobHashes.len() > 1 {
            anyhow::bail!(
                "More than 1 blob hash is not supported (got {})",
                source.blobSlice.blobHashes.len()
            );
        }
        if source.blobSlice.offset != 0 {
            anyhow::bail!(
                "Non-zero blob slice offset is not supported (offset={})",
                source.blobSlice.offset
            );
        }

        let slot = (source.blobSlice.timestamp.to::<u64>() - genesis_ts) / 12;
        let blobs = cl.get_blobs(slot, &source.blobSlice.blobHashes).await?;

        if blobs.len() != 1 {
            anyhow::bail!(
                "Expected exactly 1 blob for slot {slot}, got {}",
                blobs.len()
            );
        }

        let blob_bytes = BlobCoder::decode_blob(&blobs[0])
            .ok_or_else(|| anyhow::anyhow!("Failed to decode blob for slot {slot}"))?;

        println!("Decoded blob: {} byte(s)", blob_bytes.len());

        let manifest = DerivationSourceManifest::decompress_and_decode(
            &blob_bytes,
            source.blobSlice.offset.to::<usize>(),
        )?;

        println!("Manifest: {} block(s)", manifest.blocks.len());
        for (i, block) in manifest.blocks.iter().enumerate() {
            println!("  [{i}] timestamp: {}", block.timestamp);
        }
    }

    Ok(())
}
