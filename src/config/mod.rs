pub mod http;
pub mod source;
pub mod tasks;
pub mod value;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Config {
    tasks: Vec<tasks::Task>,
}

impl Config {}
