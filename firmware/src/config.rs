use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub(crate) struct Wifi<'a> {
    pub ssid: &'a str,
    pub password: &'a str,
}
