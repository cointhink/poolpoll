use bs58;
use ethabi::token::Token;
use ethabi::Contract;
use ethereum_types::Address;
use rand::Rng;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
        block_number: Option<u32>,
    ) -> Result<Vec<Token>, Box<dyn std::error::Error>> {
        let function_call = abi.function(function_name).unwrap();
        let function_input = function_call.encode_input(function_params).unwrap();
        let to_hex = format!("0x{}", hex::encode(to));
        let tx = tx_build(to_hex, function_input);
        let params = (tx, infura_block_param(block_number));
        let output = self.rpc_str("eth_call", ParamTypes::Infura(params))?;
        let output_no_0x = output.strip_prefix("0x").unwrap();
        let output_bytes = hex::decode(output_no_0x).unwrap();
        match function_call.decode_output(&output_bytes) {
            Err(_) => Err(output_no_0x.into()),
            Ok(tokens) => {
                println!(
                    "geth {}({:?}) => {:?}",
                    function_name, function_params, tokens
                );
                Ok(tokens)
            }
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
                        _ => "-bad non-string response".to_string(),
                    };
                    Ok(str_ret)
                }
            },
            Err(e) => Err(e),
        }
    }

    pub fn last_block_number(&self) -> u32 {
        let blk_num_str = self.rpc_str("eth_blockNumber", ParamTypes::Empty).unwrap();
        u32::from_str_radix(&blk_num_str[2..], 16).unwrap()
    }

    pub fn block(&self, block_number: u32) -> InfuraBlock {
        let infura_block_number = infura_block_param(Some(block_number));
        let params = (infura_block_number, true);
        match self
            .rpc("eth_getBlockByNumber", ParamTypes::EthBlockByHash(params))
            .unwrap()
            .part
        {
            RpcResultTypes::Result(r) => {
                if let ResultTypes::Block(b) = r.result {
                    b
                } else {
                    todo!()
                }
            }
            RpcResultTypes::Error(_) => todo!(),
        }
    }

    pub fn logs(&self, block_number: u32) -> Vec<InfuraLog> {
        let infura_block_number = infura_block_param(Some(block_number));
        let mut rpc_param = JsonRpcParam::new();
        rpc_param.insert("fromBlock".to_owned(), infura_block_number.clone());
        rpc_param.insert("toBlock".to_owned(), infura_block_number.clone());
        match self
            .rpc("eth_getLogs", ParamTypes::Standard(vec![rpc_param]))
            .unwrap()
            .part
        {
            RpcResultTypes::Result(r) => {
                if let ResultTypes::Logs(logs) = r.result {
                    log::info!("got {} logs", logs.len());
                    logs
                } else {
                    todo!()
                }
            }
            RpcResultTypes::Error(e) => {
                log::info!("{:?}", e);
                todo!()
            }
        }
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

fn infura_block_param(block_number: Option<u32>) -> String {
    match block_number {
        Some(number) => format!("0x{:x}", number),
        None => "latest".to_string(),
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
    Empty,
    Standard(Vec<JsonRpcParam>),
    Single(SingleParam),
    Infura(JsonInfuraRpcParam),
    InfuraSingle(InfuraSingleParam),
    EthBlockByHash((String, bool)),
}

pub type JsonRpcParam = HashMap<String, String>;
pub type SingleParam = (String,);
pub type InfuraSingleParam = (String, String);
pub type JsonInfuraRpcParam = (JsonRpcParam, String);

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
pub struct ResultRpc {
    pub result: ResultTypes,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ResultTypes {
    String(String),
    TransactionReceipt(TransactionReceipt),
    Block(InfuraBlock),
    Logs(Vec<InfuraLog>),
    Null,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InfuraLog {
    pub address: String,
    pub block_hash: String,
    pub block_number: String,
    pub data: String,
    pub topics: Vec<String>,
    pub transaction_hash: String,
    pub transaction_index: String,
    // { "address": "0x8306300ffd616049fd7e4b0354a64da835c1a81c", "blockHash": "0xae7dd19381472fd2d97c18d8e4e4454c9859a2279e882e02f922f924e2fdc558", "blockNumber": "0x116cfd6", "data": "0x000000000000000000000000000000000000000000000000009778e5c5e0add5", "logIndex": "0x110", "removed": false,
    //   "topics": [ "0x3d0ce9bfc3ed7d6862dbb28b2dea94561fe714a1b4d019aa8af39730d1ad7c3d", "0x0000000000000000000000001f9090aae28b8a3dceadf281b0f12828e676c326" ],
    //   "transactionHash": "0x9f125fec2a4158e4d87ff7d07adb3048580f4a6a5dbad0cca646d880f9785e35", "transactionIndex": "0x79" }
}

impl InfuraLog {
    pub fn last_block_number(sql: &mut crate::sql::Client) -> u32 {
        let row = sql.q_last(crate::geth::InfuraLog::last_block_number_sql());
        match row {
            Some(row) => row.get::<&str, i32>("block_number") as u32,
            None => 10_000_000,
        }
    }
    pub fn last_block_number_sql() -> crate::sql::SqlQuery {
        <dyn crate::Ops>::last_column("logs", "block_number")
    }
}

impl crate::sql::Ops for InfuraLog {
    fn to_upsert_sql(&self) -> crate::sql::SqlQuery {
        <dyn crate::Ops>::upsert_sql(
            "logs",
            vec![],
            vec![
                "address",
                "block_hash",
                "block_number",
                "value",
                "topic0",
                "topic1",
                "topic2",
                "transaction_hash",
                "transaction_index",
            ],
            vec![
                Box::new(self.address.strip_prefix("0x").unwrap().to_owned()),
                Box::new(self.block_hash.strip_prefix("0x").unwrap().to_owned()),
                Box::new(
                    i32::from_str_radix(&self.block_number.strip_prefix("0x").unwrap(), 16)
                        .unwrap(),
                ),
                Box::new(
                    Decimal::from_str_radix(self.data.strip_prefix("0x").unwrap(), 16).unwrap(),
                ),
                Box::new(self.topics[0].strip_prefix("0x").unwrap().to_owned()),
                Box::new(self.topics[1].strip_prefix("0x").unwrap().to_owned()),
                Box::new(self.topics[2].strip_prefix("0x").unwrap().to_owned()),
                Box::new(self.transaction_hash.strip_prefix("0x").unwrap().to_owned()),
                Box::new(
                    i32::from_str_radix(&self.transaction_index.strip_prefix("0x").unwrap(), 16)
                        .unwrap(),
                ),
            ],
        )
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InfuraBlock {
    pub difficulty: String,
    pub hash: String,
    pub number: String,
    pub transactions: Vec<InfuraTransaction>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InfuraTransaction {
    pub transaction_index: String,
    pub hash: String,
    pub from: String,
    pub input: String,
    pub to: Option<String>,
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
