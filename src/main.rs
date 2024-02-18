mod api;
mod config;

use crate::api::client::Client;
use console::style;
use env_logger::{Builder, Target};
use log::{debug, error};
use std::env;

const TOKEN_ENV: &str = "SUREPY_TOKEN";

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let mut builder = Builder::from_default_env();
    builder.target(Target::Stdout);
    builder.init();

    let cfg: config::Config = config::read_config();

    ctrlc::set_handler(move || {}).expect("setting Ctrl-C handler");
    cliclack::clear_screen()?;

    cliclack::intro(style(" RustyPet - Your SurePet CLI ").on_cyan().black())?;

    let op = cliclack::select(format!("What would you like to do?"))
        .initial_value("st")
        .item("st", "Status", "")
        .item("ls", "List Pets", "")
        .interact()?;

    // Sign in etc
    let api_client = Client::new(cfg);

    let token = check_token(&api_client).await;
    if token.is_ok() == false {
        error!(
            "failed to authenticate to SurePy: {}",
            &token.as_ref().err().unwrap()
        )
    }

    match op {
        "st" => do_status(&api_client, &token.unwrap()).await,
        "ls" => do_list(&api_client, &token.unwrap()).await,
        _ => {
            println!("This is an invalid operation");
            error!("Invalid operation")
        }
    }

    Ok(())
}

async fn do_list(api_client: &Client, token: &String) {
    debug!("Performing list operation");
}

async fn do_status(api_client: &Client, token: &String) {
    debug!("Performing status operation");
}

async fn check_token(api_client: &Client) -> std::io::Result<String> {
    // check if authentication token has been set in environment
    if env::var(TOKEN_ENV).is_ok() {
        debug!("{} found", TOKEN_ENV);
        println!("using token {}", env::var(TOKEN_ENV).unwrap());
        return Ok(env::var(TOKEN_ENV).unwrap());
    } else {
        // if no token, sign in with username and password then return the token
        debug!("{} not found", TOKEN_ENV);
        let username: String = cliclack::input("Provide your username").interact()?;

        let password = cliclack::password("Provide your password")
            .mask('▪')
            .interact()?;

        let resp = api_client
            .login(&username, &password)
            .await
            .expect("Failed to log in");

        // Set the token in the environment for use in same session
        env::set_var(TOKEN_ENV, &resp.data.token);
        debug!("Token ENV set");

        return Ok(resp.data.token);
    }
}
