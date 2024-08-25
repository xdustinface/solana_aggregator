use crate::blocks::{BlockEvent, BlockStream};
use crate::error::{Error, Result};
use crate::types::Block;
use solana_client::client_error::ClientErrorKind;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_config::RpcBlockConfig;
use solana_client::rpc_request::RpcError;
use solana_sdk::clock::Slot;
use solana_transaction_status::{UiConfirmedBlock, UiTransactionEncoding};
use std::time::Duration;
use tokio::time::sleep;

const BLOCK_NOT_AVAILABLE: i64 = -32004;
const SLOT_SKIPPED: i64 = -32007;

pub fn block_config() -> RpcBlockConfig {
    RpcBlockConfig {
        max_supported_transaction_version: Some(0),
        encoding: Some(UiTransactionEncoding::JsonParsed),
        ..RpcBlockConfig::default()
    }
}

pub async fn block_for_slot(slot: Slot, rpc_client: &RpcClient, block_config: RpcBlockConfig) -> Result<UiConfirmedBlock> {
    rpc_client.get_block_with_config(slot, block_config).await
        .map_err(|error| {
            if let ClientErrorKind::RpcError(rpc_error) = error.kind() {
                if let RpcError::RpcResponseError { code, .. } = rpc_error {
                    if code == &BLOCK_NOT_AVAILABLE {
                        return Error::SlotNotAvailable(slot)
                    }
                    if code == &SLOT_SKIPPED {
                        return Error::SlotSkippedOrMissing(slot)
                    }
                }
            }
            Error::RpcError(error)
        })
}

pub struct LiveStream {
    rpc_client: RpcClient,
    current_slot: Slot,
    block_config: RpcBlockConfig,
}

impl LiveStream {
    pub async fn create(url: String) -> Result<Self> {
        let rpc_client = RpcClient::new(url);
        let current_slot = rpc_client.get_slot().await.map_err(|error| {
            Error::RpcError(error)
        })?;
        Ok(Self{rpc_client, current_slot, block_config: block_config()})
    }
}

impl BlockStream for LiveStream {
    async fn next(&mut self) -> BlockEvent {
        loop {
            match block_for_slot(self.current_slot, &self.rpc_client, self.block_config).await {
                Ok(block) => {
                    let block = Block::from(block);
                    self.current_slot += 1;
                    log::debug!(
                        "Block: {} Transactions: {}",
                        block.height,
                        block.transactions.len()
                    );
                    return BlockEvent::Next(block)
                }
                Err(error) => {
                    match error {
                        Error::SlotNotAvailable(_) => {
                            log::debug!("Sleep: {}", error.to_string());
                            sleep(Duration::from_millis(100)).await;
                            continue
                        }
                        Error::SlotSkippedOrMissing(_) => {
                            self.current_slot += 1;
                            log::warn!("Increment current slot: {}", error.to_string());
                            continue
                        }
                        _ => {
                            return BlockEvent::Failure(error)
                        }
                    }
                }
            }
        }
    }
}