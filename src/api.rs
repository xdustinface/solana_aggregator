use crate::storage::StorageInterface;
use crate::types::Address;
use serde::Deserialize;
use std::net::SocketAddr;
use warp;
use warp::Filter;

#[derive(Deserialize)]
pub struct GetTransactions {
    pub address: Address
}

async fn get_transactions(
    params: GetTransactions,
    storage_interface: StorageInterface,
) -> Result<impl warp::Reply, warp::Rejection> {
    match storage_interface.get_transactions(params.address).await {
        Ok(transactions) => {
            Ok(warp::reply::json(&transactions))
        }
        Err(error) => {
            Ok(warp::reply::json(&error.to_string()))
        }
    }
}

async fn get_accounts(
    storage_interface: StorageInterface,
) -> Result<impl warp::Reply, warp::Rejection> {
    match storage_interface.get_accounts().await {
        Ok(accounts) => {
            Ok(warp::reply::json(&accounts))
        }
        Err(error) => {
            Ok(warp::reply::json(&error.to_string()))
        }
    }
}

pub async fn run_api(address: SocketAddr, storage_interface: StorageInterface) {
    let get_transactions_interface = storage_interface.clone();
    let get_transactions_route = warp::path!("transactions")
        .and(warp::query::<GetTransactions>())
        .and(warp::any().map(move || get_transactions_interface.clone()))
        .and_then(get_transactions);
    let get_accounts_interface = storage_interface.clone();
    let get_accounts_route = warp::path!("accounts")
        .and(warp::any().map(move || get_accounts_interface.clone()))
        .and_then(get_accounts);
    let routes = get_accounts_route.or(get_transactions_route);
    warp::serve(routes).run(address).await;
}