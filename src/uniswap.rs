pub mod v3 {
    const _UNISWAP_FACTORY: &str = "0x1f98431c8ad98523631ae4a59f267346ea31f984";
}

pub mod v2 {
    use crate::geth::Client;
    use ethabi::token::Token;
    use ethabi::Contract;
    use ethereum_types::{Address, U256};
    use std::sync::OnceLock;

    const UNISWAP_FACTORY: &str = "0x5c69bee701ef814a2b6a3edd4b1652cb9cc5aa6f";
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
            let address_hex = format!("0x{}", hex::encode(address));
            let result_t0 = geth.eth_call(address_hex.clone(), abi, "token0", &vec![])?;
            let Token::Address(addr_t0) = result_t0[0] else { println!("{:?}", result_t0[0]); unreachable!() };
            let result_t1 = geth.eth_call(address_hex.clone(), abi, "token1", &vec![])?;
            let Token::Address(addr_t1) = result_t1[0] else { println!("{:?}", result_t1[0]); unreachable!() };
            Ok((addr_t0, addr_t1))
        }

        pub fn reserves(
            geth: &Client,
            abi: &Contract,
            address: &Address,
        ) -> Result<(U256, U256), Box<dyn std::error::Error>> {
            let address_hex = format!("0x{}", hex::encode(address));
            let result = geth.eth_call(address_hex.clone(), abi, "getReserves", &vec![])?;
            let Token::Uint(r0) = result[0] else { println!("{:?}", result[0]); unreachable!() };
            let Token::Uint(r1) = result[1] else { unreachable!() };
            Ok((r0, r1))
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
        pub(crate) fn pool_count(geth: &Client) -> Result<U256, Box<dyn std::error::Error>> {
            let result = geth.eth_call(
                UNISWAP_FACTORY.to_string(),
                &ABI.get().unwrap(),
                "allPairsLength",
                &vec![],
            )?;
            let Token::Uint(count) = result[0] else { println!("{:?}", result[0]); unreachable!() };
            return Ok(count);
        }

        pub(crate) fn pool_addr(
            geth: &Client,
            pool_id: u64,
        ) -> Result<Address, Box<dyn std::error::Error>> {
            let result = geth.eth_call(
                UNISWAP_FACTORY.to_string(),
                &ABI.get().unwrap(),
                "allPairs",
                &vec![Token::Uint(pool_id.into())],
            )?;
            let Token::Address(addr) = result[0] else { unreachable!() };
            return Ok(addr);
        }
    }
}
