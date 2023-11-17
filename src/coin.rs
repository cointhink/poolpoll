use ethereum_types::Address;
use ethereum_types::U256;

use crate::sql::SqlQuery;
use crate::uniswap::v2::AddressStringNox;
use sql_query_builder as sql;

#[derive(Debug)]
pub struct Coin {
    pub contract_address: Address,
    pub name: String,
    pub symbol: String,
    pub decimals: u32,
}

impl Coin {
    pub fn find_by_contract_address(contract_address: AddressStringNox) -> SqlQuery {
        let select = sql::Select::new()
            .select("*")
            .from("coins")
            .where_clause("contract_address = $1");
        (select.to_string(), vec![Box::new(contract_address)])
    }
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
                Box::new(self.decimals as i32),
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
        let decimals = row.get::<&str, i32>("decimals") as u32;
        Coin {
            contract_address,
            name,
            symbol,
            decimals,
        }
    }
}
