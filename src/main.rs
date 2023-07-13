use crate::geth::{ResultTypes, RpcResultTypes};
use ethereum_tx_sign::Transaction;
use ethereum_types::U256;

mod config;
mod geth;
mod log;
mod sql;

const UNISWAP_V3_FACTORY: &str = "0x1f98431c8ad98523631ae4a59f267346ea31f984";
const UNISWAP_V2_FACTORY: &str = "0x5c69bee701ef814a2b6a3edd4b1652cb9cc5aa6f";

fn main() {
    log::init();
    config::CONFIG.set(config::load("config.yaml")).unwrap();
    log::info!("poolpoll");

    sql::init();
    let config = config::CONFIG.get().unwrap();

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

    tx.insert("to".to_string(), UNISWAP_V2_FACTORY.to_string());
    tx.insert("data".to_string(), format!("0x{}", hex::encode(data)));
    let params = (tx.clone(), Some("latest".to_string()));
    let result = geth
        .call("eth_call", geth::ParamTypes::Infura(params))
        .unwrap();
    match result.part {
        RpcResultTypes::Error(_) => {}
        RpcResultTypes::Result(ref r) => match &r.result {
            ResultTypes::String(rs) => {
                log::info!("{:?}", U256::from_str_radix(rs, 16))
            }
            ResultTypes::TransactionReceipt(_) => {}
            ResultTypes::Null => {}
        },
    }
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
