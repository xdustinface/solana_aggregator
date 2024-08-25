use crate::error::Error;
use crate::types::{Block, Transaction};
use solana_transaction_status::{EncodedTransaction, EncodedTransactionWithStatusMeta, UiConfirmedBlock, UiInstruction, UiMessage, UiParsedInstruction};

pub mod benchmark;
pub mod live;

pub enum BlockEvent {
    Next(Block),
    Failure(Error),
    EndOfStream,
}

pub trait BlockStream {
    fn next(&mut self) -> impl std::future::Future<Output = BlockEvent> + Send;
}

fn parse_instruction(instruction: &UiInstruction) -> Option<Transaction> {
    if let UiInstruction::Parsed(parsed) = instruction {
        if let UiParsedInstruction::Parsed(parsed_instruction) = parsed {
            if parsed_instruction.program == "system" {
                if parsed_instruction.parsed.get("type")?.as_str()? == "transfer" {
                    let info = parsed_instruction.parsed.get("info")?.as_object()?;
                    return Some(
                        Transaction {
                            sender: info.get("source")?.as_str()?.to_string(),
                            receiver: info.get("destination")?.as_str()?.to_string(),
                            amount: info.get("lamports")?.as_number()?.as_u64()?,
                        }
                    );
                }
            }
        }
    }
    None
}

fn parse_transaction(transaction: EncodedTransactionWithStatusMeta) -> Option<Vec<Transaction>> {
    let transaction = match transaction.transaction {
        EncodedTransaction::Json(transaction) => {transaction}
        _ => return None
    };

    let message = match &transaction.message {
        UiMessage::Parsed(message) => message,
        _ => return None,
    };

    let mut transactions = Vec::new();
    for instruction in &message.instructions {
        if let Some(transaction) = parse_instruction(instruction) {
            transactions.push(transaction)
        }
    }
    Some(transactions)
}


impl From<UiConfirmedBlock> for Block {
    fn from(block: UiConfirmedBlock) -> Self {
        let mut transactions: Vec<Transaction> = Vec::new();
        if let Some(block_transactions) = block.transactions {
            for transaction in block_transactions {
                if let Some(mut parsed) = parse_transaction(transaction) {
                    transactions.append(&mut parsed);
                }
            }
        };
        Self {
            height: block.block_height.unwrap(),
            hash: block.blockhash,
            transactions,
            timestamp: block.block_time.unwrap(),
        }
    }
}
