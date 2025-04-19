use std::error::Error;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::coin::Coin;
use crate::erc20::Erc20;
use crate::geth::{InfuraBlock, InfuraLog};
use crate::sql::Ops;
use crate::uniswap::v2::SwapCall;
use ethereum_types::{Address, U256};
use num_traits::Num;
use pg_bigdecimal::BigInt;
use std::ops::{Div, Mul};

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

    let last_chain_block_number = geth.last_block_number();
    log::info!("ethereum mainnet latest block #{}", last_chain_block_number);
    let first_db_block_number = InfuraBlock::last_db_block_number(&mut sql, false).unwrap();
    let last_db_block_number = InfuraBlock::last_db_block_number(&mut sql, true).unwrap_or(
        last_chain_block_number - (60 / 12 * 60 * 24), // start 1 day in eth blocks ago
    );
    let db_block_range = last_db_block_number - first_db_block_number;
    log::info!(
        "db block range #{} - #{} = {} blocks ({:.4} days)",
        first_db_block_number,
        last_db_block_number,
        db_block_range,
        db_block_range as f32 * 12.0 / 60.0 / 60.0 / 60.0
    );
    if std::env::args().find(|arg| arg == "discover").is_some() {
        discover(&geth, &mut sql);
    } else if std::env::args().find(|arg| arg == "refresh").is_some() {
        refresh(&geth, &mut sql, last_chain_block_number);
    } else if std::env::args().find(|arg| arg == "tail").is_some() {
        tail(
            &geth,
            &mut sql,
            last_db_block_number,
            last_chain_block_number,
        );
    } else {
        log::info!("commands: discover, refresh, tail")
    }
}

fn tail(
    geth: &geth::Client,
    db: &mut sql::Client,
    db_block_number: u32,
    last_chain_block_number: u32,
) {
    let mut geth_block_number = last_chain_block_number;
    loop {
        let started = std::time::Instant::now();
        log::info!(
            "tail db_block_number {} geth_block_number {}",
            db_block_number,
            geth_block_number
        );
        if db_block_number < geth_block_number {
            let fetch_block_number = db_block_number + 1;
            match geth.block(fetch_block_number) {
                Ok(block) => match geth.logs(fetch_block_number) {
                    Ok(logs) => {
                        process_logs_and_mark_block(geth, db, fetch_block_number, logs, &block);
                        let elapsed_secs = started.elapsed().as_secs_f32();
                        log::info!(
                            "processed in {:.1} seconds. db #{}. eth #{}. {} blocks / {} behind.",
                            elapsed_secs,
                            db_block_number,
                            geth_block_number,
                            geth_block_number - db_block_number,
                            elapsed_in_words(seconds_since_block(&block)),
                        );
                    }
                    Err(e) => {
                        log::info!("block {} logs fetch failed: {:?}", fetch_block_number, e)
                    }
                },
                Err(e) => log::info!("tail_from eth block get failed {:?}", e),
            }
        }
        // are we caught up?
        if db_block_number >= geth_block_number {
            geth_block_number = geth.last_block_number();
            if db_block_number >= geth_block_number {
                log::info!(
                    "sleeping 10 sec at db #{} eth #{}",
                    db_block_number,
                    geth_block_number
                );
                thread::sleep(Duration::from_secs(10)); // then sleep
            }
        }
    }
}

fn process_logs_and_mark_block(
    geth: &geth::Client,
    db: &mut sql::Client,
    fetch_block_number: u32,
    logs: Vec<InfuraLog>,
    block: &InfuraBlock,
) {
    let mut db = sql::TransactionClient::new(db);
    match process_logs(geth, &mut db, fetch_block_number, logs) {
        Ok(_) => {
            // mark block as visited
            db.q(block.to_upsert_sql());
            db.client.commit().unwrap();
        }
        Err(e) => {
            db.client.rollback().unwrap();
            log::info!("block {} processing failed: {}", fetch_block_number, e)
        }
    }
}

fn seconds_since_block(block: &InfuraBlock) -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
        - (block.timestamp as u64)
}

fn process_logs(
    geth: &geth::Client,
    db: &mut sql::TransactionClient,
    fetch_block_number: u32,
    logs: Vec<InfuraLog>,
) -> Result<(), Box<dyn Error>> {
    let mut topic_swap_count = 0;
    let mut topic_sync_count = 0;
    let mut topic_transfer_count = 0;
    for log in &logs {
        db.q(log.to_upsert_sql());
        if log.topics.len() > 0 {
            let _ = match log.topics[0].as_str() {
                uniswap::v2::TOPIC_SWAP => {
                    topic_swap_count += 1;
                    process_swap(db, log, fetch_block_number)
                }
                uniswap::v2::TOPIC_SYNC => {
                    topic_sync_count += 1;
                    process_sync(geth, db, log, fetch_block_number)
                }
                erc20::TOPIC_TRANSFER => {
                    topic_transfer_count += 1;
                    Ok(())
                }
                _ => Ok(()),
            };
        }
    }
    log::info!(
        "#{} {} logs. {} erc20 transfer logs. uniswap {} swaps {} syncs",
        fetch_block_number,
        logs.len(),
        topic_transfer_count,
        topic_swap_count,
        topic_sync_count,
    );
    Ok(())
}

