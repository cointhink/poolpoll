use crate::erc20::Erc20;
use crate::sql::Ops;

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
    log::info!("Uniswap v2 contract count {:?}", pool_count);
    let abi_file = std::fs::File::open("abi/uniswap_v2_pair.json").unwrap();
    let abi_pool = ethabi::Contract::load(abi_file).unwrap();
    for pool_idx in 0..10 {
        let address = uniswap::v2::Factory::pool_addr(&geth, pool_idx).unwrap();
        let tokens = crate::uniswap::v2::Pool::tokens(&geth, &abi_pool, &address).unwrap();
        let pool = uniswap::v2::Pool {
            uniswap_v2_index: pool_idx as i32,
            contract_address: address,
            token0: Erc20 { address: tokens.0 },
            token1: Erc20 { address: tokens.1 },
        };
        pool.token0.name(&geth).unwrap();
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
