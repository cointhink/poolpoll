use postgres::{Client, NoTls};
use crate::config;

pub(crate) fn init() {
    let config = config::CONFIG.get().unwrap();

    let mut client = Client::connect(&config.psql, NoTls).unwrap();
    log::info!("sql connected");
}
