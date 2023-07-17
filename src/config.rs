use serde::{Deserialize, Serialize};
use std::{fs::File, io::Read};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GatewayConfig {
    pub route: Vec<Route>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Route {
    pub scheme: String,
    pub authority: Authority,
    pub path: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Authority {
    pub host: String,
    pub port: String,
}

pub fn load_config(path: &str) -> GatewayConfig {
    let mut contents: String = String::new();
    let mut file: File = File::open(path).expect("Failed to open the configuration file!");
    file.read_to_string(&mut contents)
        .expect("Failed to read the configuration file!");
    serde_yaml::from_str(&contents).expect("Failed to parse the configuration file!")
}

pub fn get_route<'a>(path: &str, route: &'a [Route]) -> Option<&'a Route> {
    route.iter().find(|c: &&Route| path.starts_with(&c.path))
}
