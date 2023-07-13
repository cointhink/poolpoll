use postgres::{Client, NoTls};
use crate::config;

pub(crate) fn init() {
    let config = config::CONFIG.get().unwrap();

    let mut client = Client::connect(&config.psql, NoTls).unwrap();
    log::info!("sql connected");
    embedded::migrations::runner().run(&mut client).unwrap();
}

mod embedded {
    use refinery::embed_migrations;
    embed_migrations!("./sql");
}