fn process_sync(
    geth: &geth::Client,
    db: &mut sql::TransactionClient,
    log: &InfuraLog,
    fetch_block_number: u32,
) -> Result<(), Box<dyn Error>> {
    let pool = ensure_pool(geth, db, &log.address)?;
    let reserves = (
        U256::from_str_radix(&log.data[2..66], 16).unwrap(),
        U256::from_str_radix(&log.data[66..130], 16).unwrap(),
    );
    log::info!(
        "#{} tx {:0>3} log sync( pool {} reserves {:?} )",
        fetch_block_number,
        log.transaction_index,
        log.address.strip_prefix("0x").unwrap(),
        reserves,
    );
    update_pool_reserves(db, &pool, fetch_block_number, reserves)?;
    Ok(())
}

fn process_swap(
    db: &mut sql::TransactionClient,
    log: &InfuraLog,
    block_number: u32,
) -> Result<(), Box<dyn Error>> {
    let swap_call = SwapCall::from(log);
    let sql = uniswap::v2::Pool::find_by_contract_address(log.address.as_str().into());
    match db.first(sql) {
        Some(row) => {
            let pool = uniswap::v2::Pool::from(&row);
            let mut in0_eth = BigInt::from(0);
            let mut in1_eth = BigInt::from(0);
            let sql = uniswap::v2::Reserves::find_by_pool(&pool);
            match db.first(sql) {
                Some(row) => {
                    let reserves = uniswap::v2::Reserves::from_row(&row, &pool);
                    let x = BigInt::from_str_radix(&reserves.x.to_string(), 10).unwrap();
                    let y = BigInt::from_str_radix(&reserves.y.to_string(), 10).unwrap();
                    if is_cash_token(pool.token0) {
                        in0_eth = swap_call.in0.clone();
                        in1_eth = if y > BigInt::from(0) {
                            swap_call.in1.clone().mul(x).div(y)
                        } else {
                            BigInt::from(0)
                        };
                    } else if is_cash_token(pool.token1) {
                        in0_eth = if x > BigInt::from(0) {
                            swap_call.in0.clone().mul(y).div(x)
                        } else {
                            BigInt::from(0)
                        };
                        in1_eth = swap_call.in1.clone();
                    }
                }
                None => log::info!(
                    "Warning: swap recorded with no reserves available for pool {}",
                    pool.contract_address
                ),
            }
            log::info!(
                "#{} tx {:0>3} log swap( pool {} swap in0 {} in0_eth {:?} in1 {} in1_eth {:?} out0 {} out1 {} )",
                block_number,
                log.transaction_index,
                log.address.strip_prefix("0x").unwrap(),
                swap_call.in0,
                in0_eth,
                swap_call.in1,
                in1_eth,
                swap_call.out0,
                swap_call.out1
            );
            let swap = uniswap::v2::Swap {
                pool: &pool,
                block_number: block_number as u128,
                transaction_index: log.transaction_index,
                in0_eth,
                in1_eth,
                call_params: swap_call,
            };
            db.q(swap.to_upsert_sql());
        }
        None => {
            log::warn!("process_swap could not find pool in db {}", log.address);
        }
    }
    Ok(())
}

fn is_cash_token(token_address: Address) -> bool {
    let address = format!("{:x}", token_address);
    match address.as_str() {
        "c02aaa39b223fe8d0a0e5c4f27ead9083c756cc2" => true, // WETH
        _ => false,
    }
}

