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

#[cfg(test)]
mod tests {
    use crate::types::Transaction;
    use super::*;

    async fn assert_accounts(memory: &Memory, expected_accounts: &Vec<Account>) {
        let accounts = memory.get_accounts().await.unwrap();
        assert_eq!(accounts.len(), expected_accounts.len());
        for account in expected_accounts.clone() {
            assert!(accounts.contains(&account));
        }
    }
    async fn assert_transactions(
        memory: &Memory,
        address: &Address,
        expected_transactions: Vec<TransactionWithMeta>
    ) {
        let transactions = memory.get_transactions(&address).await.unwrap();
        assert_eq!(transactions.len(), expected_transactions.len());
        for transaction in expected_transactions.clone() {
            assert!(transactions.contains(&transaction));
        }
    }

    fn get_block(height: u64, transactions: Vec<Transaction>) -> Block {
        Block {
            height,
            hash: height.to_string(),
            timestamp: height as i64,
            transactions,
        }
    }

    #[tokio::test]
    async fn test_add_and_fetch_data() {
        let mut memory = Memory::default();

        assert_eq!(memory.get_accounts().await.unwrap().len(), 0);

        let mut account_0 = Account {
            address: "0".to_string(),
            balance: 0,
        };
        let mut account_1 = Account {
            address: "1".to_string(),
            balance: 0,
        };

        let tx_0 = TransactionWithMeta {
            data: Transaction {
                sender: account_0.address.clone(),
                receiver: account_1.address.clone(),
                amount: 1,
            },
            timestamp: 0,
        };
        let tx_1 = TransactionWithMeta {
            data: Transaction {
                sender: account_1.address.clone(),
                receiver: account_0.address.clone(),
                amount: 2,
            },
            timestamp: 1,
        };
        let tx_2 = TransactionWithMeta {
            data: Transaction {
                sender: account_1.address.clone(),
                receiver: account_1.address.clone(),
                amount: 5,
            },
            timestamp: 2,
        };
        let tx_3 = TransactionWithMeta {
            data: Transaction {
                sender: account_1.address.clone(),
                receiver: account_0.address.clone(),
                amount: 10,
            },
            timestamp: 2,
        };

        let block_0 = get_block(0, Vec::from([tx_0.data.clone()]));
        let block_1 = get_block(1, Vec::from([tx_1.data.clone()]));
        let block_2 = get_block(2, Vec::from([tx_2.data.clone(), tx_3.data.clone()]));

        let mut expected_accounts = Vec::from([account_0, account_1]);

        assert!(memory.add_block(block_0).await.is_ok());
        expected_accounts[0].balance = -1;
        expected_accounts[1].balance = 1;
        assert_accounts(&memory, &expected_accounts).await;
        assert_transactions(&memory, &expected_accounts[0].address, Vec::from([tx_0.clone()])).await;
        assert_transactions(&memory, &expected_accounts[1].address, Vec::from([tx_0.clone()])).await;

        assert!(memory.add_block(block_1).await.is_ok());
        expected_accounts[0].balance = 1;
        expected_accounts[1].balance = -1;
        assert_accounts(&memory, &expected_accounts).await;
        assert_transactions(&memory, &expected_accounts[0].address, Vec::from([tx_0.clone(), tx_1.clone()])).await;
        assert_transactions(&memory, &expected_accounts[1].address, Vec::from([tx_0.clone(), tx_1.clone()])).await;

        assert!(memory.add_block(block_2).await.is_ok());
        expected_accounts[0].balance = 11;
        expected_accounts[1].balance = -11;
        assert_accounts(&memory, &expected_accounts).await;
        assert_transactions(
            &memory,
            &expected_accounts[0].address,
            Vec::from([tx_0.clone(),tx_1.clone(), tx_3.clone()])
        ).await;
        assert_transactions(
            &memory,
            &expected_accounts[1].address,
            Vec::from([tx_0.clone(), tx_1.clone(), tx_2.clone(), tx_3.clone()])
        ).await;
    }
    async fn test_add_block_failures() {
        let mut memory = Memory::default();

        let block_0 = get_block(0, Vec::new());
        let block_1 = get_block(1, Vec::new());
        let block_2 = get_block(2, Vec::new());

        assert!(memory.add_block(block_1.clone()).await.is_ok());
        // Adding the block again should lead to failure
        let expected_error = Error::InvalidBlock(1, "Already exists in storage".to_string());
        match memory.add_block(block_1).await {
            Err(Error::InvalidBlock(height, error)) => {
                assert_eq!(height, 1);
                assert_eq!(error, "Already exists in storage");
            }
            _ => {panic!("existing block test failed")}
        }
        match memory.add_block(block_0).await {
            Err(Error::InvalidBlock(height, error)) => {
                assert_eq!(height, 0);
                assert_eq!(error, "Block height must be ascending. last_block: 1");
            }
            _ => {panic!("lower block height test failed")}
        }
    }
}