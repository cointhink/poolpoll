pub mod v3 {
    const _UNISWAP_FACTORY: &str = "0x1f98431c8ad98523631ae4a59f267346ea31f984";
}

pub mod v2 {
    use crate::geth::InfuraLog;
    use crate::{geth::Client, sql::SqlQuery};
    use ethabi::token::Token;
    use ethabi::Contract;
    use ethereum_types::{Address, U256};
    use num_traits::Num;
    use pg_bigdecimal::{BigInt, PgNumeric};
    use postgres::types::private::BytesMut;
    use postgres::types::{IsNull, Type};
    use sql_query_builder as sql;
    use std::error::Error;
    use std::sync::OnceLock;

    const UNISWAP_FACTORY: &str = "5c69bee701ef814a2b6a3edd4b1652cb9cc5aa6f";
    pub static ABI: OnceLock<Contract> = OnceLock::new();
    // Swap(address,uint256,uint256,uint256,uint256,address) 	0xd78ad95f
    pub const TOPIC_SWAP: &str =
        "0xd78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822";
    pub const TOPIC_SYNC: &str =
        "0x1c411e9a96e071241c2f21f7726b17ae89e3cab4c78be50e062b03a9fffbbad1";

    #[derive(Debug)]
    pub struct AddressStringNox(pub String);

    impl From<&str> for AddressStringNox {
        fn from(value: &str) -> Self {
            AddressStringNox(value.strip_prefix("0x").unwrap().to_owned())
        }
    }

    impl From<&Address> for AddressStringNox {
        fn from(value: &Address) -> Self {
            AddressStringNox(format!("{:x}", value))
        }
    }

    impl postgres::types::ToSql for AddressStringNox {
        fn to_sql(
            &self,
            ty: &Type,
            out: &mut BytesMut,
        ) -> Result<IsNull, Box<dyn Error + Sync + Send>>
        where
            Self: Sized,
        {
            self.0.to_sql(ty, out)
        }

        fn accepts(ty: &Type) -> bool
        where
            Self: Sized,
        {
            <String as postgres::types::ToSql>::accepts(ty)
        }

        fn to_sql_checked(
            &self,
            ty: &Type,
            out: &mut BytesMut,
        ) -> Result<IsNull, Box<dyn Error + Sync + Send>> {
            self.0.to_sql_checked(ty, out)
        }
    }

    #[derive(Debug)]
    pub(crate) struct Pool {
        pub contract_address: Address,
        pub token0: Address,
        pub token1: Address,
    }

