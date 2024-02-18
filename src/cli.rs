use clap::{Parser, Subcommand};
use crate::api::client::{Client, Pet, Device};

/// RustyPet CLI - Your SurePet command-line interface
#[derive(Parser, Debug)]
#[command(name = "rusty_pet")]
#[command(about = "A CLI tool for managing SurePet devices and pets")]
#[command(version)]
pub struct CliArgs {
    /// Output in JSON format instead of human-readable format
    #[arg(long, global = true)]
    pub json: bool,

    /// Enable verbose output for debugging
    #[arg(long, short, global = true)]
    pub verbose: bool,

    /// Subcommand to execute (if not provided, runs in interactive mode)
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Check system status and device information
    Status,
    
    /// List all pets in your account
    List {
        /// Filter by pet name (partial match)
        #[arg(long)]
        name: Option<String>,
        /// Filter by location: 'inside' or 'outside'
        #[arg(long)]
        location: Option<String>,
        /// Sort by: 'name', 'activity', 'location'
        #[arg(long, default_value = "name")]
        sort: String,
    },
    
    /// Set a pet's location (inside/outside)
    SetLocation {
        /// Pet ID or name to update
        pet: String,
        /// Location: 'inside' or 'outside'
        location: String,
    },
    
    /// Lock the pet flap completely (no access)
    Lock {
        /// Device ID or name to lock
        device: String,
    },
    
    /// Set flap to 'Keep In' mode (pets can exit but not enter)
    LockIn {
        /// Device ID or name to set to keep-in mode
        device: String,
    },
    
    /// Set flap to 'Keep Out' mode (pets can enter but not exit)
    LockOut {
        /// Device ID or name to set to keep-out mode
        device: String,
    },
    
    /// Unlock the pet flap (allow free access)
    Unlock {
        /// Device ID or name to unlock
        device: String,
    },
    
    /// Set curfew times for a device
    SetCurfew {
        /// Device ID or name to set curfew for
        device: String,
        /// Lock time in HH:MM format (e.g., 22:00)
        lock_time: Option<String>,
        /// Unlock time in HH:MM format (e.g., 06:00)
        unlock_time: Option<String>,
        /// Disable curfew instead of setting times
        #[arg(long)]
        disable: bool,
    },
    
    /// Mark a pet as indoor
    SetIndoor {
        /// Pet ID or name to mark as indoor
        pet: String,
    },
    
    /// Mark a pet as outdoor
    SetOutdoor {
        /// Pet ID or name to mark as outdoor
        pet: String,
    },
    
    /// Get feeding history for a pet
    FeedingHistory {
        /// Pet ID or name
        pet: String,
        /// Date range: 'today', 'week', 'month', or 'YYYY-MM-DD,YYYY-MM-DD'
        #[arg(long, default_value = "week")]
        range: String,
    },
    
    /// Get drinking history for a pet
    DrinkingHistory {
        /// Pet ID or name
        pet: String,
        /// Date range: 'today', 'week', 'month', or 'YYYY-MM-DD,YYYY-MM-DD'
        #[arg(long, default_value = "week")]
        range: String,
    },
    
    /// Get activity history for a pet
    ActivityHistory {
        /// Pet ID or name
        pet: String,
        /// Date range: 'today', 'week', 'month', or 'YYYY-MM-DD,YYYY-MM-DD'
        #[arg(long, default_value = "week")]
        range: String,
    },
    
    /// Export data to CSV or JSON
    Export {
        /// Export format: 'csv' or 'json'
        #[arg(long, default_value = "csv")]
        format: String,
        /// Data types to export: 'pets', 'devices', 'feeding', 'drinking', 'activity'
        #[arg(long, value_delimiter = ',')]
        types: Vec<String>,
        /// Date range for historical data: 'today', 'week', 'month', or 'YYYY-MM-DD,YYYY-MM-DD'
        #[arg(long, default_value = "month")]
        range: String,
        /// Output file path
        #[arg(long)]
        output: Option<String>,
    },
    
