use std::error::Error;
use std::thread;
use std::time::Duration;

use crate::coin::Coin;
use crate::erc20::Erc20;
use crate::geth::{InfuraBlock, InfuraLog};
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

    let last_block_number = geth.last_block_number();
    log::info!("eth last block number {}", last_block_number);
    if std::env::args().find(|arg| arg == "discover").is_some() {
        discover(&geth, &mut sql);
    } else if std::env::args().find(|arg| arg == "refresh").is_some() {
        refresh(&geth, &mut sql, last_block_number);
    } else if std::env::args().find(|arg| arg == "tail").is_some() {
        tail_from(&geth, &mut sql, last_block_number);
    } else {
        log::info!("commands: discover, refresh, tail")
    }
}

fn tail_from(geth: &geth::Client, mut db: &mut sql::Client, last_block_number: u32) {
    let mut geth_block_number = last_block_number;
    loop {
        let started = std::time::Instant::now();
        let db_block_number = match InfuraBlock::last_block_number(&mut db) {
            Some(number) => number,
            None => last_block_number - (60 / 12 * 60 * 24), // 1 day in eth blocks
        };
        if db_block_number < geth_block_number {
            let fetch_block_number = db_block_number + 1;
            log::info!(
                "fetching block {} geth_block_number {} db_block_number {}",
                fetch_block_number,
                geth_block_number,
                db_block_number
            );
            let block = geth.block(fetch_block_number);
            let logs = geth.logs(fetch_block_number);
            for log in &logs {
                db.insert(log.to_upsert_sql())
            }
            let erc20_transfer_logs = logs
                .iter()
                .filter(erc20::topic_filter_transfer)
                .collect::<Vec<&InfuraLog>>();
            let uniswap_swap_logs = logs
                .iter()
                .filter(uniswap::v2::topic_filter_swap)
                .collect::<Vec<&InfuraLog>>();
            log::info!(
                "block #{} {} logs. {} erc20 transfer logs. {} uniswap swap logs",
                fetch_block_number,
                logs.len(),
                erc20_transfer_logs.len(),
                uniswap_swap_logs.len()
            );
            let abi_file = std::fs::File::open("abi/uniswap_v2_pair.json").unwrap();
            let abi_pool = ethabi::Contract::load(abi_file).unwrap();
            for log in uniswap_swap_logs {
                let sql = uniswap::v2::Pool::find_by_contract_address(log.address.as_str().into());
                let rows = db.q(sql);
                if rows.len() > 0 {
                    //let pool = uniswap::v2::Pool::from(&rows[0]);
                } else {
                    let log_address = Address::from_slice(
                        &hex::decode(log.address.strip_prefix("0x").unwrap()).unwrap(),
                    );
                    match create_pool(geth, db, &abi_pool, log_address) {
                        Ok(pool) => match update_pool_reserves(geth, db, &pool, fetch_block_number)
                        {
                            Ok(_) => (),
                            Err(err) => log::info!("warning: pool reserves update failed. {}", err),
                        },
                        Err(_) => log::info!(
                            "warning: block {} tx #{} pool creation {} failed",
                            block.number,
                            log.transaction_index,
                            hex::encode(log_address),
                        ),
                    }
                }
            }
            // mark block as visited
            db.insert(block.to_upsert_sql());
            log::info!("{} seconds", started.elapsed().as_secs());
            if geth_block_number == fetch_block_number {
                // are we caught up?
                log::info!("sleeping 5 sec at block {}", db_block_number);
                thread::sleep(Duration::from_secs(5)); // then sleep
                log::info!("updating block number");
                geth_block_number = geth.last_block_number();
            }
        }
    }
}

fn refresh(geth: &geth::Client, db: &mut sql::Client, eth_block: u32) {
    let sql = uniswap::v2::Pool::all();
    let rows = db.q(sql);
    let rows_count = rows.len();
    for (idx, row) in rows.iter().enumerate() {
        let pool = uniswap::v2::Pool::from(row);
        log::info!("refresh: {}/{} {:?}", idx, rows_count, pool);
        match update_pool_reserves(geth, db, &pool, eth_block) {
            Ok(_) => (),
            Err(err) => log::info!("warning: pool reserves update failed. {}", err),
        };
    }
}

fn discover(geth: &geth::Client, sql: &mut sql::Client) {
    uniswap::v2::Factory::setup();
    let pool_count = uniswap::v2::Factory::pool_count(&geth).unwrap().low_u64();
    log::info!("Uniswap v2 contract count {:?}", pool_count,);
    let abi_file = std::fs::File::open("abi/uniswap_v2_pair.json").unwrap();
    let abi_pool = ethabi::Contract::load(abi_file).unwrap();
    for pool_idx in pool_count - 10..pool_count {
        let address = uniswap::v2::Factory::pool_addr(&geth, pool_idx).unwrap();
        match create_pool(geth, sql, &abi_pool, address) {
            Ok(_) => (),
            Err(err) => log::info!(
                "warning: pool creation {} failed: {}",
                hex::encode(address),
                err
            ),
        }
    }
}

fn create_pool(
    geth: &geth::Client,
    sql: &mut sql::Client,
    abi_pool: &ethabi::Contract,
    address: Address,
) -> Result<uniswap::v2::Pool, Box<dyn Error>> {
    let tokens = crate::uniswap::v2::Pool::tokens(&geth, &abi_pool, &address)?;
    let pool = uniswap::v2::Pool {
        contract_address: address,
        token0: tokens.0,
        token1: tokens.1,
    };
    refresh_token(&geth, sql, tokens.0)?;
    refresh_token(&geth, sql, tokens.1)?;

    log::info!("Created {:?}", pool);
    sql.insert(pool.to_upsert_sql());
    Ok(pool)
}

fn update_pool_reserves<'a>(
    geth: &geth::Client,
    sql: &mut sql::Client,
    pool: &'a uniswap::v2::Pool,
    eth_block: u32,
) -> Result<uniswap::v2::Reserves<'a>, Box<dyn Error>> {
    let abi_file = std::fs::File::open("abi/uniswap_v2_pair.json").unwrap();
    let abi_pool = ethabi::Contract::load(abi_file).unwrap();
    let reserves =
        uniswap::v2::Pool::reserves(&geth, &abi_pool, &pool.contract_address, eth_block)?;
    let pool_reserves = uniswap::v2::Reserves::new(&pool, eth_block, reserves);
    sql.insert(pool_reserves.to_upsert_sql());
    Ok(pool_reserves)
}

fn refresh_token(
    geth: &crate::geth::Client,
    sql: &mut crate::sql::Client,
    address: Address,
) -> Result<Coin, Box<dyn Error>> {
    let exist = sql_query_builder::Select::new()
        .select("*")
        .from("coins")
        .where_clause("contract_address = $1");
    let rows = sql.q((exist.to_string(), vec![Box::new(format!("{:x}", address))]));
    if rows.len() == 0 {
        let token = Erc20 { address };
        let name = token.name(&geth).unwrap_or_default();
        let symbol = token.symbol(&geth).unwrap_or_default();
        if let Ok(decimals) = token.decimals(&geth) {
            let coin = Coin {
                contract_address: token.address,
                name,
                symbol,
                decimals,
            };
            sql.insert(coin.to_upsert_sql());
            log::info!("Created {:?}", coin);
            Ok(coin)
        } else {
            Err(Box::from(format!("coin decimals() failed for {}", address)))
        }
    } else {
        Ok(Coin::from(&rows[0]))
    }
}