    #[derive(Debug)]
    pub(crate) struct Reserves<'a> {
        pub pool: &'a Pool,
        pub block_number: u128,
        pub x: U256,
        pub y: U256,
    }

    #[derive(Debug)]
    pub(crate) struct Swap<'a> {
        pub pool: &'a Pool,
        pub block_number: u128,
        pub transaction_index: u32,
        pub in0_eth: BigInt,
        pub in1_eth: BigInt,
        pub call_params: SwapCall,
    }

    #[derive(Debug)]
    pub(crate) struct SwapCall {
        pub in0: BigInt,
        pub in1: BigInt,
        pub out0: BigInt,
        pub out1: BigInt,
    }

    impl From<&InfuraLog> for SwapCall {
        fn from(value: &InfuraLog) -> Self {
            let in0 = BigInt::from_str_radix(&value.data[2..66], 16).unwrap();
            let in1 = BigInt::from_str_radix(&value.data[66..130], 16).unwrap();
            let out0 = BigInt::from_str_radix(&value.data[130..194], 16).unwrap();
            let out1 = BigInt::from_str_radix(&value.data[194..258], 16).unwrap();
            SwapCall {
                in0,
                in1,
                out0,
                out1,
            }
        }
    }

    impl Pool {
        pub fn tokens(
            geth: &Client,
            abi: &Contract,
            address: &Address,
        ) -> Result<(Address, Address), Box<dyn std::error::Error>> {
            let result_t0 = geth.eth_call(address, abi, "token0", &vec![], None)?;
            let Token::Address(addr_t0) = result_t0[0] else {
                println!("{:?}", result_t0[0]);
                unreachable!()
            };
            let result_t1 = geth.eth_call(address, abi, "token1", &vec![], None)?;
            let Token::Address(addr_t1) = result_t1[0] else {
                println!("{:?}", result_t1[0]);
                unreachable!()
            };
            Ok((addr_t0, addr_t1))
        }

        pub fn reserves(
            geth: &Client,
            abi: &Contract,
            address: &Address,
            eth_block: u32,
        ) -> Result<(U256, U256), Box<dyn std::error::Error>> {
            let result = geth.eth_call(address, abi, "getReserves", &vec![], Some(eth_block))?;
            let Token::Uint(r0) = result[0] else {
                println!("{:?}", result[0]);
                unreachable!()
            };
            let Token::Uint(r1) = result[1] else {
                unreachable!()
            };
            Ok((r0, r1))
        }

        pub fn all() -> SqlQuery {
            let select = sql::Select::new().select("*").from("pools");
            (select.to_string(), vec![])
        }

        pub fn find_by_contract_address(contract_address: AddressStringNox) -> SqlQuery {
            let select = sql::Select::new()
                .select("*")
                .from("pools")
                .where_clause("contract_address = $1");
            (select.to_string(), vec![Box::new(contract_address)])
        }
    }

    impl<'a> Reserves<'a> {
        pub fn new(pool: &'a Pool, eth_block: u32, reserves: (U256, U256)) -> Self {
            Reserves {
                pool,
                block_number: eth_block as u128,
                x: reserves.0,
                y: reserves.1,
            }
        }

        pub fn find_by_pool(pool: &Pool) -> SqlQuery {
            let select = sql::Select::new()
                .select("*")
                .from("reserves")
                .where_clause("contract_address = $1")
                .order_by("block_number desc")
                .limit("1");

            (
                select.to_string(),
                vec![Box::new(format!("{:x}", pool.contract_address))],
            )
        }

        pub fn from_row(row: &postgres::Row, pool: &'a Pool) -> Self {
            Reserves {
                pool: pool,
                x: U256::from_str_radix(row.get::<_, _>("x"), 10).unwrap(),
                y: U256::from_str_radix(row.get::<_, _>("y"), 10).unwrap(),
                block_number: row.get::<_, i32>("block_number") as u128,
            }
        }
    }

    impl crate::sql::Ops for Reserves<'_> {
        fn to_upsert_sql(&self) -> crate::sql::SqlQuery {
            <dyn crate::Ops>::upsert_sql(
                "reserves",
                vec!["contract_address", "block_number"],
                vec!["x", "y"],
                vec![
                    Box::new(format!("{:x}", self.pool.contract_address)),
                    Box::new(self.block_number as i32),
                    Box::new(format!("{}", self.x)),
                    Box::new(format!("{}", self.y)),
                ],
            )
        }
    }

    impl crate::sql::Ops for Swap<'_> {
        fn to_upsert_sql(&self) -> crate::sql::SqlQuery {
            <dyn crate::Ops>::upsert_sql(
                "swaps",
                vec![],
                vec![
                    "pool_contract_address",
                    "block_number",
                    "transaction_index",
                    "in0",
                    "in0_eth",
                    "in1",
                    "in1_eth",
                    "out0",
                    "out1",
                ],
                vec![
                    Box::new(format!("{:x}", self.pool.contract_address)),
                    Box::new(self.block_number as i32),
                    Box::new(self.transaction_index as i32),
                    Box::new(PgNumeric::new(Some(self.call_params.in0.clone().into()))),
                    Box::new(PgNumeric::new(Some(self.in0_eth.clone().into()))),
                    Box::new(PgNumeric::new(Some(self.call_params.in1.clone().into()))),
                    Box::new(PgNumeric::new(Some(self.in1_eth.clone().into()))),
                    Box::new(PgNumeric::new(Some(self.call_params.out0.clone().into()))),
                    Box::new(PgNumeric::new(Some(self.call_params.out1.clone().into()))),
                ],
            )
        }
    }

    impl From<&postgres::Row> for Pool {
        fn from(row: &postgres::Row) -> Self {
            Pool {
                contract_address: Address::from_slice(
                    &hex::decode(row.get::<_, String>("contract_address")).unwrap(),
                ),
                token0: Address::from_slice(&hex::decode(row.get::<_, String>("token0")).unwrap()),
                token1: Address::from_slice(&hex::decode(row.get::<_, String>("token1")).unwrap()),
            }
        }
    }

    impl crate::sql::Ops for Pool {
        fn to_upsert_sql(&self) -> crate::sql::SqlQuery {
            <dyn crate::Ops>::upsert_sql(
                "pools",
                vec!["contract_address"],
                vec!["token0", "token1"],
                vec![
                    Box::new(format!("{:x}", self.contract_address)),
                    Box::new(format!("{:x}", self.token0)),
                    Box::new(format!("{:x}", self.token1)),
                ],
            )
        }
    }

    pub(crate) struct Factory {}

    impl Factory {
        pub(crate) fn setup() {
            let abi_file = std::fs::File::open("abi/uniswap_v2_factory.json").unwrap();
            let abi_factory = ethabi::Contract::load(abi_file).unwrap();
            ABI.set(abi_factory).unwrap();
        }

        pub(crate) fn pool_count(geth: &Client) -> Result<U256, Box<dyn std::error::Error>> {
            let factory = Address::from_slice(&hex::decode(UNISWAP_FACTORY).unwrap());
            let result = geth.eth_call(
                &factory,
                &ABI.get().unwrap(),
                "allPairsLength",
                &vec![],
                None,
            )?;
            let Token::Uint(count) = result[0] else {
                println!("{:?}", result[0]);
                unreachable!()
            };
            return Ok(count);
        }

        // pub(crate) fn sql_pool_count(sql: &mut crate::sql::Client) -> u64 {
        //     let max_pool = sql_query_builder::Select::new()
        //         .select("max(uniswap_v2_index)")
        //         .from("pools");
        //     let sql_pool_count_rows = sql.q((max_pool.to_string(), vec![]));
        //     match sql_pool_count_rows[0].try_get::<&str, i32>("max") {
        //         Ok(count) => count as u64,
        //         Err(_) => 0,
        //     }
        // }

        pub(crate) fn pool_addr(
            geth: &Client,
            pool_id: u64,
        ) -> Result<Address, Box<dyn std::error::Error>> {
            let factory = Address::from_slice(&hex::decode(UNISWAP_FACTORY).unwrap());
            let result = geth.eth_call(
                &factory,
                &ABI.get().unwrap(),
                "allPairs",
                &vec![Token::Uint(pool_id.into())],
                None,
            )?;
            let Token::Address(addr) = result[0] else {
                unreachable!()
            };
            return Ok(addr);
        }
    }

    #[cfg(test)]
    mod tests {}
}
