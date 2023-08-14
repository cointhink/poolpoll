use crate::geth::Client;
use ethabi::Contract;
use ethereum_types::Address;
use std::sync::OnceLock;

pub static ABI: OnceLock<Contract> = OnceLock::new();

#[derive(Debug)]
pub struct Erc20 {
    pub address: Address,
    // pub name: String,
    // pub symbol: String,
}

impl Erc20 {
    fn name(&self, geth: &Client, abi: &Contract) -> Result<String, Box<dyn std::error::Error>> {
        let address_hex = format!("0x{}", hex::encode(self.address));
        let result = geth.eth_call(address_hex.clone(), abi, "name", &vec![])?;
        Ok(result)
    }
}
