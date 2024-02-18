mod api;
mod auth;
mod cache;
mod cli;
mod config;
mod data_processor;
mod data_validation;
mod errors;
mod export;
mod formatters;
mod headless;
mod interactive;
mod offline_manager;
mod offline_queue;
mod search;
mod token;

use crate::api::client::Client;
use crate::auth::AuthManager;
use crate::cache::CacheManager;
use crate::cli::{CliArgs, CliMode};
use crate::errors::CliError;
use crate::headless::HeadlessMode;
use crate::interactive::InteractiveMode;
use crate::offline_manager::OfflineManager;
use env_logger::{Builder, Target};
use log::{debug, error, warn};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), CliError> {
    // Parse command-line arguments first
    let args = CliArgs::parse_args();
    
    // Set up logging based on verbosity
    let mut builder = Builder::from_default_env();
    builder.target(Target::Stdout);
    if args.is_verbose() {
        builder.filter_level(log::LevelFilter::Debug);
    }
    builder.init();

    debug!("Starting RustyPet CLI");
    debug!("CLI mode: {:?}", args.get_mode());
    debug!("JSON output: {}", args.is_json_output());

    let cfg: config::Config = config::read_config();

    // Create a single API client instance for the entire session
    // This enables connection reuse and efficient resource management
    let api_client = Client::new(cfg);

    // Determine mode and execute accordingly
    match args.get_mode() {
        CliMode::Interactive => {
            run_interactive_mode(api_client, &args).await
        }
        CliMode::Headless => {
            run_headless_mode(api_client, args).await
        }
    }
}

async fn run_interactive_mode(api_client: Client, args: &CliArgs) -> Result<(), CliError> {
    debug!("Running in interactive mode");
    
    // Initialize cache manager
    let cache_manager = match CacheManager::default() {
        Ok(cache) => Arc::new(cache),
        Err(e) => {
            warn!("Failed to initialize cache manager: {}", e);
            // Continue without cache - not critical for operation
            return run_interactive_mode_without_cache(api_client, args).await;
        }
    };
    
    // Initialize offline manager
    let offline_manager = match OfflineManager::new(Arc::new(api_client.clone())) {
        Ok(manager) => Arc::new(manager),
        Err(e) => {
            warn!("Failed to initialize offline manager: {}", e);
            // Continue without offline support - not critical for operation
            return run_interactive_mode_without_offline(api_client, args, cache_manager).await;
        }
    };
    
    // Create authentication manager
    let auth_manager = AuthManager::new(api_client.clone());
    
    // Get a valid token with automatic retry and re-authentication
    let mut token = match auth_manager.get_valid_token(true).await {
        Ok(t) => t,
        Err(e) => {
            error!("Failed to authenticate: {}", e.log_details());
            println!("❌ Authentication failed: {}", e.user_message());
            return Err(e);
        }
    };
    
    let interactive = InteractiveMode::new_with_managers(api_client, args, cache_manager, offline_manager);
    interactive.run(&mut token).await.map_err(|e| e.into())
}

async fn run_interactive_mode_without_cache(api_client: Client, args: &CliArgs) -> Result<(), CliError> {
    debug!("Running in interactive mode without cache");
    
    // Create authentication manager
    let auth_manager = AuthManager::new(api_client.clone());
    
    // Get a valid token with automatic retry and re-authentication
    let mut token = match auth_manager.get_valid_token(true).await {
        Ok(t) => t,
        Err(e) => {
            error!("Failed to authenticate: {}", e.log_details());
            println!("❌ Authentication failed: {}", e.user_message());
            return Err(e);
        }
    };
    
    let interactive = InteractiveMode::new(api_client, args);
    interactive.run(&mut token).await.map_err(|e| e.into())
}

async fn run_interactive_mode_without_offline(api_client: Client, args: &CliArgs, cache_manager: Arc<CacheManager>) -> Result<(), CliError> {
    debug!("Running in interactive mode without offline support");
    
    // Create authentication manager
    let auth_manager = AuthManager::new(api_client.clone());
    
    // Get a valid token with automatic retry and re-authentication
    let mut token = match auth_manager.get_valid_token(true).await {
        Ok(t) => t,
        Err(e) => {
            error!("Failed to authenticate: {}", e.log_details());
            println!("❌ Authentication failed: {}", e.user_message());
            return Err(e);
        }
    };
    
    let interactive = InteractiveMode::new_with_cache(api_client, args, cache_manager);
    interactive.run(&mut token).await.map_err(|e| e.into())
}

async fn run_headless_mode(api_client: Client, args: CliArgs) -> Result<(), CliError> {
    debug!("Running in headless mode");
    
    // Initialize cache manager (optional for headless mode)
    let cache_manager = CacheManager::default().ok().map(Arc::new);
    
    // Initialize offline manager (optional for headless mode)
    let offline_manager = OfflineManager::new(Arc::new(api_client.clone())).ok().map(Arc::new);
    
    // Create authentication manager
    let auth_manager = AuthManager::new(api_client.clone());
    
    // Try to get a valid token (non-interactive for headless mode)
    let token = match auth_manager.get_valid_token(false).await {
        Ok(t) => t,
        Err(e) => {
            error!("Failed to authenticate: {}", e.log_details());
            
            let formatter = crate::formatters::create_formatter(args.is_json_output());
            
            if args.is_json_output() {
                // For JSON output, just show the error
                let error_output = formatter.format_error(&e.user_message());
                print!("{}", error_output);
            } else {
                // For human output, show enhanced guidance
                let guidance = auth_manager.get_headless_auth_guidance(&e);
                println!("{}", guidance);
            }
            
            return Err(e);
        }
    };
    
    let headless = match (cache_manager.as_ref(), offline_manager.as_ref()) {
        (Some(cache), Some(offline)) => {
            HeadlessMode::new_with_managers(api_client, args, Arc::clone(cache), Arc::clone(offline))
        }
        (Some(cache), None) => {
            HeadlessMode::new_with_cache(api_client, args, Arc::clone(cache))
        }
        _ => {
            HeadlessMode::new(api_client, args)
        }
    };
    
    headless.execute(&token).await.map_err(|e| e.into())
}