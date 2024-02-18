use crate::api::client::Client;
use crate::errors::CliError;
use crate::token;
use log::{debug, warn, error};
use std::env;
use std::time::Duration;
use tokio::time::sleep;

pub const TOKEN_ENV: &str = "SUREHUB_TOKEN";
const MAX_AUTH_RETRIES: u32 = 3;
const RETRY_DELAY_SECONDS: u64 = 2;

/// Authentication manager that handles token validation, retry logic, and re-authentication
pub struct AuthManager {
    client: Client,
    max_retries: u32,
    retry_delay: Duration,
}

impl AuthManager {
    /// Create a new authentication manager
    pub fn new(client: Client) -> Self {
        Self {
            client,
            max_retries: MAX_AUTH_RETRIES,
            retry_delay: Duration::from_secs(RETRY_DELAY_SECONDS),
        }
    }

    /// Get a valid authentication token with automatic retry and re-authentication
    pub async fn get_valid_token(&self, interactive: bool) -> Result<String, CliError> {
        // First, try to get an existing token
        match self.get_existing_token().await {
            Ok(token) => {
                // Validate the token by making a test API call
                match self.validate_token(&token).await {
                    Ok(_) => {
                        debug!("Existing token is valid");
                        return Ok(token);
                    }
                    Err(e) => {
                        warn!("Existing token validation failed: {}", e);
                        if interactive {
                            match &e {
                                CliError::Authentication { .. } => {
                                    println!("🔄 Your saved authentication token has expired.");
                                }
                                _ => {
                                    debug!("Token validation failed due to: {}", e.log_details());
                                }
                            }
                        }
                        // Token is invalid, clear it and try to re-authenticate
                        self.clear_invalid_token().await?;
                    }
                }
            }
            Err(e) => {
                debug!("No existing token found: {}", e);
                if interactive {
                    println!("🔐 No saved authentication token found. Please login to continue.");
                }
            }
        }

        // No valid token found, attempt authentication with retry
        self.authenticate_with_retry(interactive).await
    }

    /// Attempt authentication with automatic retry logic
    pub async fn authenticate_with_retry(&self, interactive: bool) -> Result<String, CliError> {
        let mut last_error = CliError::auth_error("Authentication failed", true);
        
        for attempt in 1..=self.max_retries {
            debug!("Authentication attempt {} of {}", attempt, self.max_retries);
            
            // Provide user feedback for interactive mode
            if interactive && attempt > 1 {
                println!("🔄 Attempting authentication (try {} of {})...", attempt, self.max_retries);
            }
            
            match self.attempt_authentication(interactive).await {
                Ok(token) => {
                    debug!("Authentication successful on attempt {}", attempt);
                    if interactive && attempt > 1 {
                        println!("✅ Authentication successful!");
                    }
                    return Ok(token);
                }
                Err(e) => {
                    error!("Authentication attempt {} failed: {}", attempt, e.log_details());
                    last_error = e;
                    
                    // Check if this is a retryable error
                    if !self.is_retryable_auth_error(&last_error) {
                        debug!("Error is not retryable, stopping attempts");
                        if interactive {
                            println!("❌ Authentication failed: {}", last_error.user_message());
                            self.show_auth_recovery_suggestions(&last_error);
                        }
                        break;
                    }
                    
                    // If not the last attempt, wait before retrying
                    if attempt < self.max_retries {
                        debug!("Waiting {:?} before retry", self.retry_delay);
                        if interactive {
                            println!("⏳ Authentication failed. Retrying in {} seconds... (attempt {} of {})", 
                                   self.retry_delay.as_secs(), attempt, self.max_retries);
                            
                            // Show specific error message for this attempt
                            match &last_error {
                                CliError::Authentication { message, .. } => {
                                    println!("   Reason: {}", message);
                                }
                                CliError::Network { message, .. } => {
                                    println!("   Network issue: {}", message);
                                }
                                CliError::Api { message, status_code, .. } => {
                                    if let Some(code) = status_code {
                                        println!("   API error ({}): {}", code, message);
                                    } else {
                                        println!("   API error: {}", message);
                                    }
                                }
                                _ => {
                                    println!("   Error: {}", last_error.user_message());
                                }
                            }
                        }
                        sleep(self.retry_delay).await;
                    } else if interactive {
                        println!("❌ All authentication attempts failed.");
                        self.show_auth_recovery_suggestions(&last_error);
                    }
                }
            }
        }
        
        // All attempts failed
        error!("All authentication attempts failed");
        Err(last_error)
    }