    /// Search pets by various criteria
    SearchPets {
        /// Search by name (partial match)
        #[arg(long)]
        name: Option<String>,
        /// Search by breed
        #[arg(long)]
        breed: Option<String>,
        /// Filter by location: 'inside' or 'outside'
        #[arg(long)]
        location: Option<String>,
        /// Filter by activity since (hours ago)
        #[arg(long)]
        active_since: Option<u32>,
    },
    
    /// Search devices by various criteria
    SearchDevices {
        /// Search by name (partial match)
        #[arg(long)]
        name: Option<String>,
        /// Filter by device type
        #[arg(long)]
        device_type: Option<String>,
        /// Filter by battery level (minimum percentage)
        #[arg(long)]
        min_battery: Option<u8>,
        /// Filter by online status
        #[arg(long)]
        online: Option<bool>,
    },
    
    /// Batch operations on multiple pets or devices
    Batch {
        /// Operation type: 'set-location', 'set-indoor', 'set-outdoor', 'lock', 'unlock'
        operation: String,
        /// Target IDs or names (comma-separated)
        #[arg(value_delimiter = ',')]
        targets: Vec<String>,
        /// Additional parameter (e.g., location for set-location)
        #[arg(long)]
        param: Option<String>,
    },
    
    /// Clear saved authentication token (logout)
    Logout,
    
    /// Reset configuration to default values
    ResetConfig {
        /// Skip confirmation prompt
        #[arg(long)]
        yes: bool,
    },
}

/// Determines the CLI mode based on command-line arguments
#[derive(Debug, PartialEq)]
pub enum CliMode {
    /// Interactive mode with menu-driven interface
    Interactive,
    /// Headless mode with direct command execution
    Headless,
}

impl CliArgs {
    /// Parse command-line arguments and return the CLI arguments structure
    pub fn parse_args() -> Self {
        Self::parse()
    }

    /// Determine whether to run in interactive or headless mode
    pub fn get_mode(&self) -> CliMode {
        match &self.command {
            Some(_) => CliMode::Headless,
            None => CliMode::Interactive,
        }
    }

    /// Check if JSON output format is requested
    pub fn is_json_output(&self) -> bool {
        self.json
    }

    /// Check if verbose output is requested
    pub fn is_verbose(&self) -> bool {
        self.verbose
    }
}

/// Convert location string to location ID
pub fn parse_location(location: &str) -> Result<u32, String> {
    match location.to_lowercase().as_str() {
        "inside" | "in" | "1" => Ok(1),
        "outside" | "out" | "2" => Ok(2),
        _ => Err(format!("Invalid location '{}'. Use 'inside' or 'outside'", location)),
    }
}

/// Parse date range string into DateRange
pub fn parse_date_range(range: &str) -> Result<crate::api::client::DateRange, String> {
    use chrono::{DateTime, Utc, Duration};
    use crate::api::client::DateRange;
    
    let now = Utc::now();
    
    match range.to_lowercase().as_str() {
        "today" => Ok(DateRange {
            from: now.date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc(),
            to: now,
        }),
        "week" => Ok(DateRange {
            from: now - Duration::days(7),
            to: now,
        }),
        "month" => Ok(DateRange {
            from: now - Duration::days(30),
            to: now,
        }),
        custom if custom.contains(',') => {
            let parts: Vec<&str> = custom.split(',').collect();
            if parts.len() != 2 {
                return Err("Custom date range must be in format 'YYYY-MM-DD,YYYY-MM-DD'".to_string());
            }
            
            let from_str = parts[0].trim();
            let to_str = parts[1].trim();
            
            let from = DateTime::parse_from_str(&format!("{} 00:00:00 +0000", from_str), "%Y-%m-%d %H:%M:%S %z")
                .map_err(|_| format!("Invalid from date '{}'. Use YYYY-MM-DD format", from_str))?
                .with_timezone(&Utc);
                
            let to = DateTime::parse_from_str(&format!("{} 23:59:59 +0000", to_str), "%Y-%m-%d %H:%M:%S %z")
                .map_err(|_| format!("Invalid to date '{}'. Use YYYY-MM-DD format", to_str))?
                .with_timezone(&Utc);
            
            if from > to {
                return Err("From date must be before to date".to_string());
            }
            
            Ok(DateRange { from, to })
        }
        _ => Err(format!("Invalid date range '{}'. Use 'today', 'week', 'month', or 'YYYY-MM-DD,YYYY-MM-DD'", range)),
    }
}

