use ethereum_types::Address;

#[derive(Debug)]
pub struct Coin {
    pub contract_address: Address,
    pub name: String,
    pub symbol: String,
}

impl crate::sql::Ops for Coin {
    fn to_upsert_sql(&self) -> crate::sql::SqlQuery {
        <dyn crate::Ops>::upsert_sql(
            "coins",
            vec!["contract_address"],
            vec!["name", "symbol"],
            vec![
                Box::new(format!("{:x}", self.contract_address)),
                Box::new(self.name.to_owned()),
                Box::new(self.symbol.to_owned()),
            ],
        )
    }
}