    /// Validate a token by making a test API call
    pub async fn validate_token(&self, token: &str) -> Result<(), CliError> {
        debug!("Validating authentication token");
        
        // Make a lightweight API call to validate the token
        // We'll use the pets endpoint as it's a simple call that requires authentication
        match self.client.get_pets(token).await {
            Ok(_) => {
                debug!("Token validation successful");
                Ok(())
            }
            Err(e) => {
                debug!("Token validation failed: {}", e);
                // Convert reqwest error to CliError with proper context
                let cli_error: CliError = e.into();
                match &cli_error {
                    CliError::Authentication { .. } => Err(cli_error),
                    CliError::Api { status_code: Some(401), .. } => {
                        Err(CliError::auth_error("Authentication token has expired or is invalid", true))
                    }
                    CliError::Api { status_code: Some(403), .. } => {
                        Err(CliError::auth_error("Authentication token does not have required permissions", false))
                    }
                    CliError::Network { .. } => {
                        // For network errors during validation, we'll assume the token might be valid
                        // but we can't verify it right now
                        debug!("Network error during token validation, assuming token is valid");
                        Ok(())
                    }
                    _ => Err(cli_error), // Other errors
                }
            }
        }
    }

    /// Re-authenticate with an existing token (refresh scenario)
    pub async fn reauthenticate(&self, interactive: bool) -> Result<String, CliError> {
        debug!("Re-authenticating due to token expiration");
        
        // Clear the existing invalid token
        self.clear_invalid_token().await?;
        
        // Attempt new authentication
        self.authenticate_with_retry(interactive).await
    }

    /// Get an existing token from environment or file
    async fn get_existing_token(&self) -> Result<String, CliError> {
        // Priority 1: Check environment variable
        if let Ok(env_token) = env::var(TOKEN_ENV) {
            debug!("{} found in environment", TOKEN_ENV);
            return Ok(env_token);
        }

        // Priority 2: Check saved token file
        match token::load_token() {
            Ok(saved_token) => {
                debug!("Found saved token file");
                // Set it in environment for this session
                env::set_var(TOKEN_ENV, &saved_token);
                Ok(saved_token)
            }
            Err(e) => {
                debug!("No saved token found: {}", e);
                Err(CliError::auth_error("No authentication token found", true))
            }
        }
    }

    /// Attempt a single authentication
    async fn attempt_authentication(&self, interactive: bool) -> Result<String, CliError> {
        if !interactive {
            return Err(CliError::auth_error(
                "Cannot authenticate in non-interactive mode", 
                false
            ).with_suggestion("Please set SUREHUB_TOKEN environment variable or run in interactive mode"));
        }

        // Check if we're in a terminal environment
        if !atty::is(atty::Stream::Stdin) {
            return Err(CliError::auth_error(
                "Cannot prompt for credentials in non-interactive environment", 
                false
            ).with_suggestion("Please set SUREHUB_TOKEN environment variable"));
        }

        debug!("Prompting for login credentials");
        
        // Enhanced user prompts with better guidance
        println!("🔐 Please enter your SurePet account credentials:");
        
        let username: String = cliclack::input("Email address")
            .placeholder("your.email@example.com")
            .interact()
            .map_err(|e| CliError::system_error_with_source(
                "Failed to read username input", 
                Some("Try running the command again".to_string()), 
                Box::new(e)
            ))?;
            
        let password = cliclack::password("Password")
            .mask('▪')
            .interact()
            .map_err(|e| CliError::system_error_with_source(
                "Failed to read password input", 
                Some("Try running the command again".to_string()), 
                Box::new(e)
            ))?;

        debug!("Attempting login with provided credentials");
        println!("🔄 Authenticating with SurePet...");
        
        let resp = self.client
            .login(&username, &password)
            .await
            .map_err(|e| {
                // Convert reqwest error to CliError with better context
                match e.status() {
                    Some(status) if status.as_u16() == 401 => {
                        CliError::auth_error_with_source(
                            "Invalid email address or password", 
                            true, 
                            Box::new(e)
                        )
                    }
                    Some(status) if status.as_u16() == 429 => {
                        CliError::api_error_with_source(
                            "Too many login attempts. Please wait before trying again", 
                            Some(status.as_u16()), 
                            Some(300), // Suggest waiting 5 minutes for rate limiting
                            Box::new(e)
                        )
                    }
                    Some(status) if status.as_u16() >= 500 => {
                        CliError::api_error_with_source(
                            "SurePet service is temporarily unavailable", 
                            Some(status.as_u16()), 
                            Some(60), // Suggest waiting 1 minute
                            Box::new(e)
                        )
                    }
                    Some(status) if status.as_u16() == 403 => {
                        CliError::auth_error_with_source(
                            "Account access is restricted. Please check your account status", 
                            false, 
                            Box::new(e)
                        )
                    }
                    _ => {
                        // Check if it's a network-related error
                        if e.is_timeout() {
                            CliError::network_error_with_source(
                                "Login request timed out. Please check your internet connection", 
                                true, 
                                Box::new(e)
                            )
                        } else if e.is_connect() {
                            CliError::network_error_with_source(
                                "Cannot connect to SurePet servers. Please check your internet connection", 
                                true, 
                                Box::new(e)
                            )
                        } else {
                            CliError::from(e)
                        }
                    }
                }
            })?;

        let new_token = resp.data.token;
        debug!("Login successful, saving token");

        // Save the new token to file with enhanced feedback
        match token::save_token(&new_token) {
            Ok(_) => {
                println!("✅ Authentication successful! Token saved for future use.");
            }
            Err(e) => {
                warn!("Failed to save token to file: {}", e);
                // Continue anyway, just warn the user with better messaging
                println!("⚠️  Authentication successful, but couldn't save token to file.");
                println!("   You'll need to login again next time you run the application.");
                println!("   Error details: {}", e);
                
                // Provide helpful suggestions
                println!("\n💡 To fix this issue:");
                println!("   • Check that your home directory is writable");
                println!("   • Ensure you have sufficient disk space");
                println!("   • Consider setting the SUREHUB_TOKEN environment variable instead");
            }
        }

        // Set the token in the environment for use in same session
        env::set_var(TOKEN_ENV, &new_token);
        debug!("Token set in environment");

        Ok(new_token)
    }

