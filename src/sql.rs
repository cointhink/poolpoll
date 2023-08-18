use crate::config;
use sql_query_builder as sql;
//use rust_decimal::Decimal;
use postgres::types::ToSql;

pub type SqlQuery = (String, Vec<Box<dyn ToSql + Sync>>);

pub trait Ops {
    fn to_upsert_sql(&self) -> SqlQuery;
}

impl dyn Ops {
    pub fn upsert_sql(
        table_name: &str,
        column_index_names: Vec<&str>,
        column_other_names: Vec<&str>,
        column_values: Vec<Box<dyn ToSql + Sync>>,
    ) -> SqlQuery {
        let all_names = vec![column_index_names.clone(), column_other_names.clone()]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        let value_positions = (1..all_names.len() + 1)
            .map(|x| format!("${}", x))
            .collect::<Vec<_>>()
            .join(", ");
        let conflict = format!(
            "({}) DO UPDATE SET {}",
            column_index_names.join(", "),
            column_other_names
                .into_iter()
                .map(|x| format!("{} = EXCLUDED.{}", x, x))
                .collect::<Vec<_>>()
                .join(", ")
        );
        let select = sql::Insert::new()
            .insert_into(&format!("{} ({})", table_name, all_names.join(", ")))
            .values(&format!("({})", value_positions))
            .on_conflict(&conflict);
        (select.as_string(), column_values)
        // let select = sql::Insert::new()
        //     .insert_into("reserves (pool_index, block_number, x, y)")
        //     .values("($1, $2, $3, $4)")
        //     .on_conflict(
        //         "(pool_index, block_number) DO UPDATE SET x = EXCLUDED.x, y = EXCLUDED.y;",
        //     );
        // (
        //     select.as_string(),
        //     vec![
        //         Box::new(self.pool.index),
        //         Box::new(self.block_number as i32),
        //         Box::new(format!("{}", self.x)),
        //         Box::new(format!("{}", self.y)),
        //     ],
        // )
    }
}

pub struct Client {
    pub client: postgres::Client,
}

pub(crate) fn new() -> Client {
    let config = config::CONFIG.get().unwrap();

    let mut client = postgres::Client::connect(&config.psql, postgres::NoTls).unwrap();

    log::info!("sql connected");
    embedded::migrations::runner().run(&mut client).unwrap();
    return Client { client };
}

impl Client {
    pub fn q(&mut self, query: SqlQuery) -> Vec<postgres::Row> {
        let params: Vec<&(dyn ToSql + Sync)> = query.1.iter().map(|y| &**y).collect();
        self.client.query(&query.0, &params).unwrap()
    }

    pub fn insert(&mut self, query: SqlQuery) {
        log::info!("sql: {} {:?}", query.0, query.1);
        // expected `&[&dyn ToSql + Sync]`, found `&Vec<Box<dyn ToSql + Sync>>`
        // self.client.execute(&query.0, &query.1).unwrap();

        // convert element type from String to ToSql+Sync
        let params: Vec<&(dyn ToSql + Sync)> = query.1.iter().map(|y| &**y).collect();

        //  params: &[&(dyn ToSql + Sync)]
        self.client.execute(&query.0, &params).unwrap();
    }
}

mod embedded {
    use refinery::embed_migrations;
    embed_migrations!("./sql");
}
