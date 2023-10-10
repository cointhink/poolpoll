use crate::geth::Client;
use crate::geth::InfuraLog;
use ethabi::token::Token;
use ethabi::Contract;
use ethereum_types::Address;
use ethereum_types::U256;
use std::sync::OnceLock;

// $ echo -n 'Transfer(address,address,uint256)' | sha3sum -a keccak256
pub const TOPIC_TRANSFER: &str =
    "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef";

pub static ABI: OnceLock<Contract> = OnceLock::new();

#[derive(Debug, Default)]
pub struct Erc20 {
    pub address: Address,
}

fn hex_to_ascii(str: &str) -> String {
    let utf8_bytes_null = hex::decode(str.to_string()).unwrap();
    let pos0 = utf8_bytes_null.iter().position(|&r| r == 0).unwrap();
    std::str::from_utf8(&utf8_bytes_null[0..pos0])
        .unwrap()
        .to_owned()
}

impl Erc20 {
    pub fn name(&self, geth: &Client) -> Result<String, Box<dyn std::error::Error>> {
        match geth.eth_call(&self.address, &ABI.get().unwrap(), "name", &vec![], None) {
            Ok(tokens) => Ok(tokens[0].to_string()),
            Err(e) => Ok(hex_to_ascii(&e.to_string())),
        }
    }
    pub fn symbol(&self, geth: &Client) -> Result<String, Box<dyn std::error::Error>> {
        match geth.eth_call(&self.address, &ABI.get().unwrap(), "symbol", &vec![], None) {
            Ok(tokens) => Ok(tokens[0].to_string()),
            Err(e) => Ok(hex_to_ascii(&e.to_string())),
        }
    }
    pub fn decimals(&self, geth: &Client) -> Result<U256, Box<dyn std::error::Error>> {
        let result = geth.eth_call(
            &self.address,
            &ABI.get().unwrap(),
            "decimals",
            &vec![],
            None,
        )?;
        let Token::Uint(decimals) = result[0] else {
            println!("{:?}", result[0]);
            unreachable!()
        };
        Ok(decimals)
    }
}

pub fn topic_filter_transfer(log: &&InfuraLog) -> bool {
    if log.topics[0] == TOPIC_TRANSFER && log.data.len() > 2 {
        if log.topics.len() == 3 {
            log::info!(
                "swap from {} to {} value {} ",
                log.topics[1],
                log.topics[2],
                ethereum_types::U256::from_str_radix(log.data.strip_prefix("0x").unwrap(), 16)
                    .unwrap(),
            );
            true
        } else {
            log::info!(
                "warning: log is swap but only {} topics {:?}",
                log.topics.len(),
                log
            );
            false
        }
    } else {
        false
    }
}
