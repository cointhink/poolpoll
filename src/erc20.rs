use crate::geth::Client;
use ethabi::token::Token;
use ethabi::Contract;
use ethereum_types::Address;
use ethereum_types::U256;
use std::sync::OnceLock;

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
        match geth.eth_call(&self.address, &ABI.get().unwrap(), "name", &vec![]) {
            Ok(tokens) => Ok(tokens[0].to_string()),
            Err(e) => Ok(hex_to_ascii(&e.to_string())),
        }
    }
    pub fn symbol(&self, geth: &Client) -> Result<String, Box<dyn std::error::Error>> {
        match geth.eth_call(&self.address, &ABI.get().unwrap(), "symbol", &vec![]) {
            Ok(tokens) => Ok(tokens[0].to_string()),
            Err(e) => Ok(hex_to_ascii(&e.to_string())),
        }
    }
    pub fn decimals(&self, geth: &Client) -> Result<U256, Box<dyn std::error::Error>> {
        let result = geth.eth_call(&self.address, &ABI.get().unwrap(), "decimals", &vec![])?;
        let Token::Uint(decimals) = result[0] else { println!("{:?}", result[0]); unreachable!() };
        Ok(decimals)
    }
}
