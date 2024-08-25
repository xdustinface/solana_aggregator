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
use clap::Parser;
use crate::blocks::benchmark::Benchmark;

#[derive(Parser)]
#[command(version, about, long_about = "Solana data aggregator")]
struct Args {
    /// Specify the socket address and port where the application should listen to for API requests.
    #[arg(short, long, default_value = "127.0.0.1:8080")]
    api_socket: SocketAddr,
    /// The output file to write results to
    /// Specify the url the RPC client or the local file path from which the application should
    /// load the RPC data from if the benchmark mode is enabled.
    #[arg(short='p', long, default_value = "https://api.devnet.solana.com")]
    source_path: String,
    /// Enabled the benchmark mode which allows to provide locally stored RPC JSON file via the
    /// --source-path argument.
    #[arg(short, long, default_value_t = false)]
    benchmark: bool,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    SimpleLogger::new()
        .with_level(LevelFilter::Error)
        .with_module_level("solana_aggregator", LevelFilter::Debug)
        .init()
        .expect("simple_logger init failed");
    log::debug!("Create data storage");
    let (storage_tx, storage_rx) = mpsc::channel(20);
    let storage_interface = StorageInterface::new(storage_tx);
    let storage_task = tokio::spawn(async move {
        Memory::default().run(storage_rx).await
    });
    log::debug!("Create source stream + aggregator and start it!");
    let aggregator_task;
    if args.benchmark {
        let stream = Benchmark::new(args.source_path);
        let mut aggregator = Aggregator::new(
            stream,
            storage_interface.clone(),
        );
        aggregator_task = tokio::spawn(async move {
            aggregator.run().await
        });
    } else {
        let stream = match LiveStream::create_with_latest_slot(
            args.source_path,
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
        );
        aggregator_task = tokio::spawn(async move {
            aggregator.run().await
        });
    }
    log::debug!("Create and start API");
    let api_task = tokio::spawn(run_api(args.api_socket, storage_interface.clone()));

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
