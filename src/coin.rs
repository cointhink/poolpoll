use ethereum_types::Address;
use ethereum_types::U256;

#[derive(Debug)]
pub struct Coin {
    pub contract_address: Address,
    pub name: String,
    pub symbol: String,
    pub decimals: U256,
}

impl crate::sql::Ops for Coin {
    fn to_upsert_sql(&self) -> crate::sql::SqlQuery {
        <dyn crate::Ops>::upsert_sql(
            "coins",
            vec!["contract_address"],
            vec!["name", "symbol", "decimals"],
            vec![
                Box::new(format!("{:x}", self.contract_address)),
                Box::new(self.name.to_owned()),
                Box::new(self.symbol.to_owned()),
                Box::new(self.decimals.low_u32() as i32),
            ],
        )
    }
}

impl From<&postgres::Row> for Coin {
    fn from(row: &postgres::Row) -> Self {
        let contract_address =
            Address::from_slice(&hex::decode::<String>(row.get("contract_address")).unwrap());
        let name = row.get::<&str, String>("name");
        let symbol = row.get::<&str, String>("symbol");
        let decimals = Into::<U256>::into(row.get::<&str, i32>("decimals"));
        Coin {
            contract_address,
            name,
            symbol,
            decimals,
        }
    }
}
