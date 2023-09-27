use crate::coin::Coin;
use crate::erc20::Erc20;
use crate::sql::Ops;
use ethereum_types::Address;

mod coin;
mod config;
mod curve;
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

    let eth_block = geth.last_block();
    log::info!("eth block {}", eth_block);
    if std::env::args().find(|arg| arg == "discover").is_some() {
        discover(&geth, &mut sql, eth_block);
    } else if std::env::args().find(|arg| arg == "refresh").is_some() {
        refresh(&geth, &mut sql, eth_block);
    } else {
        log::info!("commands: discover, refresh")
    }
}

fn refresh(geth: &geth::Client, sql: &mut sql::Client, eth_block: u32) {
    let sql_pool_count = uniswap::v2::Factory::sql_pool_count(sql);
    for pool_idx in 0..sql_pool_count {
        log::info!("{:?}", uniswap::v2::Pool::find_sql(pool_idx as i32));
        let rows = sql.q(uniswap::v2::Pool::find_sql(pool_idx as i32));
        let pool = uniswap::v2::Pool::from(&rows[0]);
        log::info!("refresh: {:?}", pool);
        update_pool_reserves(geth, sql, &pool, eth_block);
    }
}

fn discover(geth: &geth::Client, sql: &mut sql::Client, eth_block: u32) {
    uniswap::v2::Factory::setup();
    let pool_count = uniswap::v2::Factory::pool_count(&geth);
    let sql_pool_count = uniswap::v2::Factory::sql_pool_count(sql);
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
        refresh_token(&geth, sql, tokens.0);
        refresh_token(&geth, sql, tokens.1);

        let reserves = uniswap::v2::Pool::reserves(&geth, &abi_pool, &address, eth_block).unwrap();
        let pool_reserves = uniswap::v2::Reserves::new(&pool, eth_block, reserves);
        log::info!("Uniswap v2 pool info #0 {:?} {:?}", pool, reserves);
        sql.insert(pool.to_upsert_sql());
        sql.insert(pool_reserves.to_upsert_sql());
    }
}

fn update_pool_reserves(
    geth: &geth::Client,
    sql: &mut crate::sql::Client,
    pool: &uniswap::v2::Pool,
    eth_block: u32,
) {
    let abi_file = std::fs::File::open("abi/uniswap_v2_pair.json").unwrap();
    let abi_pool = ethabi::Contract::load(abi_file).unwrap();
    let reserves =
        uniswap::v2::Pool::reserves(&geth, &abi_pool, &pool.contract_address, eth_block).unwrap();
    let pool_reserves = uniswap::v2::Reserves::new(&pool, eth_block, reserves);
    sql.insert(pool_reserves.to_upsert_sql());
}

fn refresh_token(geth: &crate::geth::Client, sql: &mut crate::sql::Client, token: Address) -> Coin {
    let exist = sql_query_builder::Select::new()
        .select("*")
        .from("coins")
        .where_clause("contract_address = $1");
    let rows = sql.q((exist.to_string(), vec![Box::new(format!("{:x}", token))]));
    let coin = if rows.len() == 0 {
        let token = Erc20 { address: token };
        let token_name = token.name(&geth).unwrap();
        let token_symbol = token.symbol(&geth).unwrap();
        let token_decimals = token.decimals(&geth).unwrap();
        let coin = Coin {
            contract_address: token.address,
            name: token_name,
            symbol: token_symbol,
            decimals: token_decimals,
        };
        sql.insert(coin.to_upsert_sql());
        coin
    } else {
        log::info!("hydrated from {} rows", rows.len());
        Coin::from(&rows[0])
    };
    log::info!("coin {:?}", coin);
    coin
}