    /// Clear an invalid token from environment and file
    async fn clear_invalid_token(&self) -> Result<(), CliError> {
        debug!("Clearing invalid authentication token");
        
        // Remove from environment
        env::remove_var(TOKEN_ENV);
        
        // Remove from file
        if let Err(e) = token::delete_token() {
            warn!("Failed to delete invalid token file: {}", e);
            // Don't fail the operation, just log the warning
        }
        
        Ok(())
    }

    /// Show authentication recovery suggestions to the user
    fn show_auth_recovery_suggestions(&self, error: &CliError) {
        let suggestions = error.recovery_suggestions();
        if !suggestions.is_empty() {
            println!("\n💡 Troubleshooting suggestions:");
            for (i, suggestion) in suggestions.iter().enumerate() {
                println!("   {}. {}", i + 1, suggestion);
            }
        }
        
        // Add specific suggestions for authentication failures
        match error {
            CliError::Authentication { .. } => {
                println!("\n🔧 Additional help:");
                println!("   • Make sure your SurePet account is active");
                println!("   • Check if you can log in through the SurePet mobile app");
                println!("   • Verify your internet connection is stable");
            }
            CliError::Network { .. } => {
                println!("\n🔧 Network troubleshooting:");
                println!("   • Check your internet connection");
                println!("   • Try again in a few moments");
                println!("   • Verify that app.api.surehub.io is accessible");
            }
            CliError::Api { status_code: Some(429), .. } => {
                println!("\n⏰ Rate limiting detected:");
                println!("   • You've made too many login attempts");
                println!("   • Wait 5-10 minutes before trying again");
                println!("   • Consider using the SUREHUB_TOKEN environment variable");
            }
            _ => {}
        }
    }
    fn is_retryable_auth_error(&self, error: &CliError) -> bool {
        match error {
            CliError::Authentication { can_reauth, .. } => *can_reauth,
            CliError::Network { retry_possible, .. } => *retry_possible,
            CliError::Api { status_code, .. } => {
                match status_code {
                    Some(429) => false, // Rate limiting - don't retry immediately
                    Some(401) => true,  // Unauthorized - can retry with new credentials
                    Some(403) => false, // Forbidden - permissions issue, don't retry
                    Some(500..=599) => true, // Server errors - can retry
                    _ => false,
                }
            }
            CliError::System { .. } => false, // System errors usually aren't retryable
            _ => false,
        }
    }