/// Validate export format
pub fn validate_export_format(format: &str) -> Result<(), String> {
    match format.to_lowercase().as_str() {
        "csv" | "json" => Ok(()),
        _ => Err(format!("Invalid export format '{}'. Use 'csv' or 'json'", format)),
    }
}

/// Validate export data types
pub fn validate_export_types(types: &[String]) -> Result<(), String> {
    let valid_types = ["pets", "devices", "feeding", "drinking", "activity"];
    
    for data_type in types {
        if !valid_types.contains(&data_type.to_lowercase().as_str()) {
            return Err(format!("Invalid data type '{}'. Valid types: {}", data_type, valid_types.join(", ")));
        }
    }
    
    if types.is_empty() {
        return Err("At least one data type must be specified".to_string());
    }
    
    Ok(())
}

/// Validate sort criteria
pub fn validate_sort_criteria(sort: &str) -> Result<(), String> {
    match sort.to_lowercase().as_str() {
        "name" | "activity" | "location" => Ok(()),
        _ => Err(format!("Invalid sort criteria '{}'. Use 'name', 'activity', or 'location'", sort)),
    }
}

/// Validate batch operation
pub fn validate_batch_operation(operation: &str) -> Result<(), String> {
    match operation.to_lowercase().as_str() {
        "set-location" | "set-indoor" | "set-outdoor" | "lock" | "unlock" | "lock-in" | "lock-out" => Ok(()),
        _ => Err(format!("Invalid batch operation '{}'. Valid operations: set-location, set-indoor, set-outdoor, lock, unlock, lock-in, lock-out", operation)),
    }
}

/// Validate time format (HH:MM)
pub fn validate_time_format(time: &str) -> Result<(), String> {
    let time_regex = regex::Regex::new(r"^([01]?[0-9]|2[0-3]):[0-5][0-9]$").unwrap();
    if time_regex.is_match(time) {
        Ok(())
    } else {
        Err(format!("Invalid time format '{}'. Use HH:MM format (e.g., 22:00)", time))
    }
}

/// Generate help examples for invalid arguments
pub fn generate_help_examples(command: &str, error: &str) -> String {
    match command {
        "set-location" => format!(
            "{}\n\nExamples:\n  rusty_pet set-location \"Fluffy\" inside\n  rusty_pet set-location 123 outside",
            error
        ),
        "lock" | "unlock" | "lock-in" | "lock-out" => format!(
            "{}\n\nExamples:\n  rusty_pet {} \"Pet Door\"\n  rusty_pet {} 456",
            error, command, command
        ),
        "feeding-history" | "drinking-history" | "activity-history" => format!(
            "{}\n\nExamples:\n  rusty_pet {} \"Fluffy\" --range today\n  rusty_pet {} 123 --range week\n  rusty_pet {} \"Max\" --range 2024-01-01,2024-01-31",
            error, command, command, command
        ),
        "export" => format!(
            "{}\n\nExamples:\n  rusty_pet export --format csv --types pets,devices\n  rusty_pet export --format json --types feeding,drinking --range month --output data.json",
            error
        ),
        "search-pets" => format!(
            "{}\n\nExamples:\n  rusty_pet search-pets --name \"Flu\"\n  rusty_pet search-pets --location inside --active-since 24",
            error
        ),
        "batch" => format!(
            "{}\n\nExamples:\n  rusty_pet batch set-location \"Fluffy,Max\" --param inside\n  rusty_pet batch lock \"Door1,Door2\"",
            error
        ),
        _ => error.to_string(),
    }
}

