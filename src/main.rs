use ethereum_tx_sign::Transaction;

mod config;
mod log;
mod geth;

fn main() {
    log::init();
    let CONFIG = config::load("config.yaml");

    log::info!("poolpoll");
    let abi_file = std::fs::File::open("abi/uniswap_v2_factory.json").unwrap();
    let abi = ethabi::Contract::load(abi_file).unwrap();
    let data = abi
        .function("allPairsLength")
        .unwrap()
        .encode_input(&vec![])
        .unwrap();

    let tx = ethereum_tx_sign::LegacyTransaction {
        chain: 1,
        nonce: 0,
        to: None,
        value: 0,
        gas_price: 0,
        gas: 0,
        data: data,
    };

    let private_key = hex::decode(CONFIG.private_key).unwrap();
    let ecdsa = tx.ecdsa(&private_key).unwrap();
    let _ = tx.sign(&ecdsa);
    log::info!("signed.")
}
