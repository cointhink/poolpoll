use std::error::Error;

use serde::{Deserialize, Serialize};

pub struct Etherscan {
    api_key: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcResult {
    status: String,
    message: String,
    result: Vec<InternalTransaction>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InternalTransaction {
    from: String,
    to: String,
    r#type: String,
    value: String,
    //    {"blockNumber":"18224212","contractAddress":"","errCode":"","from":"0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2","gas":"2300","gasUsed":"55","input":"","isError":"0",
    //    "timeStamp":"1695782807","to":"0x1a2d11cb90d1de13bb81ee7b772a08ac234a8058","type":"call","value":"318405674863838"}
}

impl Etherscan {
    pub fn new(api_key: String) -> Self {
        Etherscan { api_key }
    }

    pub fn tx_list_internal(
        &self,
        address: String,
    ) -> Result<Vec<InternalTransaction>, Box<dyn Error>> {
        // https://api.etherscan.io/api
        //    ?module=account
        //    &action=txlistinternal
        //    &txhash=0x40eb908387324f2b575b4879cd9d7188f69c8fc9d87c901b9e2daaea4b442170
        //    &apikey=YourApiKeyToken

        let url = format!(
            "https://api.etherscan.io/api?module=account&action=txlistinternal&txhash={}&apikey={}",
            address, self.api_key
        );
        log::info!("{}", url);
        let result: RpcResult = ureq::get(&url).call().unwrap().into_json().unwrap();
        if result.message == "OK" {
            Ok(result.result)
        } else {
            Err(Box::from(format!("infura rpc status: {}", result.message)))
        }
    }
}
