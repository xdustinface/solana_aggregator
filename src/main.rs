mod error;
mod api;
mod aggregator;
mod blocks;
mod storage;
mod types;

use crate::aggregator::Aggregator;
use crate::api::run_api;
use crate::blocks::live::LiveStream;
use crate::storage::memory::Memory;
use crate::storage::{Storage, StorageInterface};
use log::LevelFilter;
use simple_logger::SimpleLogger;
use std::net::SocketAddr;
use std::process::exit;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use clap::Parser;
use crate::blocks::benchmark::Benchmark;
use tokio::signal;

#[derive(Parser)]
#[command(version, about, long_about = "Solana data aggregator")]
struct Args {
    #[arg(short, long, default_value = "127.0.0.1:8080")]
    api_socket: SocketAddr,
    #[arg(short='p', long, default_value = "https://api.devnet.solana.com")]
    source_path: String,
    #[arg(short, long, default_value_t = false)]
    benchmark: bool,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    SimpleLogger::new()
        .with_level(LevelFilter::Error)  // Set the global logging level to Error
        .with_module_level("solana_aggregator", LevelFilter::Debug)  // Set your crate's logging level to Debug
        .init()
        .expect("simple_logger init failed");
    let token = CancellationToken::new();
    let storage_token = token.clone();
    log::debug!("Create data storage");
    let (storage_tx, storage_rx) = mpsc::channel(20);
    let storage_interface = StorageInterface::new(storage_tx);
    let mut storage = Memory::default();
    // Spawn the API server in an async task
    let storage_task = tokio::spawn(async move {
        storage.run(storage_rx, storage_token).await
    });
    log::debug!("Create source stream + aggregator and start it!");
    let aggregator_task;
    if args.benchmark {
        let stream = Benchmark::new(args.source_path);
        let mut aggregator = Aggregator::new(
            stream,
            storage_interface.clone(),
            token.clone()
        );
        aggregator_task = tokio::spawn(async move {
            aggregator.run().await
        });
    } else {
        let stream = match LiveStream::create_with_latest_slot(
            args.source_path,
            token.clone()
        ).await {
            Ok(stream) => {stream}
            Err(error) => {
                log::error!("Failed to create stream {}", error);
                exit(1);
            }
        };
        let mut aggregator = Aggregator::new(
            stream,
            storage_interface.clone(),
            token.clone()
        );
        aggregator_task = tokio::spawn(async move {
            aggregator.run().await
        });
    }
    log::debug!("Create and start API");
    // Spawn the API server in an async task
    let api_task = tokio::spawn(
        run_api(
            args.api_socket,
            storage_interface.clone(),
            token.clone()
        )
    );

    let shutdown_handle = tokio::spawn(async move {
        // Wait for a shutdown signal (SIGINT or SIGTERM).
        signal::ctrl_c().await.unwrap();
        log::debug!("Shutdown signal received!");
        // Let the rest of the application know about the shutdown.
        token.cancel();
    });

    tokio::join!(
        storage_task,
        aggregator_task,
        api_task,
        shutdown_handle
    );

    log::debug!("Done!");
}
