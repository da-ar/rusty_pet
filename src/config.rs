use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use crate::errors::CliError;
use chrono::{DateTime, Utc};
use log::debug;

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub(crate) api: Api,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Api {
    pub(crate) surehub_url: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserPreferences {
    pub date_format: String,
    pub time_format: String,
    pub timezone: String,
    pub default_date_range: DateRange,
    pub cache_ttl_hours: u64,
    pub auto_refresh_interval: Option<u64>,
    pub show_raw_json: bool,
    pub use_colors: bool,
    pub compact_mode: bool,
    pub max_history_items: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum DateRange {
    Today,
    Week,
    Month,
    Custom { start: DateTime<Utc>, end: DateTime<Utc> },
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            date_format: "%Y-%m-%d".to_string(),
            time_format: "%H:%M:%S".to_string(),
            timezone: "UTC".to_string(),
            default_date_range: DateRange::Week,
            cache_ttl_hours: 24,
            auto_refresh_interval: Some(30),
            show_raw_json: false,
            use_colors: true,
            compact_mode: false,
            max_history_items: 100,
        }
    }
}

impl UserPreferences {
    /// Get the path to the user preferences file
    fn get_preferences_path() -> Result<PathBuf, CliError> {
        let home_dir = dirs::home_dir()
            .ok_or_else(|| CliError::system_error(
                "Unable to determine home directory",
                Some("Please ensure your home directory is properly set".to_string())
            ))?;
        
        Ok(home_dir.join(".rusty_pet_preferences.toml"))
    }

    /// Load user preferences from file, or return defaults if file doesn't exist
    pub fn load() -> Result<Self, CliError> {
        let prefs_path = Self::get_preferences_path()?;
        
        if !prefs_path.exists() {
            debug!("Preferences file not found, using defaults");
            return Ok(Self::default());
        }

        let content = fs::read_to_string(&prefs_path)
            .map_err(|e| CliError::system_error_with_source(
                "Failed to read preferences file",
                Some(format!("Check permissions for {}", prefs_path.display())),
                Box::new(e)
            ))?;

        let preferences: UserPreferences = toml::from_str(&content)
            .map_err(|_e| CliError::validation_error(
                "Invalid preferences file format".to_string(),
                vec!["Try resetting preferences with --reset-config".to_string()],
                None
            ))?;

        debug!("Loaded user preferences from {}", prefs_path.display());
        Ok(preferences)
    }

    /// Save user preferences to file
    pub fn save(&self) -> Result<(), CliError> {
        let prefs_path = Self::get_preferences_path()?;
        
        // Create parent directory if it doesn't exist
        if let Some(parent) = prefs_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| CliError::system_error_with_source(
                    "Failed to create preferences directory",
                    Some("Check directory permissions".to_string()),
                    Box::new(e)
                ))?;
        }

        let content = toml::to_string_pretty(self)
            .map_err(|e| CliError::system_error_with_source(
                "Failed to serialize preferences",
                None,
                Box::new(e)
            ))?;

        fs::write(&prefs_path, content)
            .map_err(|e| CliError::system_error_with_source(
                "Failed to write preferences file",
                Some(format!("Check permissions for {}", prefs_path.display())),
                Box::new(e)
            ))?;

        debug!("Saved user preferences to {}", prefs_path.display());
        Ok(())
    }

    /// Validate preferences and return feedback
    pub fn validate(&self) -> Result<Vec<String>, CliError> {
        let mut feedback = Vec::new();

        // Validate date format
        match chrono::format::strftime::StrftimeItems::new(&self.date_format).next() {
            Some(_) => feedback.push("Date format is valid".to_string()),
            None => {
                return Err(CliError::validation_error(
                    "Invalid date format".to_string(),
                    vec!["%Y-%m-%d".to_string(), "%m/%d/%Y".to_string(), "%d.%m.%Y".to_string()],
                    Some("date_format".to_string())
                ));
            }
        }

        // Validate time format
        match chrono::format::strftime::StrftimeItems::new(&self.time_format).next() {
            Some(_) => feedback.push("Time format is valid".to_string()),
            None => {
                return Err(CliError::validation_error(
                    "Invalid time format".to_string(),
                    vec!["%H:%M:%S".to_string(), "%I:%M %p".to_string(), "%H:%M".to_string()],
                    Some("time_format".to_string())
                ));
            }
        }

        // Validate timezone
        if let Err(_) = self.timezone.parse::<chrono_tz::Tz>() {
            return Err(CliError::validation_error(
                "Invalid timezone".to_string(),
                vec!["UTC".to_string(), "America/New_York".to_string(), "Europe/London".to_string()],
                Some("timezone".to_string())
            ));
        } else {
            feedback.push("Timezone is valid".to_string());
        }

        // Validate cache TTL
        if self.cache_ttl_hours == 0 {
            return Err(CliError::validation_error(
                "Cache TTL must be greater than 0".to_string(),
                vec!["24".to_string(), "48".to_string(), "168".to_string()],
                Some("cache_ttl_hours".to_string())
            ));
        } else {
            feedback.push(format!("Cache TTL set to {} hours", self.cache_ttl_hours));
        }

        // Validate auto refresh interval
        if let Some(interval) = self.auto_refresh_interval {
            if interval == 0 {
                return Err(CliError::validation_error(
                    "Auto refresh interval must be greater than 0 seconds".to_string(),
                    vec!["30".to_string(), "60".to_string(), "300".to_string()],
                    Some("auto_refresh_interval".to_string())
                ));
            } else {
                feedback.push(format!("Auto refresh interval set to {} seconds", interval));
            }
        } else {
            feedback.push("Auto refresh disabled".to_string());
        }

        // Validate max history items
        if self.max_history_items == 0 {
            return Err(CliError::validation_error(
                "Max history items must be greater than 0".to_string(),
                vec!["50".to_string(), "100".to_string(), "500".to_string()],
                Some("max_history_items".to_string())
            ));
        } else {
            feedback.push(format!("Max history items set to {}", self.max_history_items));
        }

        Ok(feedback)
    }

    /// Reset preferences to default values
    pub fn reset() -> Result<(), CliError> {
        let prefs_path = Self::get_preferences_path()?;
        
        if prefs_path.exists() {
            fs::remove_file(&prefs_path)
                .map_err(|e| CliError::system_error_with_source(
                    "Failed to remove preferences file",
                    Some(format!("Check permissions for {}", prefs_path.display())),
                    Box::new(e)
                ))?;
            debug!("Removed preferences file: {}", prefs_path.display());
        }

        // Save default preferences
        let default_prefs = Self::default();
        default_prefs.save()?;
        
        Ok(())
    }
}

pub fn read_config() -> Config {
    let config_file: &str = include_str!("./assets/client_config.toml");
    return toml::from_str(&config_file).unwrap();
}
