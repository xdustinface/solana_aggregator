pub mod memory;

use crate::error::{Error, Result};
use crate::types::{Account, Address, Block, TransactionWithMeta};
use tokio::sync::{mpsc, oneshot};


pub type AddBlockResult = Result<()>;
pub type GetAccountsResult = Result<Vec<Account>>;
pub type GetTransactionsResult = Result<Vec<TransactionWithMeta>>;


async fn receive<Type>(sender: &str, receiver: oneshot::Receiver<Type>) -> Result<Type> {
    receiver.await.map_err(|error| {
        Error::ChannelFailure(sender.to_string(), error.to_string())
    })
}

pub enum StorageCommand {
    AddBlock(Block, oneshot::Sender<AddBlockResult>),
    GetAccounts(oneshot::Sender<GetAccountsResult>),
    GetTransactions(Address, oneshot::Sender<GetTransactionsResult>),
}

impl StorageCommand {
    pub async fn send(self, from: &str, sender: mpsc::Sender<StorageCommand>) -> Result<()> {
        sender.send(self).await.map_err(|error| {
            Error::ChannelFailure(from.to_string(), error.to_string())
        })?;
        Ok(())
    }
}

#[derive(Clone)]
pub struct StorageInterface {
    pub command_tx: mpsc::Sender<StorageCommand>,
}

impl StorageInterface {
    pub fn new(command_tx: mpsc::Sender<StorageCommand>) -> Self {
        Self {command_tx}
    }
    pub async fn add_block(&self, block: Block) -> AddBlockResult {
        let (tx, rx) = oneshot::channel();
        let command = StorageCommand::AddBlock(block, tx);
        command.send("add_block", self.command_tx.clone()).await?;
        receive("add_block", rx).await?
    }
    pub async fn get_accounts(&self) -> GetAccountsResult {
        let (tx, rx) = oneshot::channel();
        let command = StorageCommand::GetAccounts(tx);
        command.send("get_accounts", self.command_tx.clone()).await?;
        receive("get_accounts", rx).await?
    }
    pub async fn get_transactions(&self, address: Address) -> GetTransactionsResult {
        let (tx, rx) = oneshot::channel();
        let command = StorageCommand::GetTransactions(address, tx);
        command.send("get_transactions", self.command_tx.clone()).await?;
        receive("get_transactions", rx).await?
    }
}

pub trait Storage {
    async fn run(&mut self, mut receiver: mpsc::Receiver<StorageCommand>) -> Result<()> {
        while let Some(command) = receiver.recv().await {
            match command {
                StorageCommand::AddBlock(block, sender) => {
                    if let Err(_) = sender.send(self.add_block(block).await) {
                        return Err(
                            Error::ChannelFailure(
                                "storage_add_block".to_string(),
                                "send failure".to_string(),
                            )
                        )
                    }
                }
                StorageCommand::GetAccounts(sender) => {
                    if let Err(_) = sender.send(self.get_accounts().await) {
                        return Err(Error::ChannelFailure(
                            "storage_get_accounts".to_string(),
                            "send failure".to_string())
                        )
                    }
                }
                StorageCommand::GetTransactions(address, sender) => {
                    if let Err(_) = sender.send(self.get_transactions(&address).await) {
                        return Err(Error::ChannelFailure(
                            "storage_get_transactions".to_string(),
                            "send failure".to_string())
                        )
                    }
                }
            }
        }
        Ok(())
    }
    async fn add_block(&mut self, block: Block) -> Result<()>;
    async fn get_accounts(&self) -> Result<Vec<Account>>;
    async fn get_transactions(&self, address: &Address) -> Result<Vec<TransactionWithMeta>>;
}