use std::fs;
use solana_transaction_status::UiConfirmedBlock;
use crate::source::{SourceEvent, SourceStream};
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

impl SourceStream for Benchmark {
    async fn next(&mut self) -> SourceEvent {
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
            return SourceEvent::EndOfStream;
        }
        let next_block = Block::from(self.blocks[self.current_block].clone());
        log::debug!(
            "Block: {} Transactions: {}",
            next_block.height,
            next_block.transactions.len()
        );
        self.current_block += 1;
        SourceEvent::Next(next_block)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;
    use super::*;

    #[tokio::test]
    async fn benchmark() {
        let mut benchmark = Benchmark::new("benchmark.json".to_string());
        for height in 288381105..288381120 {
            let event = benchmark.next().await;
            match event {
                SourceEvent::Next(block) => {
                    assert_eq!(block.height, height);
                }
                SourceEvent::Failure(error) => {
                    assert!(false, "unexpected failure {}", error)
                }
                SourceEvent::EndOfStream => {
                    assert!(height >= 288381116)
                }
            }
        }
        let elapsed = benchmark.measure.unwrap().elapsed();
        assert!(elapsed < Duration::from_millis(50), "duration {:?}", elapsed);
    }
}