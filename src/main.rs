use crate::coin::Coin;
use crate::erc20::Erc20;
use crate::etherscan::Etherscan;
use crate::sql::Ops;
use ethereum_types::Address;
use std::ops::Not;

mod coin;
mod config;
mod curve;
mod erc20;
mod etherscan;
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

    let last_block_number = 18224212; //geth.last_block_number();
    log::info!("eth last block number {}", last_block_number);
    if std::env::args().find(|arg| arg == "discover").is_some() {
        discover(&geth, &mut sql, last_block_number);
    } else if std::env::args().find(|arg| arg == "refresh").is_some() {
        refresh(&geth, &mut sql, last_block_number);
    } else if std::env::args().find(|arg| arg == "tail").is_some() {
        let mut block_number = last_block_number;
        loop {
            tail(&geth, &mut sql, block_number);
            block_number = block_number + 1;
        }
    } else {
        log::info!("commands: discover, refresh, tail")
    }
}

fn tail(geth: &geth::Client, sql: &mut sql::Client, block_number: u32) {
    let config = config::CONFIG.get().unwrap();
    let etherscan = Etherscan::new(config.etherscan_key.clone());
    let block = geth.block(block_number);
    log::info!(
        "tail block {} {} transactions",
        block_number,
        block.transactions.len()
    );
    let abi_file = std::fs::File::open("abi/uniswap_v2_pair.json").unwrap();
    let abi_pool = ethabi::Contract::load(abi_file).unwrap();
    let transactions_with_to: Vec<_> = block
        .transactions
        .into_iter()
        .filter(|t| t.to.is_some())
        .collect();
    let transactions_to_known_pools = transactions_with_to.into_iter().filter(|t| {
        sql.q(uniswap::v2::Pool::find_by_contract_address(
            t.to.as_ref().unwrap().as_str().into(),
        ))
        .is_empty()
        .not()
    });
    for transaction in transactions_to_known_pools {
        let swap = abi_pool.function("swap").unwrap();
        let swap_sig = hex::encode(swap.short_signature());
        if transaction.input[2..10] == swap_sig {
            let input_params = &transaction.input[10..];
            let input_bytes = hex::decode(input_params).unwrap();
            let input_tokens = swap.decode_input(&input_bytes).unwrap();
            let callback = input_tokens[3].clone().into_bytes().unwrap();
            log::info!(
                "swap pool {} token0 {:?} token1 {:?} to {:?} callback {:?}",
                transaction.to.unwrap(),
                input_tokens[0],
                input_tokens[1],
                input_tokens[2],
                hex::encode(&callback)
            );
            //let callback_tokens = swap.decode_input(&callback).unwrap();
            //log::info!("swap pool callback {:?}", callback_tokens);
            let internal_txs = etherscan.tx_list_internal(transaction.hash).unwrap();
            log::info!("swap pool internal txs {:#?}", internal_txs);
        }
    }
}

fn refresh(geth: &geth::Client, sql: &mut sql::Client, eth_block: u32) {
    let sql_pool_count = uniswap::v2::Factory::sql_pool_count(sql);
    for pool_idx in 0..sql_pool_count {
        log::info!(
            "{:?}",
            uniswap::v2::Pool::find_by_uniswap_v2_index(pool_idx as i32)
        );
        let rows = sql.q(uniswap::v2::Pool::find_by_uniswap_v2_index(pool_idx as i32));
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
