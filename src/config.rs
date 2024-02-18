use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub(crate) api: Api,
}

#[derive(Deserialize, Debug)]
pub struct Api {
    pub(crate) surepy_url: String,
}

pub fn read_config() -> Config {
    let config_file: &str = include_str!("./assets/client_config.toml");
    return toml::from_str(&config_file).unwrap();
}
