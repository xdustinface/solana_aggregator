use crate::blocks::{BlockEvent, BlockStream};
use crate::error::Result;
use crate::storage::StorageInterface;
use crate::types::Block;

pub struct Aggregator<Source>
where
    Source: BlockStream,
{
    source: Source,
    storage: StorageInterface,
}

impl <Source> Aggregator<Source>
where
    Source: BlockStream,
{
    pub fn new(source: Source, storage: StorageInterface) -> Self {
        Self {source, storage}
    }

    pub async fn run(&mut self) {
        loop {
            let event = self.source.next().await;
            match event {
                BlockEvent::Next(block) => {
                    match self.process_block(block).await {
                        Ok(_) => {continue}
                        Err(error) => {
                            log::error!("Failed to process block: {}", error);
                            return
                        }
                    }
                }
                BlockEvent::Failure(error) => {
                    log::error!("Stream broken: {}", error);
                    return
                }
                BlockEvent::EndOfStream => {
                    log::info!("End of stream");
                    return
                }
            }
        }
    }

    async fn process_block(&mut self, block: Block) -> Result<()> {
        if block.transactions.len() != 0 {
            self.storage.add_block(block).await?;
        }
        Ok(())
    }
}