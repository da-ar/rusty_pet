use log::{debug, error};
use std::fs;
use std::path::PathBuf;

const TOKEN_FILE: &str = ".rusty_pet_token";

/// Get the path to the token file in the user's home directory
fn get_token_file_path() -> Result<PathBuf, std::io::Error> {
    let home_dir = dirs::home_dir()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "Home directory not found"))?;
    Ok(home_dir.join(TOKEN_FILE))
}

/// Save the authentication token to a file
pub fn save_token(token: &str) -> Result<(), std::io::Error> {
    let token_path = get_token_file_path()?;
    debug!("Saving token to: {:?}", token_path);
    
    fs::write(&token_path, token)?;
    debug!("Token saved successfully");
    Ok(())
}

/// Load the authentication token from a file
pub fn load_token() -> Result<String, std::io::Error> {
    let token_path = get_token_file_path()?;
    debug!("Loading token from: {:?}", token_path);
    
    match fs::read_to_string(&token_path) {
        Ok(token) => {
            debug!("Token loaded successfully");
            Ok(token.trim().to_string())
        }
        Err(e) => {
            debug!("Failed to load token: {}", e);
            Err(e)
        }
    }
}

/// Delete the saved token file
pub fn delete_token() -> Result<(), std::io::Error> {
    let token_path = get_token_file_path()?;
    debug!("Deleting token file: {:?}", token_path);
    
    match fs::remove_file(&token_path) {
        Ok(_) => {
            debug!("Token file deleted successfully");
            Ok(())
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            debug!("Token file doesn't exist, nothing to delete");
            Ok(())
        }
        Err(e) => {
            error!("Failed to delete token file: {}", e);
            Err(e)
        }
    }
}