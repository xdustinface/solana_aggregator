use rusqlite::{Connection, Params, params, Statement};
use crate::error::{Error, Result};
use crate::storage::Storage;
use crate::types::{Account, Address, Block, TransactionWithMeta};

pub struct Sqlite {
    connection: Connection,
}

impl <'a> Sqlite {
    fn new(connection: Connection) -> Self {
        Self {connection}
    }
    fn execute<P: Params>(&self, sql: &str, params: P) -> Result<usize> {
        self.connection.execute(sql, params).map_err(|error|
            Error::StorageFailure(error.to_string())
        )
    }
    fn prepare(&self, sql: &str) -> Result<Statement<'a>> {
        self.connection.prepare(sql).map_err(|error|
            Error::StorageFailure(error.to_string())
        )
    }
    fn initialize(&self) -> Result<()> {
        self.execute(
            "CREATE TABLE account (
                  address    BLOB PRIMARY KEY,
                  balance  INTEGER NOT NULL,
                  )",
            [],
        )?;
        self.execute(
            "CREATE TABLE block (
                  height    INTEGER PRIMARY KEY,
                  hash  BLOB NOT NULL,
                  timestamp INTEGER NOT NULL,
                  )",
            [],
        )?;
        self.execute(
            "CREATE TABLE transaction (
                  hash    BLOB PRIMARY KEY,
                  sender  BLOB NOT NULL,
                  receiver BLOB NOT NULL,
                  amount INTEGER NOT NULL,
                  )",
            [],
        )?;
        Ok(())
    }
}

impl Storage for Sqlite {
    async fn add_block(&mut self, block: Block) -> Result<()> {
        self.execute(
            "INSERT INTO block (height, hash, timestamp) VALUES (?1, ?2, ?3)",
            params![block.height, block.hash.as_bytes().to_vec(), block.timestamp],
        )?;
        Ok(())
    }

    async fn get_accounts(&self) -> Result<Vec<Account>> {
        let mut query = self.prepare("SELECT address, balance FROM account")?;
        let account_iter = query.query_map([], |row| {
            Ok(Account {
                address: row.get(0)?,
                balance: row.get(1)?,
            })
        }).map_err(|error| Error::StorageFailure(error.to_string()))?;
        let mut accounts = Vec::new();
        for account in account_iter {
            accounts.push(account.map_err(|error| Error::StorageFailure(error.to_string()))?);
        }
        Ok(accounts)
    }

    async fn get_transactions(&self, address: &Address) -> Result<Vec<TransactionWithMeta>> {
        let mut transactions = Vec::new();
        Ok(transactions)
    }
}