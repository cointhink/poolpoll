use bs58;
use ethabi::token::Token;
use ethabi::Contract;
use ethereum_types::Address;
use postgres::types::ToSql;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, time::Duration};

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
        let tx = tx_build(to_hex.clone(), function_input);
        let params = (tx, infura_block_param(block_number));
        let output = self.rpc_str("eth_call", ParamTypes::Infura(params))?;
        let output_no_0x = output.strip_prefix("0x").unwrap();
        let output_bytes = hex::decode(output_no_0x).unwrap();
        match function_call.decode_output(&output_bytes) {
            Err(err) => Err(format!(
                "geth call {}({:?})@{} => Error decoding output {:?} {}",
                function_name, function_params, to_hex, err, output_no_0x
            )
            .into()),
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

    pub fn block(&self, block_number: u32) -> Result<InfuraBlock, Box<dyn std::error::Error>> {
        let infura_block_number = infura_block_param(Some(block_number));
        let params = (infura_block_number, true);
        match self
            .rpc("eth_getBlockByNumber", ParamTypes::EthBlockByHash(params))?
            .part
        {
            RpcResultTypes::Result(result) => {
                if let ResultTypes::Block(block) = result.result {
                    Ok(block)
                } else {
                    Err(Box::from(format!(
                        "geth.block: Unexpected result type {:?}",
                        result
                    )))
                }
            }
            RpcResultTypes::Error(err) => {
                Err(Box::from(format!("geth.block: RPC Error {:?}", err)))
            }
        }
    }

    pub fn logs(&self, block_number: u32) -> Result<Vec<InfuraLog>, Box<dyn std::error::Error>> {
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
                    Ok(logs)
                } else {
                    panic!("Unexpected result type from eth_getLogs {:?}", r)
                }
            }
            RpcResultTypes::Error(e) => return Err(Box::from(e.error.message)),
        }
    }

    pub fn rpc(
        &self,
        method: &str,
        params: ParamTypes,
    ) -> Result<JsonRpcResult, Box<dyn std::error::Error>> {
        let params_str = format!("{:?}", params);
        let jrpc = JsonRpc {
            jsonrpc: "2.0".to_string(),
            id: gen_id(),
            method: method.to_string(),
            params: params,
        };
        let result = ureq::post(&self.url)
            .timeout(Duration::new(12, 0))
            .send_json(&jrpc);
        match result {
            Ok(res) => {
                log::info!(target: "http", "{} {} {} {}", self.url, method, params_str, res.status_text() );
                let rpc_result = res.into_json::<JsonRpcResult>().unwrap();
                Ok(rpc_result)
            }
            Err(e) => {
                log::info!(target: "http", "{} {} {} {}", self.url, method, params_str, e);
                Err(Box::new(e))
            }
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

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct InfuraLog {
    pub address: String,
    pub block_hash: String,
    pub block_number: String,
    pub data: String,
    pub topics: Vec<String>,
    pub transaction_hash: String,
    #[serde(deserialize_with = "hexstr_to_u32")]
    pub transaction_index: u32,
    // { "address": "0x8306300ffd616049fd7e4b0354a64da835c1a81c", "blockHash": "0xae7dd19381472fd2d97c18d8e4e4454c9859a2279e882e02f922f924e2fdc558",
    //   "blockNumber": "0x116cfd6", "data": "0x000000000000000000000000000000000000000000000000009778e5c5e0add5", "logIndex": "0x110", "removed": false,
    //   "topics": [ "0x3d0ce9bfc3ed7d6862dbb28b2dea94561fe714a1b4d019aa8af39730d1ad7c3d", "0x0000000000000000000000001f9090aae28b8a3dceadf281b0f12828e676c326" ],
    //   "transactionHash": "0x9f125fec2a4158e4d87ff7d07adb3048580f4a6a5dbad0cca646d880f9785e35", "transactionIndex": "0x79" }
}

impl crate::sql::Ops for InfuraLog {
    fn to_upsert_sql(&self) -> crate::sql::SqlQuery {
        let mut fields = vec![
            "address",
            "block_hash",
            "block_number",
            "data",
            "transaction_hash",
            "transaction_index",
        ];
        let mut topic_fields = vec![];
        for topic_num in 0..self.topics.len() {
            topic_fields.push(format!("topic{}", topic_num))
        }
        fields.append(&mut topic_fields.iter().map(|f| f.as_str()).collect());

        let mut values: Vec<Box<dyn ToSql + Sync>> = vec![
            Box::new(self.address.strip_prefix("0x").unwrap().to_owned()),
            Box::new(self.block_hash.strip_prefix("0x").unwrap().to_owned()),
            Box::new(
                i32::from_str_radix(&self.block_number.strip_prefix("0x").unwrap(), 16).unwrap(),
            ),
            Box::new(self.data.strip_prefix("0x").unwrap().to_owned()),
            Box::new(self.transaction_hash.strip_prefix("0x").unwrap().to_owned()),
            Box::new(self.transaction_index as i32),
        ];
        for topic_num in 0..self.topics.len() {
            values.push(Box::new(
                self.topics[topic_num]
                    .strip_prefix("0x")
                    .unwrap()
                    .to_owned(),
            ));
        }
        <dyn crate::Ops>::upsert_sql("logs", vec![], fields, values)
    }
}

fn hexstr_to_u32<'de, D>(str: D) -> Result<u32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let json_str = String::deserialize(str)?;
    let num = u32::from_str_radix(json_str.strip_prefix("0x").unwrap(), 16).unwrap();
    Ok(num)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InfuraBlock {
    pub hash: String,
    #[serde(deserialize_with = "hexstr_to_u32")]
    pub number: u32,
    #[serde(deserialize_with = "hexstr_to_u32")]
    pub timestamp: u32,
    pub transactions: Vec<InfuraTransaction>,
}

impl InfuraBlock {
    pub fn last_db_block_number(db: &mut crate::sql::Client, descend: bool) -> Option<u32> {
        let sql = Self::last_block_number_sql(descend);
        match db.q_last(sql) {
            Some(row) => Some(row.get::<&str, i32>("number") as u32),
            None => None,
        }
    }
    fn last_block_number_sql(descend: bool) -> crate::sql::SqlQuery {
        <dyn crate::Ops>::last_column("blocks", "number", descend)
    }
}

impl crate::sql::Ops for InfuraBlock {
    fn to_upsert_sql(&self) -> crate::sql::SqlQuery {
        <dyn crate::Ops>::upsert_sql(
            "blocks",
            vec!["number"],
            vec!["hash", "timestamp"],
            vec![
                Box::new(self.number as i32),
                Box::new(self.hash.strip_prefix("0x").unwrap().to_owned()),
                Box::new(self.timestamp as i32),
            ],
        )
    }
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
