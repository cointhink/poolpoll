use crate::geth::Client;
use ethabi::token::Token;
use ethabi::Contract;
use ethereum_types::Address;
use std::sync::OnceLock;

pub static ABI: OnceLock<Contract> = OnceLock::new();

#[derive(Debug, Default)]
pub struct Erc20 {
    pub address: Address,
}

impl Erc20 {
    pub fn name(&self, geth: &Client) -> Result<String, Box<dyn std::error::Error>> {
        let address_hex = format!("0x{}", hex::encode(self.address));
        let result = geth.eth_call(address_hex.clone(), &ABI.get().unwrap(), "name", &vec![])?;
        let Token::String(name) = &result[0] else { unreachable!() };
        Ok(name.to_owned())
    }
}
