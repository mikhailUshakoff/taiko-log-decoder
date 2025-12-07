use alloy::{
    primitives::{Address, FixedBytes},
    providers::{Provider, ProviderBuilder},
    rpc::types::{Filter},
    sol_types::SolEvent,
};
use std::str::FromStr;

mod bindings;
use bindings::taiko_inbox::ITaikoInbox;

#[tokio::main]
async fn main() {
    println!("Hello, world!");

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
