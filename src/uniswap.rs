use crate::geth::{Client, ResultTypes, RpcResultTypes};
use crate::{config, geth};
use ethabi::Contract;
use ethereum_types::U256;

const UNISWAP_V3_FACTORY: &str = "0x1f98431c8ad98523631ae4a59f267346ea31f984";
const UNISWAP_V2_FACTORY: &str = "0x5c69bee701ef814a2b6a3edd4b1652cb9cc5aa6f";

pub(crate) struct V2 {
    abi: Contract,
}

impl V2 {
    pub(crate) fn new(abi: Contract) -> Self {
        let config = config::CONFIG.get().unwrap();
        return V2 { abi: abi };
    }

    pub(crate) fn pool_count(&self, geth: &Client) -> Result<U256, String> {
        let data = self
            .abi
            .function("allPairsLength")
            .unwrap()
            .encode_input(&vec![])
            .unwrap();
        let mut tx = geth::JsonRpcParam::new();

        tx.insert("to".to_string(), UNISWAP_V2_FACTORY.to_string());
        tx.insert("data".to_string(), format!("0x{}", hex::encode(data)));
        let params = (tx.clone(), Some("latest".to_string()));
        let result = geth
            .call("eth_call", geth::ParamTypes::Infura(params))
            .unwrap();
        match result.part {
            RpcResultTypes::Error(_) => Err("s".to_owned()),
            RpcResultTypes::Result(ref r) => match &r.result {
                ResultTypes::String(rs) => {
                    return match U256::from_str_radix(rs, 16) {
                        Ok(u) => Ok(u),
                        Err(_) => return Err("boo".to_owned()),
                    }
                }
                ResultTypes::TransactionReceipt(_) => return Err("a".to_owned()),
                ResultTypes::Null => return Err("Null".to_owned()),
            },
        }
    }
    pub(crate) fn pool(&self, geth: &Client, pool_id: u64) -> Result<U256, String> {
        return Ok(2.try_into().unwrap());
    }
}
