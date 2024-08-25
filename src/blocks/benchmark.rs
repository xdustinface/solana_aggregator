use std::fs;
use solana_transaction_status::UiConfirmedBlock;
use crate::blocks::{BlockEvent, BlockStream};
use crate::types::Block;
use std::time::Instant;

pub struct Benchmark {
    current_block: usize,
    blocks: Vec<UiConfirmedBlock>,
    measure: Option<Instant>,
}

impl Benchmark {
    pub fn new(path: String) -> Self {
        let benchmark_data = fs::read_to_string(path)
            .expect("Failed to read benchmark.json");

        Self {
            current_block: 0,
            blocks: serde_json::from_str(&benchmark_data).unwrap(),
            measure: None,
        }
    }
}

impl BlockStream for Benchmark {
    async fn next(&mut self) -> BlockEvent {
        if self.measure.is_none() {
            log::debug!("Benchmark start with {} blocks", self.blocks.len());
            self.measure = Some(Instant::now());
        }
        if self.current_block >= self.blocks.len() {
            if let Some(measure) = self.measure {
                log::debug!(
                    "Benchmark done with {} blocks in {:?}", self.blocks.len(), measure.elapsed()
                );
            }
            return BlockEvent::EndOfStream;
        }
        let next_block = Block::from(self.blocks[self.current_block].clone());
        log::debug!(
            "Block: {} Transactions: {}",
            next_block.height,
            next_block.transactions.len()
        );
        self.current_block += 1;
        BlockEvent::Next(next_block)
    }
}