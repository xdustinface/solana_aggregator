use std::time::Duration;
use futures::future::err;
use solana_client::client_error::ClientErrorKind;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_config::RpcBlockConfig;
use solana_client::rpc_request::RpcError;
use solana_sdk::clock::Slot;
use solana_transaction_status::{UiConfirmedBlock, UiMessage, UiParsedInstruction, UiTransactionEncoding};
use tokio::time::sleep;
use crate::blocks::{BlockEvent, BlockStream};
use crate::error::{Error, Result};
use crate::types::Block;

const BLOCK_NOT_AVAILABLE: i64 = -32004;
const SLOT_SKIPPED: i64 = -32007;

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
        let block_config = RpcBlockConfig {
            max_supported_transaction_version: Some(0),
            encoding: Some(UiTransactionEncoding::JsonParsed),
            ..RpcBlockConfig::default()
        };
        Ok(Self{rpc_client, current_slot, block_config})
    }

    async fn block_for_current_slot(&self) -> Result<UiConfirmedBlock> {
        self.rpc_client.get_block_with_config(self.current_slot, self.block_config)
            .await
            .map_err(|error| {
                if let ClientErrorKind::RpcError(rpc_error) = error.kind() {
                    if let RpcError::RpcResponseError { code, .. } = rpc_error {
                        if code == &BLOCK_NOT_AVAILABLE {
                            return Error::SlotNotAvailable(self.current_slot)
                        }
                        if code == &SLOT_SKIPPED {
                            return Error::SlotSkippedOrMissing(self.current_slot)
                        }
                    }
                }
                Error::RpcError(error)
            })
    }
}

impl BlockStream for LiveStream {
    async fn next(&mut self) -> BlockEvent {
        loop {
            match self.block_for_current_slot().await {
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