use crate::config;
use postgres::{Client, NoTls};
use rust_decimal::Decimal;

pub(crate) fn init() {
    let config = config::CONFIG.get().unwrap();

    let mut client = Client::connect(&config.psql, NoTls).unwrap();
    log::info!("sql connected");
    embedded::migrations::runner().run(&mut client).unwrap();

    let q = sql_query_builder::Select::new()
        .select("*")
        .from("pools");
    log::info!("{}", q.to_string());
    let rows = client.query(&q.to_string(), &[]).unwrap();
    log::info!("rows: {:?}", rows);
    //log::info!("0.0: {:?}", rows[0].get::<usize, Decimal>(0));
}

mod embedded {
    use refinery::embed_migrations;
    embed_migrations!("./sql");
}
