pub mod v3 {
    const _UNISWAP_FACTORY: &str = "0x1f98431c8ad98523631ae4a59f267346ea31f984";
}

pub mod v2 {
    use crate::{geth::Client, sql::SqlQuery};
    use ethabi::token::Token;
    use ethabi::Contract;
    use ethereum_types::{Address, U256};
    use sql_query_builder as sql;
    use std::sync::OnceLock;

    const UNISWAP_FACTORY: &str = "5c69bee701ef814a2b6a3edd4b1652cb9cc5aa6f";
    pub static ABI: OnceLock<Contract> = OnceLock::new();

    #[derive(Debug)]
    pub(crate) struct Pool {
        pub uniswap_v2_index: i32,
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

        pub fn find_by_uniswap_v2_index(uniswap_v2_index: i32) -> SqlQuery {
            let select = sql::Select::new()
                .select("*")
                .from("pools")
                .where_clause("uniswap_v2_index = $1");
            (select.to_string(), vec![Box::new(uniswap_v2_index)])
        }

        pub fn find_by_contract_address(contract_address: String) -> SqlQuery {
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

    impl From<&postgres::Row> for Pool {
        fn from(row: &postgres::Row) -> Self {
            Pool {
                uniswap_v2_index: row.get("uniswap_v2_index"),
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
                vec!["uniswap_v2_index", "token0", "token1"],
                vec![
                    Box::new(format!("{:x}", self.contract_address)),
                    Box::new(self.uniswap_v2_index),
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

        pub(crate) fn sql_pool_count(sql: &mut crate::sql::Client) -> u64 {
            let max_pool = sql_query_builder::Select::new()
                .select("max(uniswap_v2_index)")
                .from("pools");
            let sql_pool_count_rows = sql.q((max_pool.to_string(), vec![]));
            match sql_pool_count_rows[0].try_get::<&str, i32>("max") {
                Ok(count) => count as u64,
                Err(_) => 0,
            }
        }

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
}
