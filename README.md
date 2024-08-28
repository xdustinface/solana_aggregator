## Soalana aggregator

This is a simple command-line application written in Rust to fetch and process Solana blockchain data via the official 
RPC interface to store it either in a temporary or permanent storage from where the data gets served via the
application's own REST API server.

It's minimal and was developed as part of an interview process (based the task formulation in [TASK.md](https://github.com/xdustinface/solana_aggregator/tree/main/TASK.md)) to demonstrate my
programing skills and style.

## Considerations

While the application runs stable and reliable there are a lot of simplifications and assumptions I made here to reduce
the overall complexity. I spent over 3 days now on this project which I consider enough for a task which is part of an
interview process. It should give a good enough idea about my coding style and skills.

Few things I want to mention here about the application:

- I never looked into Solana before so my knowledge about its protocol was limited when i started this.
- The transaction parser only considers standard SOL instruction as transaction. It also splits up a transaction into
  multiple transactions based on the instructions provided in the transaction without having a identifier for the
  transactions at all so they are just indexed via the block height.
- The `Account` data processed and stored are completely off since it doesn't consider the initial state
when the first transaction gets applied. To fix this there would need to be some initialization phase when a new account
gets added to fetch the balance at the starting height.
- The processing of the blocks might still have some edge cases where slots wouldn't contain a block for some reason. I
  found the slots sometimes being skipped (RPC error -32007) or there is no block for a slot (RPC error -32004) and since
  im not yet familiar with the Solana protocol i can't tell what else might come up here.