    /// Perform an authenticated API operation with automatic re-authentication on token expiry
    #[allow(dead_code)] // Future functionality
    pub async fn with_auth<F, T>(&self, interactive: bool, operation: F) -> Result<T, CliError>
    where
        F: Fn(&str) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, CliError>> + Send + '_>>,
    {
        // Get a valid token
        let token = self.get_valid_token(interactive).await?;
        
        // Try the operation
        match operation(&token).await {
            Ok(result) => Ok(result),
            Err(CliError::Authentication { can_reauth: true, .. }) |
            Err(CliError::Api { status_code: Some(401), .. }) => {
                // Token expired during operation, try to re-authenticate once
                debug!("Token expired during operation, attempting re-authentication");
                
                if interactive {
                    println!("🔄 Your session has expired. Re-authenticating...");
                }
                
                match self.reauthenticate(interactive).await {
                    Ok(new_token) => {
                        if interactive {
                            println!("✅ Re-authentication successful! Retrying your request...");
                        }
                        
                        // Retry the operation with the new token
                        operation(&new_token).await
                    }
                    Err(reauth_error) => {
                        if interactive {
                            println!("❌ Re-authentication failed: {}", reauth_error.user_message());
                            self.show_auth_recovery_suggestions(&reauth_error);
                        }
                        Err(reauth_error)
                    }
                }
            }
            Err(e) => Err(e),
        }
    }

    /// Get enhanced error message for headless mode failures
    pub fn get_headless_auth_guidance(&self, error: &CliError) -> String {
        let mut guidance = String::new();
        
        guidance.push_str("Authentication failed in headless mode.\n\n");
        guidance.push_str("To use RustyPet in headless mode, you need to provide authentication:\n\n");
        guidance.push_str("Option 1 - Environment Variable (Recommended):\n");
        guidance.push_str("  export SUREHUB_TOKEN=\"your_token_here\"\n");
        guidance.push_str("  rusty_pet list\n\n");
        guidance.push_str("Option 2 - Interactive Login First:\n");
        guidance.push_str("  rusty_pet  # Run without arguments to login interactively\n");
        guidance.push_str("  rusty_pet list  # Then use headless commands\n\n");
        
        match error {
            CliError::Authentication { .. } => {
                guidance.push_str("The issue: No valid authentication token was found.\n");
                guidance.push_str("This happens when you haven't logged in yet or your token has expired.\n");
            }
            CliError::Network { .. } => {
                guidance.push_str("The issue: Network connectivity problems.\n");
                guidance.push_str("Please check your internet connection and try again.\n");
            }
            _ => {
                guidance.push_str(&format!("The issue: {}\n", error.user_message()));
            }
        }
        
        guidance
    }
}



#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    fn create_test_client() -> Client {
        // This would normally use a test configuration
        // For now, we'll use a minimal config for testing
        let config = Config {
            api: crate::config::Api {
                surehub_url: "https://app.api.surehub.io".to_string(),
            },
        };
        Client::new(config)
    }

    #[test]
    fn test_auth_manager_creation() {
        let client = create_test_client();
        let auth_manager = AuthManager::new(client);
        
        assert_eq!(auth_manager.max_retries, MAX_AUTH_RETRIES);
        assert_eq!(auth_manager.retry_delay, Duration::from_secs(RETRY_DELAY_SECONDS));
    }

    #[test]
    fn test_is_retryable_auth_error() {
        let client = create_test_client();
        let auth_manager = AuthManager::new(client);
        
        // Test retryable errors
        let retryable_auth = CliError::auth_error("Token expired", true);
        assert!(auth_manager.is_retryable_auth_error(&retryable_auth));
        
        let retryable_network = CliError::network_error("Connection failed", true);
        assert!(auth_manager.is_retryable_auth_error(&retryable_network));
        
        let retryable_api = CliError::api_error("Unauthorized", Some(401), None);
        assert!(auth_manager.is_retryable_auth_error(&retryable_api));
        
        // Test non-retryable errors
        let non_retryable_auth = CliError::auth_error("Invalid credentials", false);
        assert!(!auth_manager.is_retryable_auth_error(&non_retryable_auth));
        
        let rate_limit = CliError::api_error("Rate limited", Some(429), Some(300));
        assert!(!auth_manager.is_retryable_auth_error(&rate_limit));
        
        let forbidden = CliError::api_error("Forbidden", Some(403), None);
        assert!(!auth_manager.is_retryable_auth_error(&forbidden));
    }

    // Note: Integration tests for actual authentication would require
    // a test server or mocking framework, which is beyond the scope
    // of this unit test suite.
}