use std::error::Error;

use serde::{Deserialize, Serialize};

const HOST: &str = "https://api.etherscan.io/api";

pub struct Etherscan {
    api_key: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcResult<T> {
    status: String,
    message: String,
    result: T,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InternalTransaction {
    from: String,
    to: String,
    r#type: String,
    value: String,
    // {"blockNumber":"18224212","contractAddress":"","errCode":"","from":"0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2","gas":"2300","gasUsed":"55","input":"","isError":"0",
    //  "timeStamp":"1695782807","to":"0x1a2d11cb90d1de13bb81ee7b772a08ac234a8058","type":"call","value":"318405674863838"}
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenTransfer {
    pub from: String,
    pub to: String,
    pub contract_address: String,
    pub transaction_index: String,
    pub hash: String,
    pub value: String,
    pub token_name: String,
    pub token_symbol: String,
    pub token_decimal: String,
    // { "blockNumber": "18224212", "timeStamp": "1695782807", "hash": "0x880f3a720cf0b20b5b895ad5c118654cc8c8d9a0e6e002d4858f6aee64f09b29", "nonce": "116", "blockHash": "0x938ff16353ac0fae4661c874100172fa30a7b96b9e999f564a6be94c4fe6699b", "from": "0xb4e16d0168e52d35cacd2c6185b44281ec28c9dc", "contractAddress": "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48", "to": "0x1a2d11cb90d1de13bb81ee7b772a08ac234a8058", "value": "67844355",
    //   "tokenName": "USDC", "tokenSymbol": "USDC", "tokenDecimal": "6", "transactionIndex": "14", "gas": "1023611", "gasPrice": "7175452897", "gasUsed": "639769", "cumulativeGasUsed": "2882244", "input": "deprecated", "confirmations": "47202" },
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
            "{}?module=account&action=txlistinternal&txhash={}&apikey={}",
            HOST, address, self.api_key
        );
        Self::call::<Vec<InternalTransaction>>(url)
    }

    pub fn tx_token_xfer(
        &self,
        address: String,
        block_number: u32,
    ) -> Result<Vec<TokenTransfer>, Box<dyn Error>> {
        let url = format!(
            "{}?module=account&action=tokentx&address={}&startblock={}&endblock={}apikey={}",
            HOST, address, block_number, block_number, self.api_key
        );
        Self::call::<Vec<TokenTransfer>>(url)
    }

    pub fn call<T>(url: String) -> Result<T, Box<dyn Error>>
    where
        for<'a> T: Deserialize<'a>,
    {
        log::info!(target: "http", "{}", url);
        let result: RpcResult<T> = ureq::get(&url).call().unwrap().into_json()?;
        if result.status == "1" {
            Ok(result.result)
        } else {
            Err(Box::from(format!(
                "etherscan rpc status: {}",
                result.message
            )))
        }
    }
}