fn ensure_pool(
    geth: &geth::Client,
    db: &mut sql::TransactionClient,
    address: &str,
) -> Result<uniswap::v2::Pool, Box<dyn Error>> {
    let sql = uniswap::v2::Pool::find_by_contract_address(address.into());
    match db.first(sql) {
        Some(pool_row) => Ok(uniswap::v2::Pool::from(&pool_row)),
        None => {
            let abi_file = std::fs::File::open("abi/uniswap_v2_pair.json").unwrap();
            let abi_uniswap_pair = ethabi::Contract::load(abi_file).unwrap();
            let log_address =
                Address::from_slice(&hex::decode(address.strip_prefix("0x").unwrap()).unwrap());
            match create_pool(geth, db, &abi_uniswap_pair, log_address) {
                Ok(pool) => Ok(pool),
                Err(e) => {
                    log::warn!("pool creation {} failed: {}", hex::encode(log_address), e);
                    Err(Box::from(e))
                }
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
        let abi_file = std::fs::File::open("abi/uniswap_v2_pair.json").unwrap();
        let abi_pool = ethabi::Contract::load(abi_file).unwrap();
        let reserves =
            uniswap::v2::Pool::reserves(&geth, &abi_pool, &pool.contract_address, eth_block)
                .unwrap();
        let mut db = sql::TransactionClient::new(db);
        match update_pool_reserves(&mut db, &pool, eth_block, reserves) {
            Ok(_) => {
                db.client.commit().unwrap();
            }
            Err(err) => {
                db.client.rollback().unwrap();
                log::info!("warning: pool reserves update failed. {}", err)
            }
        };
    }
}

fn discover(geth: &geth::Client, db: &mut sql::Client) {
    uniswap::v2::Factory::setup();
    let pool_count = uniswap::v2::Factory::pool_count(&geth).unwrap().low_u64();
    log::info!("Uniswap v2 contract count {:?}", pool_count,);
    let abi_file = std::fs::File::open("abi/uniswap_v2_pair.json").unwrap();
    let abi_pool = ethabi::Contract::load(abi_file).unwrap();
    for pool_idx in pool_count - 10..pool_count {
        let address = uniswap::v2::Factory::pool_addr(&geth, pool_idx).unwrap();
        let mut db = sql::TransactionClient::new(db);
        match create_pool(geth, &mut db, &abi_pool, address) {
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
    db: &mut sql::TransactionClient,
    abi_pool: &ethabi::Contract,
    address: Address,
) -> Result<uniswap::v2::Pool, Box<dyn Error>> {
    let tokens = crate::uniswap::v2::Pool::tokens(&geth, &abi_pool, &address)?;
    let pool = uniswap::v2::Pool {
        contract_address: address,
        token0: tokens.0,
        token1: tokens.1,
    };
    create_token(&geth, db, tokens.0)?;
    create_token(&geth, db, tokens.1)?;

    log::info!("Created {:?}", pool);
    db.q(pool.to_upsert_sql());
    Ok(pool)
}

fn update_pool_reserves<'a>(
    db: &mut sql::TransactionClient,
    pool: &'a uniswap::v2::Pool,
    eth_block: u32,
    reserves: (U256, U256),
) -> Result<uniswap::v2::Reserves<'a>, Box<dyn Error>> {
    let pool_reserves = uniswap::v2::Reserves::new(&pool, eth_block, reserves);
    db.q(pool_reserves.to_upsert_sql());
    Ok(pool_reserves)
}

fn create_token(
    geth: &crate::geth::Client,
    db: &mut sql::TransactionClient,
    address: Address,
) -> Result<Coin, Box<dyn Error>> {
    let exist = sql_query_builder::Select::new()
        .select("*")
        .from("coins")
        .where_clause("contract_address = $1");
    let rows = db.q((exist.to_string(), vec![Box::new(format!("{:x}", address))]));
    if rows.len() == 0 {
        let token = Erc20 { address };
        let mut name = token.name(&geth).unwrap_or_else(|e| {
            log::info!("warning: token decode fail: {:?}", e);
            "".to_string()
        });
        string_filter_null(&mut name); // psql does not allow nulls
        let mut symbol = token.symbol(&geth).unwrap_or_else(|e| {
            log::info!("warning: symbol decode fail: {:?}", e);
            "".to_string()
        });
        string_filter_null(&mut symbol);
        if let Ok(decimals) = token.decimals(&geth) {
            let coin = Coin {
                contract_address: token.address,
                name,
                symbol,
                decimals,
            };
            db.q(coin.to_upsert_sql());
            log::info!("Created {:?}", coin);
            Ok(coin)
        } else {
            Err(Box::from(format!("coin decimals() failed for {}", address)))
        }
    } else {
        Ok(Coin::from(&rows[0]))
    }
}

fn string_filter_null(str: &mut String) {
    str.retain(|c| c != '\0')
}

fn elapsed_in_words(secs: u64) -> String {
    let mut secs = secs;
    let mut msg = "".to_string();
    if secs > 60 * 60 * 24 {
        let days = secs / 60 / 60 / 24;
        msg.push_str(&format!("{} days ", days));
        secs = secs - days * 60 * 60 * 24;
    }
    if secs > 60 * 60 {
        let hours = secs / 60 / 60;
        msg.push_str(&format!("{} hours ", hours));
        secs = secs - hours * 60 * 60;
    }
    if secs > 60 {
        let mins = secs / 60;
        msg.push_str(&format!("{} mins ", mins));
        secs = secs - mins * 60;
    }
    msg.push_str(&format!("{} secs", secs));
    return msg;
}

#[cfg(test)]
mod tests {
    use crate::uniswap::v2::{Pool, Reserves};

    use super::*;

    #[test]
    fn test_token0_to_token0_eth() {
        let pool = Pool {
            contract_address: [0; 20].into(),
            token0: [0; 20].into(),
            token1: [0; 20].into(),
        };
        let reserves = Reserves {
            pool: &pool,
            block_number: 1,
            x: U256::from_str_radix("33044264430781", 10).unwrap(),
            y: U256::from_str_radix("16632437277688007258761", 10).unwrap(),
        };

        let x = BigInt::from_str_radix(&reserves.x.to_string(), 10).unwrap();
        let y = BigInt::from_str_radix(&reserves.y.to_string(), 10).unwrap();

        let in0 = BigInt::from_str_radix("1200000000", 10).unwrap();
        let in0_eth = in0.mul(y).div(x);

        assert_eq!(
            in0_eth,
            BigInt::from_str_radix("604005720116248332", 10).unwrap()
        )
    }
}
