use ethereum_types::Address;

#[derive(Debug)]
pub struct Coin {
    pub address: Address,
    pub name: String,
}
