mod error;
mod api;
mod aggregator;
mod blocks;
mod storage;
mod types;

use std::net::SocketAddr;
use std::process::exit;
use log::LevelFilter;
use simple_logger::SimpleLogger;
use tokio::sync::mpsc;
use crate::aggregator::Aggregator;
use crate::api::run_api;
use crate::blocks::live::LiveStream;
use crate::storage::memory::Memory;
use crate::storage::{Storage, StorageInterface};

#[tokio::main]
async fn main() {
    SimpleLogger::new()
        .with_level(LevelFilter::Error)  // Set the global logging level to Error
        .with_module_level("solana_aggregator", LevelFilter::Debug)  // Set your crate's logging level to Debug
        .init()
        .expect("simple_logger init failed");
    log::debug!("Create source stream");
    let rpc_url = "https://api.devnet.solana.com".to_string();
    let stream = match LiveStream::create(rpc_url).await {
        Ok(stream) => {stream}
        Err(error) => {
            log::error!("Failed to create stream {}", error);
            exit(1);
        }
    };
    log::debug!("Create data storage");
    let (storage_tx, storage_rx) = mpsc::channel(20);
    let storage_interface = StorageInterface::new(storage_tx);
    let mut storage = Memory::default();
    // Spawn the API server in an async task
    let storage_task = tokio::spawn(async move {
        storage.run(storage_rx).await
    });
    log::debug!("Create and start aggregator");
    let mut aggregator = Aggregator::new(stream, storage_interface.clone());
    let aggregator_task = tokio::spawn(async move {
        aggregator.run().await
    });
    log::debug!("Create and start API");
    let socket_address = SocketAddr::new("127.0.0.1".parse().unwrap(), 8080);
    // Spawn the API server in an async task
    let api_task = tokio::spawn(run_api(socket_address, storage_interface.clone()));

    // Await all tasks to run them concurrently
    tokio::select! {
        _ = storage_task => {
            log::debug!("Storage task finished.");
        }
        _ = aggregator_task => {
            log::debug!("Aggregator task finished.");
        }
        _ = api_task => {
            log::debug!("API task finished.");
        }
    }
    log::debug!("Done!");
}
