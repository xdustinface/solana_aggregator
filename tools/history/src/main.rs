extern crate solana_aggregator;

use std::fs;
use std::path::PathBuf;
use std::process::exit;
use std::io::Write;
use clap::{arg, Parser};
use tokio;
use std::io;
use std::string::ToString;
use std::time::Duration;
use solana_client::nonblocking::rpc_client::RpcClient;
use tokio::time::sleep;
use solana_aggregator::blocks::live::{block_config, block_for_slot};
use solana_aggregator::error::Error;

#[derive(Parser, Debug)]
#[command(version, about, long_about = "Solana historical block downloader")]
struct Args {
    /// The path where the application will store the generated JSON file.
    out: PathBuf,
    #[arg(short, long, default_value = None)]
    /// The slot from where to start downloading blocks. It starts 10 blocks behind the latest block
    /// if this option is not provided.
    start: Option<u64>,
    #[arg(short, long, default_value_t = 10)]
    /// The number of blocks to download.
    limit: u64,
    #[arg(short, long, default_value = "https://api.devnet.solana.com")]
    /// The url from where the RPC client will download the block data.
    rpc_url: String,
}


#[tokio::main]
async fn main() {
    let args = Args::parse();

    if !args.out.exists() {
        eprintln!("ERROR: out path doesn't exist {:?}", args.out);
        exit(1);
    }

    let rpc_client = RpcClient::new(args.rpc_url);

    let mut blocks = Vec::new();
    let config = block_config();
    let start_slot = if let Some(start_slot) = args.start {
        start_slot
    } else {
        match rpc_client.get_slot().await {
            Ok(slot) => { slot as u64 - args.limit }
            Err(error) => {
                eprintln!("ERROR: Failed to fetch start slot {:?}", error.to_string());
                exit(1);
            }
        }
    };
    let mut current_slot = start_slot;
    loop {
        match block_for_slot(current_slot, &rpc_client, config).await {
            Ok(block) => {
                blocks.push(block);
                current_slot += 1;
                if current_slot > start_slot + args.limit {
                    break
                }
                println!(
                    "Block received: {} - Blocks total: {}",
                    blocks.last().unwrap().block_height.unwrap(),
                    blocks.len()
                );
            }
            Err(error) => {
                match error {
                    Error::SlotNotAvailable(_) => {
                        println!("Sleep: {}", error.to_string());
                        sleep(Duration::from_millis(100)).await;
                        continue
                    }
                    Error::SlotSkippedOrMissing(_) => {
                        current_slot += 1;
                        eprintln!("Increment current slot: {}", error.to_string());
                        continue
                    }
                    _ => {
                        eprintln!("Aborted due to failure: {}", error.to_string());
                        return
                    }
                }
            }
        }
    }

    let out_path = args.out.join(format!("start_{}_limit_{}.json", start_slot, args.limit));
    let file = fs::File::create(&out_path).expect(format!("Failed to create file: {:?}", out_path).as_str());
    let mut writer = io::BufWriter::new(file);
    serde_json::to_writer(&mut writer, &blocks).unwrap();
    writer.flush().unwrap();
    println!("Saved to {}", out_path.to_str().unwrap());
}