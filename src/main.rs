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

const DEFAULT_RPC_URL: &str = "https://api.devnet.solana.com";

#[derive(Parser)]
#[command(version, about, long_about = "Solana data aggregator")]
struct Args {
    /// The socket address and port where the application should listen to for API requests.
    #[arg(short, long, default_value = "127.0.0.1:8080")]
    api_socket: SocketAddr,
    /// The url from where the RPC client will download the block data.
    #[arg(short, long, default_value = None)]
    rpc_url: Option<String>,
    /// The path to a local JSON file containing a list of block objects returned by the get_block
    /// RPC interface call of the official Solana RPC interface.
    #[arg(short, long, default_value = None)]
    file_path: Option<String>,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    if args.rpc_url.is_some() && args.file_path.is_some() {
        eprintln!("You can only use one of: --rpc-url / --file-path");
        exit(1);
    }
    SimpleLogger::new()
        .with_level(LevelFilter::Error)
        .with_module_level("solana_aggregator", LevelFilter::Debug)
        .init()
        .expect("simple_logger init failed");
    let token = CancellationToken::new();
    let storage_token = token.clone();
    log::debug!("Create data storage");
    let (storage_tx, storage_rx) = mpsc::channel(20);
    let storage_interface = StorageInterface::new(storage_tx);
    let storage_task = tokio::spawn(async move {
        Memory::default().run(storage_rx, storage_token).await
    });
    log::debug!("Create source stream + aggregator and start it!");
    let aggregator_task;
    if let Some(file_path) = args.file_path {
        let stream = Benchmark::new(file_path);
        let mut aggregator = Aggregator::new(
            stream,
            storage_interface.clone(),
            token.clone()
        );
        aggregator_task = tokio::spawn(async move {
            aggregator.run().await
        });
    } else {
        let url = if let Some(rpc_url) = args.rpc_url {
            rpc_url
        } else {
            DEFAULT_RPC_URL.to_string()
        };
        let stream = match LiveStream::create_with_latest_slot(
            url,
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
    let api_task = tokio::spawn(
        run_api(
            args.api_socket,
            storage_interface.clone(),
            token.clone()
        )
    );

    let shutdown_task = tokio::spawn(async move {
        // Wait for a shutdown signal (SIGINT or SIGTERM).
        signal::ctrl_c().await.unwrap();
        log::debug!("Shutdown signal received!");
        // Let the rest of the application know about the shutdown.
        token.cancel();
    });

    // Wait for all tasks to be done!
    tokio::join!(
        storage_task,
        aggregator_task,
        api_task,
        shutdown_task
    );

    log::debug!("Done!");
}
