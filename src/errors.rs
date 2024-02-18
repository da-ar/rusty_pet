use thiserror::Error;
use std::error::Error;

/// Comprehensive error types for the RustyPet CLI application
#[derive(Debug, Error)]
pub enum CliError {
    /// Network-related errors (connection timeouts, DNS failures, API unavailability)
    #[error("Network error: {message}")]
    Network { 
        message: String, 
        retry_possible: bool,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
    
    /// Authentication errors (token expiration, invalid credentials, authorization failures)
    #[error("Authentication failed: {message}")]
    Authentication { 
        message: String, 
        can_reauth: bool,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
    
    /// Input validation errors (invalid formats, missing parameters)
    #[error("Invalid input: {message}")]
    Validation { 
        message: String, 
        examples: Vec<String>,
        field: Option<String>,
    },
    
    /// Data processing errors (malformed API responses, missing fields)
    #[error("Data processing error: {message}")]
    Data { 
        message: String, 
        context: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
    
    /// System errors (file system issues, permission problems, resource constraints)
    #[error("System error: {message}")]
    System { 
        message: String, 
        suggestion: Option<String>,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
    
    /// Configuration errors (invalid config files, missing settings)
    #[error("Configuration error: {message}")]
    Configuration {
        message: String,
        config_path: Option<String>,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
    
    /// API-specific errors (rate limiting, service unavailable)
    #[error("API error: {message}")]
    Api {
        message: String,
        status_code: Option<u16>,
        retry_after: Option<u64>, // seconds
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
}

impl CliError {
    /// Get a user-friendly error message suitable for display
    pub fn user_message(&self) -> String {
        match self {
            CliError::Network { message, retry_possible, .. } => {
                if *retry_possible {
                    format!("Network connection issue: {}. You can try again.", message)
                } else {
                    format!("Network error: {}. Please check your internet connection.", message)
                }
            }
            CliError::Authentication { message, can_reauth, .. } => {
                if *can_reauth {
                    format!("Authentication expired: {}. Please login again.", message)
                } else {
                    format!("Authentication failed: {}. Please check your credentials.", message)
                }
            }
            CliError::Validation { message, examples, field } => {
                let mut msg = format!("Invalid input: {}", message);
                if let Some(field_name) = field {
                    msg.push_str(&format!(" (field: {})", field_name));
                }
                if !examples.is_empty() {
                    msg.push_str(&format!("\nValid examples: {}", examples.join(", ")));
                }
                msg
            }
            CliError::Data { message, context, .. } => {
                format!("Data error: {} (context: {})", message, context)
            }
            CliError::System { message, suggestion, .. } => {
                let mut msg = format!("System error: {}", message);
                if let Some(suggestion_text) = suggestion {
                    msg.push_str(&format!("\nSuggestion: {}", suggestion_text));
                }
                msg
            }
            CliError::Configuration { message, config_path, .. } => {
                let mut msg = format!("Configuration error: {}", message);
                if let Some(path) = config_path {
                    msg.push_str(&format!(" (config: {})", path));
                }
                msg
            }
            CliError::Api { message, status_code, retry_after, .. } => {
                let mut msg = format!("API error: {}", message);
                if let Some(code) = status_code {
                    msg.push_str(&format!(" (HTTP {})", code));
                }
                if let Some(retry_seconds) = retry_after {
                    msg.push_str(&format!("\nPlease wait {} seconds before retrying.", retry_seconds));
                }
                msg
            }
        }
    }

    /// Get detailed error information for logging purposes
    pub fn log_details(&self) -> String {
        let mut details = format!("Error: {}", self);
        
        // Add source chain if available
        let mut current_source = self.source();
        while let Some(source) = current_source {
            details.push_str(&format!("\nCaused by: {}", source));
            current_source = source.source();
        }
        
        // Add context-specific details
        match self {
            CliError::Network { retry_possible, .. } => {
                details.push_str(&format!("\nRetry possible: {}", retry_possible));
            }
            CliError::Authentication { can_reauth, .. } => {
                details.push_str(&format!("\nCan re-authenticate: {}", can_reauth));
            }
            CliError::Validation { examples, field, .. } => {
                if let Some(field_name) = field {
                    details.push_str(&format!("\nField: {}", field_name));
                }
                if !examples.is_empty() {
                    details.push_str(&format!("\nValid examples: {:?}", examples));
                }
            }
            CliError::Api { status_code, retry_after, .. } => {
                if let Some(code) = status_code {
                    details.push_str(&format!("\nHTTP Status: {}", code));
                }
                if let Some(retry_seconds) = retry_after {
                    details.push_str(&format!("\nRetry after: {} seconds", retry_seconds));
                }
            }
            _ => {}
        }
        
        details
    }

    /// Get recovery suggestions for the error
    pub fn recovery_suggestions(&self) -> Vec<String> {
        match self {
            CliError::Network { retry_possible, .. } => {
                let mut suggestions = vec![
                    "Check your internet connection".to_string(),
                    "Verify that the SurePet API is accessible".to_string(),
                ];
                if *retry_possible {
                    suggestions.push("Try the operation again".to_string());
                }
                suggestions
            }
            CliError::Authentication { can_reauth, .. } => {
                let mut suggestions = vec![];
                if *can_reauth {
                    suggestions.push("Run the command again to re-authenticate".to_string());
                    suggestions.push("Check if your credentials are still valid".to_string());
                } else {
                    suggestions.push("Verify your username and password".to_string());
                    suggestions.push("Check if your account is active".to_string());
                }
                suggestions
            }
            CliError::Validation { examples, .. } => {
                let mut suggestions = vec!["Check the input format".to_string()];
                if !examples.is_empty() {
                    suggestions.push(format!("Use one of these formats: {}", examples.join(", ")));
                }
                suggestions.push("Use --help to see usage information".to_string());
                suggestions
            }
            CliError::Data { .. } => {
                vec![
                    "Try refreshing the data".to_string(),
                    "Check if the API response format has changed".to_string(),
                    "Contact support if the issue persists".to_string(),
                ]
            }
            CliError::System { suggestion, .. } => {
                let mut suggestions = vec![];
                if let Some(suggestion_text) = suggestion {
                    suggestions.push(suggestion_text.clone());
                }
                suggestions.push("Check file permissions".to_string());
                suggestions.push("Ensure sufficient disk space".to_string());
                suggestions
            }
            CliError::Configuration { config_path, .. } => {
                let mut suggestions = vec!["Check the configuration file format".to_string()];
                if let Some(path) = config_path {
                    suggestions.push(format!("Verify the config file exists: {}", path));
                }
                suggestions.push("Reset configuration to defaults if needed".to_string());
                suggestions
            }
            CliError::Api { status_code, retry_after, .. } => {
                let mut suggestions = vec![];
                match status_code {
                    Some(429) => {
                        suggestions.push("You're being rate limited".to_string());
                        if let Some(retry_seconds) = retry_after {
                            suggestions.push(format!("Wait {} seconds before retrying", retry_seconds));
                        } else {
                            suggestions.push("Wait a few minutes before retrying".to_string());
                        }
                    }
                    Some(500..=599) => {
                        suggestions.push("The API service is experiencing issues".to_string());
                        suggestions.push("Try again later".to_string());
                    }
                    Some(401) => {
                        suggestions.push("Your authentication token may have expired".to_string());
                        suggestions.push("Try logging in again".to_string());
                    }
                    Some(403) => {
                        suggestions.push("You don't have permission for this operation".to_string());
                        suggestions.push("Check your account permissions".to_string());
                    }
                    Some(404) => {
                        suggestions.push("The requested resource was not found".to_string());
                        suggestions.push("Check if the pet/device ID is correct".to_string());
                    }
                    _ => {
                        suggestions.push("Check the API documentation".to_string());
                        suggestions.push("Contact support if the issue persists".to_string());
                    }
                }
                suggestions
            }
        }
    }



    /// Create a network error with retry capability
    pub fn network_error(message: impl Into<String>, retry_possible: bool) -> Self {
        CliError::Network {
            message: message.into(),
            retry_possible,
            source: None,
        }
    }

    /// Create a network error from a source error
    pub fn network_error_with_source(
        message: impl Into<String>, 
        retry_possible: bool, 
        source: Box<dyn std::error::Error + Send + Sync>
    ) -> Self {
        CliError::Network {
            message: message.into(),
            retry_possible,
            source: Some(source),
        }
    }

    /// Create an authentication error
    pub fn auth_error(message: impl Into<String>, can_reauth: bool) -> Self {
        CliError::Authentication {
            message: message.into(),
            can_reauth,
            source: None,
        }
    }

    /// Create an authentication error from a source error
    pub fn auth_error_with_source(
        message: impl Into<String>, 
        can_reauth: bool, 
        source: Box<dyn std::error::Error + Send + Sync>
    ) -> Self {
        CliError::Authentication {
            message: message.into(),
            can_reauth,
            source: Some(source),
        }
    }

    /// Create a validation error with examples
    pub fn validation_error(
        message: impl Into<String>, 
        examples: Vec<String>, 
        field: Option<String>
    ) -> Self {
        CliError::Validation {
            message: message.into(),
            examples,
            field,
        }
    }



    /// Create a data processing error from a source error
    pub fn data_error_with_source(
        message: impl Into<String>, 
        context: impl Into<String>, 
        source: Box<dyn std::error::Error + Send + Sync>
    ) -> Self {
        CliError::Data {
            message: message.into(),
            context: context.into(),
            source: Some(source),
        }
    }

    /// Create a system error with suggestion
    pub fn system_error(message: impl Into<String>, suggestion: Option<String>) -> Self {
        CliError::System {
            message: message.into(),
            suggestion,
            source: None,
        }
    }

    /// Create a system error from a source error
    pub fn system_error_with_source(
        message: impl Into<String>, 
        suggestion: Option<String>, 
        source: Box<dyn std::error::Error + Send + Sync>
    ) -> Self {
        CliError::System {
            message: message.into(),
            suggestion,
            source: Some(source),
        }
    }



    /// Create an API error with status code and retry information
    #[cfg(test)]
    pub fn api_error(
        message: impl Into<String>, 
        status_code: Option<u16>, 
        retry_after: Option<u64>
    ) -> Self {
        CliError::Api {
            message: message.into(),
            status_code,
            retry_after,
            source: None,
        }
    }

    /// Create an API error from a source error
    pub fn api_error_with_source(
        message: impl Into<String>, 
        status_code: Option<u16>, 
        retry_after: Option<u64>,
        source: Box<dyn std::error::Error + Send + Sync>
    ) -> Self {
        CliError::Api {
            message: message.into(),
            status_code,
            retry_after,
            source: Some(source),
        }
    }



    /// Add a suggestion to a system error (helper method)
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        match &mut self {
            CliError::System { suggestion: ref mut s, .. } => {
                *s = Some(suggestion.into());
            }
            _ => {} // Only applies to system errors
        }
        self
    }
}

/// Convert from reqwest::Error to CliError
impl From<reqwest::Error> for CliError {
    fn from(error: reqwest::Error) -> Self {
        let status_code = error.status().map(|s| s.as_u16());
        
        if error.is_timeout() {
            CliError::network_error_with_source(
                "Request timed out", 
                true, 
                Box::new(error)
            )
        } else if error.is_connect() {
            CliError::network_error_with_source(
                "Failed to connect to server", 
                true, 
                Box::new(error)
            )
        } else if let Some(status) = status_code {
            match status {
                401 => CliError::auth_error_with_source(
                    "Authentication failed", 
                    true, 
                    Box::new(error)
                ),
                403 => CliError::auth_error_with_source(
                    "Access forbidden", 
                    false, 
                    Box::new(error)
                ),
                429 => {
                    // Try to extract retry-after header if available
                    let retry_after = None; // Would need response headers to extract this
                    CliError::api_error_with_source(
                        "Rate limit exceeded", 
                        Some(status), 
                        retry_after, 
                        Box::new(error)
                    )
                }
                500..=599 => CliError::api_error_with_source(
                    "Server error", 
                    Some(status), 
                    Some(60), // Suggest waiting 1 minute for server errors
                    Box::new(error)
                ),
                _ => CliError::api_error_with_source(
                    "API request failed", 
                    Some(status), 
                    None, 
                    Box::new(error)
                ),
            }
        } else {
            CliError::network_error_with_source(
                "Network request failed", 
                true, 
                Box::new(error)
            )
        }
    }
}

/// Convert from std::io::Error to CliError
impl From<std::io::Error> for CliError {
    fn from(error: std::io::Error) -> Self {
        match error.kind() {
            std::io::ErrorKind::NotFound => CliError::system_error_with_source(
                "File or directory not found", 
                Some("Check if the path exists and you have permission to access it".to_string()), 
                Box::new(error)
            ),
            std::io::ErrorKind::PermissionDenied => CliError::system_error_with_source(
                "Permission denied", 
                Some("Check file permissions or run with appropriate privileges".to_string()), 
                Box::new(error)
            ),
            std::io::ErrorKind::ConnectionRefused => CliError::network_error_with_source(
                "Connection refused", 
                true, 
                Box::new(error)
            ),
            std::io::ErrorKind::TimedOut => CliError::network_error_with_source(
                "Operation timed out", 
                true, 
                Box::new(error)
            ),
            _ => CliError::system_error_with_source(
                "System I/O error", 
                None, 
                Box::new(error)
            ),
        }
    }
}

/// Convert from serde_json::Error to CliError
impl From<serde_json::Error> for CliError {
    fn from(error: serde_json::Error) -> Self {
        CliError::data_error_with_source(
            "JSON parsing failed", 
            "Invalid or malformed JSON data", 
            Box::new(error)
        )
    }
}

/// Convert from toml::de::Error to CliError
impl From<toml::de::Error> for CliError {
    fn from(error: toml::de::Error) -> Self {
        config_error_with_source(
            "TOML configuration parsing failed", 
            None, 
            Box::new(error)
        )
    }
}



/// Helper function to create a configuration error with source
pub fn config_error_with_source(
    message: impl Into<String>, 
    config_path: Option<String>, 
    source: Box<dyn std::error::Error + Send + Sync>
) -> CliError {
    CliError::Configuration {
        message: message.into(),
        config_path,
        source: Some(source),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_error_creation() {
        let error = CliError::network_error("Connection failed", true);
        assert!(matches!(error, CliError::Network { retry_possible: true, .. }));
        
        let user_msg = error.user_message();
        assert!(user_msg.contains("You can try again"));
    }

    #[test]
    fn test_auth_error_creation() {
        let error = CliError::auth_error("Token expired", true);
        assert!(matches!(error, CliError::Authentication { can_reauth: true, .. }));
        
        let user_msg = error.user_message();
        assert!(user_msg.contains("Please login again"));
    }

    #[test]
    fn test_validation_error_with_examples() {
        let examples = vec!["example1".to_string(), "example2".to_string()];
        let error = CliError::validation_error("Invalid format", examples.clone(), Some("field1".to_string()));
        
        let user_msg = error.user_message();
        assert!(user_msg.contains("field: field1"));
        assert!(user_msg.contains("example1, example2"));
    }

    #[test]
    fn test_recovery_suggestions() {
        let error = CliError::network_error("Connection failed", true);
        let suggestions = error.recovery_suggestions();
        
        assert!(!suggestions.is_empty());
        assert!(suggestions.iter().any(|s| s.contains("internet connection")));
        assert!(suggestions.iter().any(|s| s.contains("Try the operation again")));
    }

    #[test]
    fn test_reqwest_error_conversion() {
        // This would require creating a mock reqwest error, which is complex
        // In a real implementation, you'd test this with integration tests
    }

    #[test]
    fn test_io_error_conversion() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
        let cli_error: CliError = io_error.into();
        
        assert!(matches!(cli_error, CliError::System { .. }));
        
        let suggestions = cli_error.recovery_suggestions();
        assert!(suggestions.iter().any(|s| s.contains("permission")));
    }
}