use alloy::{
    providers::{Provider, ProviderBuilder},
    rpc::types::Filter,
    sol_types::SolEvent,
};

mod bindings;
use bindings::taiko_inbox::ITaikoInbox;

mod config;
use config::Config;

#[derive(Default, Debug)]
struct Stats {
    total_batches: u64,
    total_blocks: usize,
    total_txs: u32,
}

impl Stats {
    fn print(&self) {
        println!("========================");
        println!("Total batches : {}", self.total_batches);
        println!("Total blocks  : {}", self.total_blocks);
        println!("Total txs     : {}", self.total_txs);
    }
}

pub async fn run() -> anyhow::Result<()> {
    let cfg = Config::from_env()?;

    let provider = ProviderBuilder::new()
        .connect(&cfg.rpc)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to connect to {}: {e}", cfg.rpc))?;

    let mut start_block = cfg.start_block;
    let mut batch_id = cfg.initial_batch_id;
    let mut stats = Stats::default();

    while start_block <= cfg.end_block {
        let end_block = (start_block + cfg.block_step).min(cfg.end_block);

        let filter = Filter::new()
            .address(cfg.inbox)
            .event_signature(cfg.batch_proposed_topic)
            .from_block(start_block)
            .to_block(end_block);

        let logs = match provider.get_logs(&filter).await {
            Ok(logs) => logs,
            Err(e) => {
                eprintln!(
                    "RPC error [{start_block}..{end_block}]: {e} — retrying in {}s",
                    cfg.retry_delay_secs
                );
                tokio::time::sleep(std::time::Duration::from_secs(cfg.retry_delay_secs)).await;
                continue; // retry the same range
            }
        };

        println!("Range [{start_block}..{end_block}]: {} log(s)", logs.len());

        if logs.is_empty() {
            start_block = end_block + 1;
            tokio::time::sleep(std::time::Duration::from_secs(cfg.poll_delay_secs)).await;
            continue;
        }

        for log in &logs {
            let block_num = log.block_number.unwrap_or(0);
            let event = ITaikoInbox::BatchProposed::decode_log(&log.inner)?;
            let current_id = event.data.meta.batchId;

            if current_id != batch_id + 1 {
                stats.print();
                anyhow::bail!("Batch ID gap: expected {}, got {current_id}", batch_id + 1);
            }
            batch_id = current_id;

            println!(
                "  Block: {block_num}  batch: {current_id}  coinbase: {}",
                event.data.info.coinbase
            );

            if event.data.info.coinbase != cfg.target_coinbase {
                continue;
            }

            let block_count = event.data.info.blocks.len();
            let tx_count: u32 = event
                .data
                .info
                .blocks
                .iter()
                .map(|b| b.numTransactions as u32)
                .sum();

            println!("    blocks: {block_count}  txs: {tx_count}");

            stats.total_batches += 1;
            stats.total_blocks += block_count;
            stats.total_txs += tx_count;
        }

        start_block = end_block + 1;
        tokio::time::sleep(std::time::Duration::from_secs(cfg.poll_delay_secs)).await;
    }

    stats.print();
    Ok(())
}