- I didn't spend much time now at the end to write tests, which is something I would spend a lot of time for in a
more production like environment. But I still want to express my importance of it here hence I added few minimal test
examples in [src/storage/memory.rs](https://github.com/xdustinface/solana_aggregator/tree/main/src/storage/memory.rs) and [src/source/benchmark.rs](https://github.com/xdustinface/solana_aggregator/tree/main/src/source/benchmark.rs).

## Design

It's an asynchronous application based on the [tokio runtime](https://github.com/tokio-rs/tokio),
and it's separated into four long-running async tasks:

#### 1. Aggregator

The [aggregator](https://github.com/xdustinface/solana_aggregator/blob/main/src/aggregator.rs) collects the data from
its given [source](https://github.com/xdustinface/solana_aggregator/tree/main/src/source) and relays it to the
[storage](https://github.com/xdustinface/solana_aggregator/tree/main/src/storage). The source can be anything that implements the `SourceStream` trait:

```rust
pub trait SourceStream {
    fn next(&mut self) -> impl std::future::Future<Output = SourceEvent> + Send;
}
```
There are two implementations included here:
- `LiveStream` which is an actual live stream of the ongoing Solana blocks based on polling the RPC interface which
would be better done via the Websocket [blockSubscribe](https://solana.com/docs/rpc/websocket/blockSubscribe) channel but that's unstable and apparently not available in
standard RPC interfaces unless the validator was started with `--rpc-pubsub-enable-block-subscription` which it doesn't
seem to be in the public interfaces I tried. Another better option would also be to use the [slotSubscribe](https://solana.com/docs/rpc/websocket/slotSubscribe) channel
and fetch block via the RPC interface when a slot event was received. But for this example I decided to stick with the
polling. The default interface [https://api.devnet.solana.com](https://api.devnet.solana.com) used in the application has a rate-limit, that's why
the
block processing slows down for a few seconds while running on it.
- `Benchmark` which reads a provided JSON file containing stored RPC data to run the blockchain data without side
effects of networking as a way to profile the block processing. See `-b/--benchmark` command line argument.

#### 2. Storage

Implemented via the `Storage` trait 

```rust
pub trait Storage {
    async fn run(&mut self, mut receiver: mpsc::Receiver<StorageCommand>, token: CancellationToken) -> Result<()> {...}
    async fn process_command(&mut self, command: StorageCommand) -> Result<()> {...}
    async fn add_block(&mut self, block: Block) -> Result<()>;
    async fn get_accounts(&self) -> Result<Vec<Account>>;
    async fn get_transactions(&self, address: &Address) -> Result<Vec<TransactionWithMeta>>;
}
```
there is a long-running tasks which listens on a MPSC channel for a
`StorageCommand` which is defined as:


```rust
pub enum StorageCommand {
    AddBlock(Block, oneshot::Sender<AddBlockResult>),
    GetAccounts(oneshot::Sender<GetAccountsResult>),
    GetTransactions(Address, oneshot::Sender<GetTransactionsResult>),
}
```

The command `AddBlock` is used to add a new block to the underlying storage while the other commands are used to fetch data
from the storage. For every received commands it uses the oneshot channel embedded in the command data to send the
responses back to the sender. There is an example implementation for the storage with some `HashMap`s in
[src/storage/memory.rs](https://github.com/xdustinface/solana_aggregator/tree/main/src/storage/memory.rs) and an
[WIP branch "sqlite"](https://github.com/xdustinface/solana_aggregator/tree/sqlite) for a SQLite implementation.

### 3. API

API server with two simple endpoints without pagination or further scaling considerations. The server listens on the
IP/Port provided via the `-a/--api-socket` command line argument which is `127.0.0.1:8080` by default. It provides the following endpoints:

#### GET /accounts

Serves a list of objects containing all addresses with their balances of all available addresses
in the applications storage.

**Example output**
```bash
curl 127.0.0.1:8080/accounts
[
  {
    "address":"BhN2e75JhW3mJH4S88kkL4xfjf6j6M2sNhyT6yXBXvr8",
    "balance":10000000000
  },
  {
    "address":""4a7s9iC5NwfUtf8fXpKWxYXcekfqiN6mRqipYXMtcrUS"",
    "balance":20000000000
  },
  ...
]
```

#### GET /transactions?address=:address
Serves all the transactions which involve the provided `:address` as sender or receiver.

**Example output**
```bash
curl 127.0.0.1:8080/transactions?address=2ZHGpnNF4ddcCVUf9TFdbRntxx
[
  {
    "data": {
      "sender":"2ZHGpnNF4ddcCVUve2PfiUKjQduYZV41df9TFdbRntxx",
      "receiver":"4a7s9iC5NwfUtf8fXpKWxYXcekfqiN6mRqipYXMtcrUS",
      "amount":50000000
    },
    "timestamp":1716188782
  },
  {
    "data": {
      "sender":"CoEkevzqF3mqzXoKWisp9kuoLN4MfjToP4qsgF4sbKjX",
      "receiver":"2ZHGpnNF4ddcCVUve2PfiUKjQduYZV41df9TFdbRntxx",
      "amount":90000000
    },
    "timestamp":1716188789
  },
  ...
]
```
#### 4. Shutdown

Waits for OS shutdown signals and informs all other long-running tasks when a shutdown was requested to gracefully
stop everything.

## Installation

### Prerequisites

- Rust and Cargo installed. If you don't have them installed, you can get them [here](https://www.rust-lang.org/tools/install).

### Install and run

```bash
git clone https://github.com/xdustinface/solana_aggregator.git
cd solana_aggregator
cargo build --release
./target/release/solana_aggregator
```

## Command line interface 

```
Minimal and simplified data aggregator for Solana blockchain data

Usage: solana_aggregator [OPTIONS]

Options:
  -a, --api-socket <API_SOCKET>  The socket address and port where the application should listen to for API requests [default: 127.0.0.1:8080]
  -r, --rpc-url <RPC_URL>        The url from where the RPC client will download the block data
  -f, --file-path <FILE_PATH>    The path to a local JSON file containing a list of block objects returned by the get_block RPC interface call of the official Solana RPC interface
  -h, --help                     Print help (see more with '--help')
  -V, --version                  Print version
```

## History downloader

There is also a tool included to download historic blocks based on a start slot number. They are getting saved to a JSON
file after downloading. See [tools/history](https://github.com/xdustinface/solana_aggregator/tree/main/tools/history).

**Command line interface**
```
Download historical Solana blocks and save them to a JSON file.

Usage: history [OPTIONS] <OUT>

Arguments:
  <OUT>  The path where the application will store the generated JSON file

Options:
  -s, --start <START>      The slot from where to start downloading blocks. It starts 10 blocks behind the latest block if this option is not provided
  -l, --limit <LIMIT>      The number of blocks to download [default: 10]
  -r, --rpc-url <RPC_URL>  The url from where the RPC client will download the block data [default: https://api.devnet.solana.com]
  -h, --help               Print help (see more with '--help')
  -V, --version            Print version
```

## License
This project is licensed under the MIT License - see the LICENSE file for details.