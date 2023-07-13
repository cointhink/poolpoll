use crate::geth::{JsonInfuraRpcParam, JsonRpcParam};
use ethereum_tx_sign::Transaction;

mod config;
mod geth;
mod log;

fn main() {
    log::init();
    let config_file = config::load("config.yaml");
    config::CONFIG.set(config_file).unwrap();
    let config = config::CONFIG.get().unwrap();

    log::info!("poolpoll");
    let abi_file = std::fs::File::open("abi/uniswap_v2_factory.json").unwrap();
    let abi = ethabi::Contract::load(abi_file).unwrap();
    let data = abi
        .function("allPairsLength")
        .unwrap()
        .encode_input(&vec![])
        .unwrap();
    let url = format!("{}/{}", config.geth_url, config.infura_key);
    let geth = geth::Client::build(&url);
    let mut tx = geth::JsonRpcParam::new();

    const uniswap_v3_factory: &str = "0x1f98431c8ad98523631ae4a59f267346ea31f984";
    const uniswap_v2_factory: &str = "0x5c69bee701ef814a2b6a3edd4b1652cb9cc5aa6f";

    tx.insert(
        "to".to_string(),
        uniswap_v2_factory.to_string(),
    );
    tx.insert("data".to_string(), format!("0x{}", hex::encode(data)));
    let params = (tx.clone(), Some("latest".to_string()));
    let result = geth
        .call("eth_call", geth::ParamTypes::Infura(params))
        .unwrap();
    log::info!("{:?}", result);
}

fn sign() {
    let tx = ethereum_tx_sign::LegacyTransaction {
        chain: 1,
        nonce: 0,
        to: None,
        value: 0,
        gas_price: 0,
        gas: 0,
        data: vec![],
    };

    let private_key = hex::decode(&config::CONFIG.get().unwrap().private_key).unwrap();
    let ecdsa = tx.ecdsa(&private_key).unwrap();
    let _ = tx.sign(&ecdsa);
    log::info!("signed.")
}
