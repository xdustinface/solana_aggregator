use crate::error::{Error, Result};
use crate::storage::Storage;
use crate::types::{Account, Address, Block, TransactionWithMeta};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone, Debug, PartialEq)]
struct TransactionIndex {
    pub block_height: u64,
    pub index: usize,
}

#[derive(Default, Debug)]
struct Data {
    last_block: u64,
    blocks: HashMap<u64, Block>,
    accounts: HashMap<Address, i64>,
    transaction_index: HashMap<Address, Vec<TransactionIndex>>,
}

#[derive(Default, Clone, Debug)]
pub struct Memory {
    data: Arc<Mutex<Data>>
}

impl Storage for Memory {
    async fn add_block(&mut self, block: Block) -> Result<()> {
        let mut data = self.data.lock().await;
        let block_height = block.height;
        if data.blocks.contains_key(&block_height) {
            return Err(Error::InvalidBlock(block.height, "Already exists in storage".to_string()));
        }
        if block_height < data.last_block {
            return Err(Error::InvalidBlock(
                block.height,
                format!("Block height must be ascending. last_block: {}", data.last_block))
            );
        }
        data.last_block = block_height;
        for (index, transaction) in block.transactions.iter().enumerate() {
            // Update transaction index
            let tx_index = TransactionIndex {
                block_height,
                index,
            };
            let sender_index = data.transaction_index
                .entry(transaction.sender.clone()).or_default();
            if !sender_index.contains(&tx_index) {
                sender_index.push(tx_index.clone());
            }
            let receiver_index = data.transaction_index
                .entry(transaction.receiver.clone()).or_default();
            if !receiver_index.contains(&tx_index) {
                receiver_index.push(tx_index.clone());
            }
            // Update accounts
            let mut receiver_account = data.accounts
                .entry(transaction.receiver.clone()).or_default();
            *receiver_account += transaction.amount as i64;
            let mut sender_account = data.accounts
                .entry(transaction.sender.clone()).or_default();
            *sender_account -= transaction.amount as i64;
        }
        data.blocks.insert(block_height, block);
        Ok(())
    }

    async fn get_accounts(&self) -> Result<Vec<Account>> {
        let data = self.data.lock().await;
        let mut accounts = Vec::with_capacity(data.accounts.len());
        for (address, balance) in data.accounts.iter() {
            accounts.push(
                Account {
                    address: address.clone(),
                    balance: balance.clone(),
                }
            );
        }
        Ok(accounts)
    }

    async fn get_transactions(&self, address: &Address) -> Result<Vec<TransactionWithMeta>> {
        let data = self.data.lock().await;
        let mut transactions = Vec::new();
        if let Some(transaction_index) = data.transaction_index.get(address) {
            transactions.reserve(transaction_index.len());
            for index in transaction_index {
                let block = data.blocks.get(&index.block_height).unwrap();
                let transaction = block.transactions.get(index.index).unwrap();
                transactions.push(
                    TransactionWithMeta {
                        data: transaction.clone(),
                        timestamp: block.timestamp,
                    }
                );
            }
        }
        Ok(transactions)
    }
}