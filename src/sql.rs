use crate::config;
use rust_decimal::Decimal;
use std::sync::OnceLock;

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
    fn q(&mut self) {
        let q = sql_query_builder::Select::new().select("*").from("pools");
        log::info!("{}", q.to_string());
        let rows = self.client.query(&q.to_string(), &[]).unwrap();
        log::info!("rows: {:?}", rows);
        log::info!("0.0: {:?}", rows[0].get::<&str, String>("address"));
    }
}

mod embedded {
    use refinery::embed_migrations;
    embed_migrations!("./sql");
}
