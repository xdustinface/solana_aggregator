use tokio_util::sync::CancellationToken;
use crate::source::{SourceEvent, SourceStream};
use crate::error::{Error, Result};
use crate::storage::StorageInterface;
use crate::types::Block;

pub struct Aggregator<Source>
where
    Source: SourceStream,
{
    source: Source,
    storage: StorageInterface,
    token: CancellationToken,
}

impl <Source> Aggregator<Source>
where
    Source: SourceStream,
{
    pub fn new(source: Source, storage: StorageInterface, token: CancellationToken) -> Self {
        Self {source, storage, token}
    }

    pub async fn run(&mut self) {
        loop {
            if self.token.is_cancelled() {
                log::debug!("run() interrupted");
                return
            }
            let event = self.source.next().await;
            match event {
                SourceEvent::Next(block) => {
                    match self.process_block(block).await {
                        Ok(_) => {continue}
                        Err(error) => {
                            log::error!("Failed to process block: {}", error);
                            return
                        }
                    }
                }
                SourceEvent::Failure(error) => {
                    let mut level = log::Level::Error;
                    if let Error::Shutdown = &error {
                        level = log::Level::Debug;
                    }
                    log::log!(level, "Stream broken: {}", error);
                    return
                }
                SourceEvent::EndOfStream => {
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