pub mod v3 {
    const _UNISWAP_FACTORY: &str = "0x1f98431c8ad98523631ae4a59f267346ea31f984";
}

pub mod v2 {
    use crate::config;
    use crate::geth::Client;
    use ethabi::token::Token;
    use ethabi::Contract;
    use ethereum_types::{Address, U256};
    use sql_query_builder as sql;
    use std::str::FromStr;

    const UNISWAP_FACTORY: &str = "0x5c69bee701ef814a2b6a3edd4b1652cb9cc5aa6f";

    #[derive(Debug)]
    pub(crate) struct Pool {
        pub index: i32,
        pub address: Address,
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
            let address_hex = format!("0x{}", hex::encode(address));
            let result_t0 = geth.eth_call(address_hex.clone(), abi, "token0", &vec![])?;
            let result_t1 = geth.eth_call(address_hex.clone(), abi, "token1", &vec![])?;
            println!("token0 {}", &result_t0[26..]);
            Ok((
                Address::from_str(&result_t0[26..]).unwrap(),
                Address::from_str(&result_t1[26..]).unwrap(),
            ))
        }

        pub fn reserves(
            geth: &Client,
            abi: &Contract,
            address: &Address,
        ) -> Result<(U256, U256), Box<dyn std::error::Error>> {
            let address_hex = format!("0x{}", hex::encode(address));
            let result = geth.eth_call(address_hex.clone(), abi, "getReserves", &vec![])?;
            let t0 = U256::from_str_radix(&result[2..66], 16).unwrap();
            let t1 = U256::from_str_radix(&result[66..130], 16).unwrap();
            Ok((t0, t1))
        }
    }

    impl crate::sql::Ops for Reserves<'_> {
        fn to_sql(&self) -> crate::sql::SqlQuery {
            let select = sql::Insert::new()
                .insert_into("reserves (pool_index, block_number, x, y)")
                .values("($1, $2, $3, $4)")
                .on_conflict(
                    "(pool_index, block_number) DO UPDATE SET x = EXCLUDED.x, y = EXCLUDED.y;",
                );
            (
                select.as_string(),
                vec![
                    Box::new(self.pool.index),
                    Box::new(self.block_number as i32),
                    Box::new(format!("{}", self.x)),
                    Box::new(format!("{}", self.y)),
                ],
            )
        }
    }

    impl crate::sql::Ops for Pool {
        fn to_sql(&self) -> crate::sql::SqlQuery {
            let select = sql::Insert::new()
                .insert_into("pools")
                .values("($1, $2, $3, $4)")
                .on_conflict("(index) DO UPDATE SET contract_address = EXCLUDED.contract_address, token0 = EXCLUDED.token0, token1 = EXCLUDED.token1;");
            (
                select.as_string(),
                vec![
                    Box::new(self.index),
                    Box::new(format!("{:x}", self.address)),
                    Box::new(format!("{:x}", self.token0)),
                    Box::new(format!("{:x}", self.token1)),
                ],
            )
        }
    }

    pub(crate) struct Factory {
        abi: Contract,
    }

    impl Factory {
        pub(crate) fn new(abi: Contract) -> Self {
            let _config = config::CONFIG.get().unwrap();
            return Factory { abi: abi };
        }

        pub(crate) fn pool_count(&self, geth: &Client) -> Result<U256, Box<dyn std::error::Error>> {
            let result_str = geth.eth_call(
                UNISWAP_FACTORY.to_string(),
                &self.abi,
                "allPairsLength",
                &vec![],
            )?;
            return Ok(U256::from_str_radix(&result_str, 16)?);
        }

        pub(crate) fn pool_addr(
            &self,
            geth: &Client,
            pool_id: u64,
        ) -> Result<Address, Box<dyn std::error::Error>> {
            let result_str = geth.eth_call(
                UNISWAP_FACTORY.to_string(),
                &self.abi,
                "allPairs",
                &vec![Token::Uint(pool_id.into())],
            )?;
            let short_rs = &result_str[result_str.len() - 40..];
            return Ok(Address::from_str(short_rs).unwrap());
        }
    }
}
