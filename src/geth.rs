use bs58;
use ethabi::token::Token;
use ethabi::Contract;
use ethereum_types::Address;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionReceipt {
    pub status: String,
    pub cumulative_gas_used: String,
}

pub struct Client {
    url: String,
}

impl Client {
    pub fn build(url: &str, key: &str) -> Client {
        let url = format!("{}/{}", url, key);
        Client { url }
    }

    pub fn eth_call(
        &self,
        to: &Address,
        abi: &Contract,
        function_name: &str,
        function_params: &[Token],
    ) -> Result<Vec<Token>, Box<dyn std::error::Error>> {
        let function_call = abi.function(function_name).unwrap();
        let function_input = function_call.encode_input(function_params).unwrap();
        let to_hex = format!("0x{}", hex::encode(to));
        let tx = tx_build(to_hex, function_input);
        let params = (tx, Some("latest".to_string()));
        println!("geth {} {} {:?}", self.url, function_name, function_params);
        let output = self.rpc_str("eth_call", ParamTypes::Infura(params))?;
        let output_no_0x = output.strip_prefix("0x").unwrap();
        let output_bytes = hex::decode(output_no_0x).unwrap();
        match function_call.decode_output(&output_bytes) {
            Err(_) => Err(output_no_0x.into()),
            Ok(tokens) => Ok(tokens),
        }
    }

    pub fn rpc_str(
        &self,
        method: &str,
        params: ParamTypes,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let result = self.rpc(method, params);
        match result {
            Ok(rpc_result) => match rpc_result.part {
                RpcResultTypes::Error(e) => Err(Box::try_from(e.error.message).unwrap()),
                RpcResultTypes::Result(r) => {
                    let str_ret = match r.result {
                        ResultTypes::String(s) => s,
                        _ => "-bad response".to_string(),
                    };
                    Ok(str_ret)
                }
            },
            Err(e) => Err(e),
        }
    }

    pub fn last_block(&self) -> u32 {
        let blk_num_str = self
            .rpc_str("eth_blockNumber", ParamTypes::Single(("".to_string(),)))
            .unwrap();
        u32::from_str_radix(&blk_num_str[2..], 16).unwrap()
    }

    pub fn nonce(&self, addr: &str) -> Result<u32, Box<dyn error::Error>> {
        let params = (addr.to_string(), "latest".to_string());
        let tx_count_str =
            self.rpc_str("eth_getTransactionCount", ParamTypes::InfuraSingle(params))?;
        Ok(u32::from_str_radix(&tx_count_str[2..], 16).unwrap())
    }

    pub fn rpc(
        &self,
        method: &str,
        params: ParamTypes,
    ) -> Result<JsonRpcResult, Box<dyn std::error::Error>> {
        let jrpc = JsonRpc {
            jsonrpc: "2.0".to_string(),
            id: gen_id(),
            method: method.to_string(),
            params: params,
        };
        let result = ureq::post(&self.url).send_json(&jrpc);
        match result {
            Ok(res) => {
                let rpc_result = res.into_json::<JsonRpcResult>().unwrap();
                Ok(rpc_result)
            }
            Err(e) => Err(Box::new(e)),
        }
    }
}

fn tx_build(to: String, data: Vec<u8>) -> JsonRpcParam {
    let mut tx = JsonRpcParam::new();

    tx.insert("to".to_string(), to);
    tx.insert("data".to_string(), format!("0x{}", hex::encode(data)));
    return tx;
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpc {
    pub jsonrpc: String,
    pub id: String,
    pub method: String,
    pub params: ParamTypes,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ParamTypes {
    Standard(JsonRpcParam),
    Single(SingleParam),
    Infura(JsonInfuraRpcParam),
    InfuraSingle(InfuraSingleParam),
}

pub type JsonRpcParam = HashMap<String, String>;
pub type SingleParam = (String,);
pub type InfuraSingleParam = (String, String);
pub type JsonInfuraRpcParam = (JsonRpcParam, Option<String>);

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcResult {
    pub jsonrpc: String,
    pub id: String,
    #[serde(flatten)]
    pub part: RpcResultTypes,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RpcResultTypes {
    Error(ErrorRpc),
    Result(ResultRpc),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ResultTypes {
    String(String),
    TransactionReceipt(TransactionReceipt),
    Null,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResultRpc {
    pub result: ResultTypes,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorRpc {
    pub error: ErrorDetailRpc,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorDetailRpc {
    pub code: i32,
    pub message: String,
}

impl std::fmt::Display for ErrorDetailRpc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("#{} {}", self.code, self.message))
    }
}

pub fn gen_id() -> String {
    let mut pad = [0u8; 6];
    rand::thread_rng().fill(&mut pad);
    bs58::encode(pad).into_string()
}

/*
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EthGasStationResult {
    pub fast: f32,
    pub fastest: f32,
    pub safe_low: f32,
    pub average: f32,
}

pub fn ethgasstation() -> EthGasStationResult {
    let url = "https://ethgasstation.info/api/ethgasAPI.json";
    let result = ureq::get(url).call().unwrap();
    result.into_json().unwrap()
}

pub fn ethgasstation_fast() -> u64 {
    let gas_prices = ethgasstation();
    (gas_prices.fast as f64 * 100_000_000u64 as f64) as u64
}
*/
