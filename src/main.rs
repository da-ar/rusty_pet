mod api;
mod config;

use crate::api::client::Client;
use console::style;
use env_logger::{Builder, Target};
use log::{debug, error};
use std::env;

const TOKEN_ENV: &str = "SUREPY_TOKEN";

pub struct Context {
    pub config: config::Config,
    pub token: String,
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let mut log_builder = Builder::from_default_env();
    log_builder.target(Target::Stdout);
    log_builder.init();

    let mut ctx: Context = Context {
        config: config::read_config(),
        token: "".to_string(),
    };

    ctrlc::set_handler(move || {}).expect("setting Ctrl-C handler");
    cliclack::clear_screen()?;

    cliclack::intro(style(" RustyPet - Your SurePet CLI ").on_cyan().black())?;

    let op = cliclack::select(format!("What would you like to do?"))
        .initial_value("st")
        .item("st", "Status", "")
        .item("ls", "List Pets", "")
        .interact()?;

    // Sign in etc
    let api_client = Client::new(&ctx);

    let token = check_token(&mut ctx, &api_client).await;
    if token.is_ok() == false {
        error!(
            "failed to authenticate to SurePy: {}",
            &token.as_ref().err().unwrap()
        )
    }

    match op {
        "st" => do_status(&mut ctx, &api_client).await,
        "ls" => do_list(&mut ctx, &api_client).await,
        _ => {
            println!("This is an invalid operation");
            error!("Invalid operation")
        }
    }

    Ok(())
}

async fn do_list(ctx: &mut Context, api_client: &Client<'_>) {
    debug!("Performing list operation");
}

async fn do_status(ctx: &mut Context, api_client: &Client<'_>) {
    debug!("Performing status operation");
}

async fn check_token(ctx: &mut Context, api_client: &Client<'_>) -> std::io::Result<()> {
    // check if authentication token has been set in environment
    if env::var(TOKEN_ENV).is_ok() {
        debug!("{} found", TOKEN_ENV);
        println!("using token {}", env::var(TOKEN_ENV).unwrap());

        // Set the token in the context for future use
        ctx.token = env::var(TOKEN_ENV).unwrap();

        return Ok(());
    } else if !ctx.token.is_empty() {
        return Ok(());
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

        // Set the token in the context for future use
        ctx.token = resp.data.token;

        return Ok(());
    }
}