/// Resolve pet identifier (name or ID) to pet ID
pub async fn resolve_pet_id(client: &Client, token: &str, pet_identifier: &str) -> Result<u32, String> {
    // Try to parse as ID first
    if let Ok(id) = pet_identifier.parse::<u32>() {
        return Ok(id);
    }
    
    // Get all pets and search by name
    let pets_response = client.get_pets(token).await
        .map_err(|e| format!("Failed to fetch pets: {}", e))?;
    
    // Look for exact name match first
    if let Some(pet) = pets_response.data.iter().find(|p| p.name.eq_ignore_ascii_case(pet_identifier)) {
        return Ok(pet.id);
    }
    
    // Look for partial name match
    let matches: Vec<&Pet> = pets_response.data.iter()
        .filter(|p| p.name.to_lowercase().contains(&pet_identifier.to_lowercase()))
        .collect();
    
    match matches.len() {
        0 => Err(format!("No pet found with name or ID '{}'", pet_identifier)),
        1 => Ok(matches[0].id),
        _ => {
            let names: Vec<String> = matches.iter().map(|p| format!("{} (ID: {})", p.name, p.id)).collect();
            Err(format!("Multiple pets match '{}': {}. Please be more specific or use the pet ID.", 
                pet_identifier, names.join(", ")))
        }
    }
}

/// Resolve device identifier (name or ID) to device ID
pub async fn resolve_device_id(client: &Client, token: &str, device_identifier: &str) -> Result<u32, String> {
    // Try to parse as ID first
    if let Ok(id) = device_identifier.parse::<u32>() {
        return Ok(id);
    }
    
    // Get all devices and search by name
    let devices_response = client.get_devices(token).await
        .map_err(|e| format!("Failed to fetch devices: {}", e))?;
    
    // Look for exact name match first
    if let Some(device) = devices_response.data.iter().find(|d| d.name.eq_ignore_ascii_case(device_identifier)) {
        return Ok(device.id);
    }
    
    // Look for partial name match
    let matches: Vec<&Device> = devices_response.data.iter()
        .filter(|d| d.name.to_lowercase().contains(&device_identifier.to_lowercase()))
        .collect();
    
    match matches.len() {
        0 => Err(format!("No device found with name or ID '{}'", device_identifier)),
        1 => Ok(matches[0].id),
        _ => {
            let names: Vec<String> = matches.iter().map(|d| format!("{} (ID: {})", d.name, d.id)).collect();
            Err(format!("Multiple devices match '{}': {}. Please be more specific or use the device ID.", 
                device_identifier, names.join(", ")))
        }
    }
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_location() {
        assert_eq!(parse_location("inside"), Ok(1));
        assert_eq!(parse_location("Inside"), Ok(1));
        assert_eq!(parse_location("in"), Ok(1));
        assert_eq!(parse_location("1"), Ok(1));
        
        assert_eq!(parse_location("outside"), Ok(2));
        assert_eq!(parse_location("Outside"), Ok(2));
        assert_eq!(parse_location("out"), Ok(2));
        assert_eq!(parse_location("2"), Ok(2));
        
        assert!(parse_location("invalid").is_err());
    }

    #[test]
    fn test_cli_mode_detection() {
        // Test interactive mode (no subcommand)
        let args = CliArgs {
            json: false,
            verbose: false,
            command: None,
        };
        assert_eq!(args.get_mode(), CliMode::Interactive);

        // Test headless mode (with subcommand)
        let args = CliArgs {
            json: false,
            verbose: false,
            command: Some(Commands::Status),
        };
        assert_eq!(args.get_mode(), CliMode::Headless);
    }

    #[test]
    fn test_json_output_flag() {
        let args = CliArgs {
            json: true,
            verbose: false,
            command: None,
        };
        assert!(args.is_json_output());

        let args = CliArgs {
            json: false,
            verbose: false,
            command: None,
        };
        assert!(!args.is_json_output());
    }
}