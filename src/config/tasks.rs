use serde::Deserialize;

use super::http;

#[derive(Deserialize)]
pub enum Task {
    Http(http::Task),
}
