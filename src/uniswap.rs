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
        pub x: u128,
        pub y: u128,
    }

    impl Pool {
        pub fn reserves(
            geth: &Client,
            abi: &Contract,
            address: &Address,
        ) -> Result<(String, String), Box<dyn std::error::Error>> {
            log::info!("reserves address {:?}", hex::encode(address));
            let result_t0 = geth.eth_call(hex::encode(address), abi, "token0", &vec![])?;
            let result_t1 = geth.eth_call(hex::encode(address), abi, "token0", &vec![])?;
            Ok((result_t0, result_t1))
        }
    }

    impl crate::sql::Ops for Pool {
        fn to_sql(&self) -> crate::sql::SqlQuery {
            let select = sql::Insert::new()
                .insert_into("pools")
                .values("($1, $2, $3, $4)")
                .on_conflict("(index) DO UPDATE SET address = EXCLUDED.address, x = EXCLUDED.x, y = EXCLUDED.y;");
            (
                select.as_string(),
                vec![
                    Box::new(self.index),
                    Box::new(format!("{:x}", self.address)),
                    Box::new(self.x.to_string()),
                    Box::new(self.y.to_string()),
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
