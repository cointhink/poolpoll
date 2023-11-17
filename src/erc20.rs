use crate::geth::Client;
use ethabi::token::Token;
use ethabi::Contract;
use ethereum_types::Address;
use ethereum_types::U256;
use std::error::Error;
use std::sync::OnceLock;

// $ echo -n 'Transfer(address,address,uint256)' | sha3sum -a keccak256
pub const TOPIC_TRANSFER: &str =
    "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef";

pub static ABI: OnceLock<Contract> = OnceLock::new();

#[derive(Debug, Default)]
pub struct Erc20 {
    pub address: Address,
}

fn hex_to_ascii(str: &str) -> Result<String, Box<dyn Error>> {
    let utf8_bytes_null = hex::decode(str.to_string())?;
    let pos0 = utf8_bytes_null.iter().position(|&r| r == 0).unwrap_or(0);
    Ok(std::str::from_utf8(&utf8_bytes_null[0..pos0])
        .unwrap()
        .to_owned())
}

impl Erc20 {
    pub fn name(&self, geth: &Client) -> Result<String, Box<dyn std::error::Error>> {
        match geth.eth_call(&self.address, &ABI.get().unwrap(), "name", &vec![], None) {
            Ok(tokens) => Ok(tokens[0].to_string()),
            Err(e) => hex_to_ascii(&e.to_string()), //fallback decode
        }
    }
    pub fn symbol(&self, geth: &Client) -> Result<String, Box<dyn std::error::Error>> {
        match geth.eth_call(&self.address, &ABI.get().unwrap(), "symbol", &vec![], None) {
            Ok(tokens) => Ok(tokens[0].to_string()),
            Err(e) => hex_to_ascii(&e.to_string()), //fallback decode
        }
    }
    pub fn decimals(&self, geth: &Client) -> Result<u32, Box<dyn std::error::Error>> {
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
        Ok(decimals.low_u32())
    }
}
