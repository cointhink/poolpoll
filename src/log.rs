pub use log::info;
use log4rs;

pub fn init() {
    log4rs::init_file("log4rs.yaml", Default::default()).unwrap();
}
