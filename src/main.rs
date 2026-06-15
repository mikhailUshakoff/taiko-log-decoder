mod consensus_layer;

use alloy::{
    primitives::{Address, FixedBytes},
    providers::{Provider, ProviderBuilder},
    rpc::types::{Filter},
    sol_types::SolEvent,
};
use std::str::FromStr;

mod bindings;
use bindings::taiko_inbox::ITaikoInbox;

use taiko_bindings::{inbox::Inbox::Proposed};
use taiko_protocol::shasta::manifest::DerivationSourceManifest;
use taiko_protocol::shasta::BlobCoder;

#[tokio::main]
async fn main() {
    println!("Hello, world!");
    shasta().await;
}

async fn shasta() {
    let rpc = "https://ethereum-hoodi-rpc.publicnode.com";
    let beacon_rpc = "https://ethereum-hoodi-beacon-api.publicnode.com/";
    let provider = match ProviderBuilder::new()
        .connect(rpc).await {
        Ok(provider) => provider,
        Err(e) => {
            panic!(
                "Failed to create WebSocket provider: {e}"
            );
        }
    };

    //let shasta_inbox= Address::from_str("0x3477f9e8A890C2286C5E62150ad6593EeF4590b9").unwrap();
    //let start_block = 2_755_667;
    //let end_block = 2_755_667;

    let shasta_inbox = Address::from_str("0xeF4bB7A442Bd68150A3aa61A6a097B86b91700BF").unwrap();
    let start_block = 3021518;
    let end_block = 3021518;

    let filter = Filter::new()
                .address(shasta_inbox)
                .event(Proposed::SIGNATURE)
                .from_block(start_block)
                .to_block(end_block);

    let logs = match provider.get_logs(
            &filter
        ).await {
            Err(e) => {
                println!("Error fetching logs from block {} to {}: {}", start_block, end_block, e);
                return;
            },
            Ok(logs) => {logs},
        };

    println!("---Logs: {}", logs.len());
    for log in logs {
        let event = Proposed::decode_log(&log.inner).unwrap();

        println!("Block: {} Proposal ID: {}, proposer: {}", log.block_number.unwrap_or(0), event.data.id, event.data.proposer);
        event.sources.iter().for_each(|source| {
            source.isForcedInclusion;
            println!("Is forced inclusion: {}", source.isForcedInclusion);
            println!("Blob slice offset: {}, timestamp: {}, blob hashes: {:?}", source.blobSlice.offset, source.blobSlice.timestamp, source.blobSlice.blobHashes);
        });

        if event.sources[0].blobSlice.blobHashes.len() > 1 {
            println!("More than 1 blob hash not supported yet");
            return;
        }

        let cl = consensus_layer::ConsensusLayer::new(beacon_rpc, std::time::Duration::from_secs(10)).unwrap();
        let genesis_ts = cl.get_genesis_time().await.unwrap();
        let slot: u64 = (event.sources[0].blobSlice.timestamp.to::<u64>() - genesis_ts) / 12;
        let blobs = cl.get_blobs(slot, &event.sources[0].blobSlice.blobHashes).await.unwrap();

        if blobs.len() != 1 {
            println!("Only one blob supported for slot {}, blobs {}", slot, blobs.len());
            return;
        }

        if event.sources[0].blobSlice.offset != 0 {
            println!("Non-zero blob slice offset not supported yet");
            return;
        }
        
        

        let blob_bytes = match BlobCoder::decode_blob(&blobs[0]) {
            Some(data) => data,
            None => {
                println!("Error decoding blob");
                return;
            }
        };

        println!("blob_bytes length: {}", blob_bytes.len());
        
        let m = DerivationSourceManifest::decompress_and_decode(&blob_bytes, event.sources[0].blobSlice.offset.to::<usize>()).unwrap();

        println!("Total blocks in manifest: {}", m.blocks.len());
        for (i, block) in m.blocks.iter().enumerate() {
            println!("Block: {}, timestamp: {}", i, block.timestamp);
        }
    }

}

 
async fn pacaya() {
    let rpc = "https://ethereum-rpc.publicnode.com";

    let provider = match ProviderBuilder::new()
        .connect(rpc).await {
        Ok(provider) => provider,
        Err(e) => {
            panic!(
                "Failed to create WebSocket provider: {e}"
            );
        }
    };

    let mut start_block = 23139968;
    let end_block = 23946792;
    let target_coinbase = Address::from_str("0xCbeB5d484b54498d3893A0c3Eb790331962e9e9d").unwrap();

    let mut batch_id = 1321330;

    let mut total_batches = 0;
    let mut total_blocks = 0;
    let mut total_txs = 0;

    while start_block < end_block {
        let mut current_end = start_block + 300;
        if current_end > end_block {
            current_end = end_block;
        }

        let filter = Filter::new()
                .address(Address::from_str("0x06a9Ab27c7e2255df1815E6CC0168d7755Feb19a").unwrap())
                .event_signature(FixedBytes::from_str("0x9eb7fc80523943f28950bbb71ed6d584effe3e1e02ca4ddc8c86e5ee1558c096").unwrap())
                .from_block(start_block)
                .to_block(current_end);

        let logs = match provider.get_logs(
            &filter
        ).await {
            Err(e) => {
                println!("Error fetching logs from block {} to {}: {}", start_block, current_end, e);
                tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                continue;
            },
            Ok(logs) => {logs},
        };

        println!("---Logs: {}", logs.len());
        if logs.len() == 0 {
            println!("========================");
            println!("Total batches: {}", total_batches);
            println!("Total blocks: {}", total_blocks);
            println!("Total txs: {}", total_txs);
            println!("Error logs length is equal to zero from block {} to {}", start_block, current_end);
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
            continue;
        }
        for log in logs {
            let block_num = log.block_number.unwrap_or(0);
            let event = ITaikoInbox::BatchProposed::decode_log(&log.inner).unwrap();

            if batch_id + 1 != event.data.meta.batchId {
                println!("========================");
                println!("Total batches: {}", total_batches);
                println!("Total blocks: {}", total_blocks);
                println!("Total txs: {}", total_txs);
                panic!("Warning: missing batch ID. Previous: {}, current: {}", batch_id, event.data.meta.batchId);
            }
            batch_id = event.data.meta.batchId;

            println!("Block: {} Batch ID: {}, coinbase {}", block_num, event.data.meta.batchId, event.data.info.coinbase);
            if target_coinbase != event.data.info.coinbase {
                continue;
            }
            let block_count = event.data.info.blocks.len();
            println!("Total blocks count in batch: {}", block_count);

            let tx_count: u32 = event.data.info.blocks.iter().map(|block| block.numTransactions as u32).sum();
            println!("Total tx count in batch: {}", tx_count);

            total_batches += 1;
            total_blocks += block_count;
            total_txs += tx_count;
        }

        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        start_block = current_end + 1;
    }

    println!("========================");
    println!("Total batches: {}", total_batches);
    println!("Total blocks: {}", total_blocks);
    println!("Total txs: {}", total_txs);
}
