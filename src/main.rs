use crate::sql::Ops;

mod config;
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
    let abi_file = std::fs::File::open("abi/uniswap_v2_factory.json").unwrap();
    let abi_factory = ethabi::Contract::load(abi_file).unwrap();
    let uniswap = uniswap::v2::Factory::new(abi_factory);
    let pool_count = uniswap.pool_count(&geth);
    log::info!("Uniswap v2 contract count {:?}", pool_count);
    let abi_file = std::fs::File::open("abi/uniswap_v2_pair.json").unwrap();
    let abi_pool = ethabi::Contract::load(abi_file).unwrap();
    for pool_idx in 0..10 {
        let address = uniswap.pool_addr(&geth, pool_idx).unwrap();
        let tokens = crate::uniswap::v2::Pool::tokens(&geth, &abi_pool, &address).unwrap();
        let pool = uniswap::v2::Pool {
            index: pool_idx as i32,
            address,
            token0: tokens.0,
            token1: tokens.1,
        };
        let reserves = uniswap::v2::Pool::reserves(&geth, &abi_pool, &address).unwrap();
        log::info!("Uniswap v2 pool info #0 {:?} {:?}", pool, reserves);
        sql.insert(pool.to_sql());
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
