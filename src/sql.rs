use crate::config;
//use rust_decimal::Decimal;
use postgres::types::ToSql;

pub type SqlQuery = (String, Vec<String>);

pub trait Ops {
    fn to_sql(&self) -> SqlQuery;
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
    pub fn q(&mut self) {
        let q = sql_query_builder::Select::new().select("*").from("pools");
        log::info!("{}", q.to_string());
        let rows = self.client.query(&q.to_string(), &[]).unwrap();
        log::info!("rows: {:?}", rows);
        log::info!("0.0: {:?}", rows[0].get::<&str, String>("address"));
    }
    pub fn insert(&mut self, query: SqlQuery) {
        //self.client .execute(&query.0, &query.1).unwrap();
    }
}

mod embedded {
    use refinery::embed_migrations;
    embed_migrations!("./sql");
}
