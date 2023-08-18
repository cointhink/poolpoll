use crate::coin::Coin;
use crate::erc20::Erc20;
use crate::sql::Ops;

mod coin;
mod config;
mod erc20;
mod geth;
mod log;
mod sql;
mod uniswap;

fn main() {
    log::init();
    config::CONFIG.set(config::load("config.yaml")).unwrap();
    log::info!("poolpoll");

    let config = config::CONFIG.get().unwrap();
    let mut sql = sql::new();

    let geth = geth::Client::build(&config.geth_url, &config.infura_key);
    let abi_file = std::fs::File::open("abi/ERC20.json").unwrap();
    erc20::ABI
        .set(ethabi::Contract::load(abi_file).unwrap())
        .unwrap();

    let abi_file = std::fs::File::open("abi/uniswap_v2_factory.json").unwrap();
    let abi_factory = ethabi::Contract::load(abi_file).unwrap();
    uniswap::v2::ABI.set(abi_factory).unwrap();
    let pool_count = uniswap::v2::Factory::pool_count(&geth);
    let max_pool = sql_query_builder::Select::new()
        .select("max(uniswap_v2_index)")
        .from("pools");
    let sql_pool_count_rows = sql.q((max_pool.to_string(), vec![]));
    let sql_pool_count = sql_pool_count_rows[0].get::<&str, i32>("max") as u64;
    log::info!(
        "Uniswap v2 contract count {:?} (db highest {:?})",
        pool_count,
        sql_pool_count
    );
    let abi_file = std::fs::File::open("abi/uniswap_v2_pair.json").unwrap();
    let abi_pool = ethabi::Contract::load(abi_file).unwrap();
    for pool_idx in sql_pool_count..sql_pool_count + 10 {
        let address = uniswap::v2::Factory::pool_addr(&geth, pool_idx).unwrap();
        let tokens = crate::uniswap::v2::Pool::tokens(&geth, &abi_pool, &address).unwrap();
        let pool = uniswap::v2::Pool {
            uniswap_v2_index: pool_idx as i32,
            contract_address: address,
            token0: tokens.0,
            token1: tokens.1,
        };
        let token0 = Erc20 { address: tokens.0 };
        let token0_name = token0.name(&geth).unwrap();
        let token0_symbol = token0.symbol(&geth).unwrap();
        let token0_decimals = token0.decimals(&geth).unwrap();
        let coin0 = Coin {
            contract_address: token0.address,
            name: token0_name,
            symbol: token0_symbol,
            decimals: token0_decimals,
        };
        log::info!("coin0{:?}", coin0);
        sql.insert(coin0.to_upsert_sql());

        let token1 = Erc20 { address: tokens.1 };
        let token1_name = token1.name(&geth).unwrap();
        let token1_symbol = token1.symbol(&geth).unwrap();
        let token1_decimals = token1.decimals(&geth).unwrap();
        let coin1 = Coin {
            contract_address: token1.address,
            name: token1_name,
            symbol: token1_symbol,
            decimals: token1_decimals,
        };
        sql.insert(coin1.to_upsert_sql());

        let reserves = uniswap::v2::Pool::reserves(&geth, &abi_pool, &address).unwrap();
        let pool_reserves = uniswap::v2::Reserves {
            pool: &pool,
            block_number: 0,
            x: reserves.0,
            y: reserves.1,
        };
        log::info!("Uniswap v2 pool info #0 {:?} {:?}", pool, reserves);
        sql.insert(pool.to_upsert_sql());
        sql.insert(pool_reserves.to_upsert_sql());
    }
}

// use ethereum_tx_sign::Transaction;
// fn sign() {
//     let tx = ethereum_tx_sign::LegacyTransaction {
//         chain: 1,
//         nonce: 0,
//         to: None,
//         value: 0,
//         gas_price: 0,
//         gas: 0,
//         data: vec![],
//     };

//     let private_key = hex::decode(&config::CONFIG.get().unwrap().private_key).unwrap();
//     let ecdsa = tx.ecdsa(&private_key).unwrap();
//     let _ = tx.sign(&ecdsa);
//     log::info!("signed.")
// }
