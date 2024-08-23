use solana_client::client_error::ClientError;
use solana_sdk::clock::Slot;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Channel failure for: {0} - Failure: {1}")]
    ChannelFailure(String, String),
    #[error("RPC error: {0}")]
    RpcError(ClientError),
    #[error("Slot {0} not available")]
    SlotNotAvailable(Slot),
    #[error("Slot {0} was skipped or is missing")]
    SlotSkippedOrMissing(Slot),
    #[error("Invalid block with height {0} - Reason: {1}")]
    InvalidBlock(u64, String),
}
