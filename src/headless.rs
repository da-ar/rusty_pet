use crate::api::client::{Client, CurfewTime};
use crate::cache::CacheManager;
use crate::cli::{CliArgs, Commands, parse_location, parse_date_range, validate_export_format, 
                validate_export_types, validate_sort_criteria, validate_batch_operation, 
                validate_time_format, generate_help_examples, resolve_pet_id, resolve_device_id};
use crate::config::UserPreferences;
use crate::data_validation::{PetDataValidator, DeviceDataValidator};
use crate::formatters::{OutputFormatter, create_formatter};
use crate::offline_manager::OfflineManager;
use crate::token;
use log::{debug, warn};
use std::io;
use std::sync::Arc;

/// Headless mode handler for direct command execution
pub struct HeadlessMode {
    pub client: Client,
    pub args: CliArgs,
    formatter: Box<dyn OutputFormatter>,
    #[allow(dead_code)] // Future functionality
    cache_manager: Option<Arc<CacheManager>>,
    #[allow(dead_code)] // Future functionality
    offline_manager: Option<Arc<OfflineManager>>,
}

impl HeadlessMode {
    /// Create a new headless mode handler
    pub fn new(client: Client, args: CliArgs) -> Self {
        let formatter = create_formatter(args.is_json_output());
        
        Self {
            client,
            args,
            formatter,
            cache_manager: None,
            offline_manager: None,
        }
    }

    /// Create a new headless mode handler with cache manager
    pub fn new_with_cache(client: Client, args: CliArgs, cache_manager: Arc<CacheManager>) -> Self {
        let formatter = create_formatter(args.is_json_output());
        
        Self {
            client,
            args,
            formatter,
            cache_manager: Some(cache_manager),
            offline_manager: None,
        }
    }

    /// Create a new headless mode handler with all managers
    pub fn new_with_managers(client: Client, args: CliArgs, cache_manager: Arc<CacheManager>, offline_manager: Arc<OfflineManager>) -> Self {
        let formatter = create_formatter(args.is_json_output());
        
        Self {
            client,
            args,
            formatter,
            cache_manager: Some(cache_manager),
            offline_manager: Some(offline_manager),
        }
    }

    /// Execute the headless command
    pub async fn execute(&self, token: &str) -> io::Result<()> {
        match &self.args.command {
            Some(command) => self.execute_command(command, token).await,
            None => {
                // This shouldn't happen in headless mode, but handle gracefully
                Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "No command specified for headless mode"
                ))
            }
        }
    }

    async fn execute_command(&self, command: &Commands, token: &str) -> io::Result<()> {
        match command {
            Commands::Status => self.handle_status(token).await,
            Commands::List { name, location, sort } => {
                self.handle_list(token, name.as_deref(), location.as_deref(), sort).await
            }
            Commands::SetLocation { pet, location } => {
                self.handle_set_location(pet, location, token).await
            }
            Commands::Lock { device } => self.handle_lock(device, token).await,
            Commands::LockIn { device } => self.handle_lock_in(device, token).await,
            Commands::LockOut { device } => self.handle_lock_out(device, token).await,
            Commands::Unlock { device } => self.handle_unlock(device, token).await,
            Commands::SetCurfew { device, lock_time, unlock_time, disable } => {
                self.handle_set_curfew(device, lock_time.as_deref(), unlock_time.as_deref(), *disable, token).await
            }
            Commands::SetIndoor { pet } => self.handle_set_indoor(pet, token).await,
            Commands::SetOutdoor { pet } => self.handle_set_outdoor(pet, token).await,
            Commands::FeedingHistory { pet, range } => {
                self.handle_feeding_history(pet, range, token).await
            }
            Commands::DrinkingHistory { pet, range } => {
                self.handle_drinking_history(pet, range, token).await
            }
            Commands::ActivityHistory { pet, range } => {
                self.handle_activity_history(pet, range, token).await
            }
            Commands::Export { format, types, range, output } => {
                self.handle_export(format, types, range, output.as_deref(), token).await
            }
            Commands::SearchPets { name, breed, location, active_since } => {
                self.handle_search_pets(name.as_deref(), breed.as_deref(), location.as_deref(), *active_since, token).await
            }
            Commands::SearchDevices { name, device_type, min_battery, online } => {
                self.handle_search_devices(name.as_deref(), device_type.as_deref(), *min_battery, *online, token).await
            }
            Commands::Batch { operation, targets, param } => {
                self.handle_batch(operation, targets, param.as_deref(), token).await
            }
            Commands::Logout => self.handle_logout().await,
            Commands::ResetConfig { yes } => self.handle_reset_config(*yes).await,
        }
    }

    async fn handle_status(&self, token: &str) -> io::Result<()> {
        debug!("Executing status command in headless mode");
        
        match self.client.get_devices(token).await {
            Ok(devices_response) => {
                // Perform data completeness checks for each device (Requirement 4.1)
                for device in &devices_response.data {
                    let status_report = DeviceDataValidator::validate_device_status(device);
                    
                    // Log warnings for headless mode monitoring
                    for warning in &status_report.warnings {
                        warn!("Device {}: {}", device.name, warning);
                    }
                    
                    if !status_report.is_complete {
                        warn!("Device {} has incomplete status data: missing {}", 
                              device.name, status_report.missing_fields.join(", "));
                    }
                }
                
                let output = self.formatter.format_devices(&devices_response);
                print!("{}", output);
                Ok(())
            }
            Err(e) if e.status() == Some(reqwest::StatusCode::UNAUTHORIZED) => {
                let _ = token::delete_token();
                let error_msg = "Authentication failed. Please run the command again to re-authenticate.";
                let output = self.formatter.format_error(error_msg);
                print!("{}", output);
                Err(io::Error::new(io::ErrorKind::PermissionDenied, error_msg))
            }
            Err(e) => {
                let error_msg = format!("Failed to get device status: {}", e);
                let output = self.formatter.format_error(&error_msg);
                print!("{}", output);
                Err(io::Error::new(io::ErrorKind::Other, error_msg))
            }
        }
    }

    async fn handle_list(&self, token: &str, name_filter: Option<&str>, location_filter: Option<&str>, sort_by: &str) -> io::Result<()> {
        debug!("Executing list command in headless mode with filters");
        
        // Validate sort criteria
        if let Err(e) = validate_sort_criteria(sort_by) {
            let help = generate_help_examples("list", &e);
            let output = self.formatter.format_error(&help);
            print!("{}", output);
            return Err(io::Error::new(io::ErrorKind::InvalidInput, e));
        }
        
        match self.client.get_pets(token).await {
            Ok(mut pets_response) => {
                // Perform data completeness checks for each pet (Requirement 3.3)
                for pet in &pets_response.data {
                    let details_report = PetDataValidator::validate_pet_details(pet);
                    
                    // Log warnings for headless mode monitoring
                    for warning in &details_report.warnings {
                        warn!("Pet {}: {}", pet.name, warning);
                    }
                    
                    if !details_report.is_complete {
                        warn!("Pet {} has incomplete details: missing {}", 
                              pet.name, details_report.missing_fields.join(", "));
                    }
                }
                
                // Apply name filter
                if let Some(name) = name_filter {
                    pets_response.data.retain(|pet| 
                        pet.name.to_lowercase().contains(&name.to_lowercase())
                    );
                }
                
                // Apply location filter
                if let Some(location) = location_filter {
                    let location_id = match parse_location(location) {
                        Ok(id) => id,
                        Err(e) => {
                            let help = generate_help_examples("list", &e);
                            let output = self.formatter.format_error(&help);
                            print!("{}", output);
                            return Err(io::Error::new(io::ErrorKind::InvalidInput, e));
                        }
                    };
                    
                    pets_response.data.retain(|pet| 
                        pet.position.as_ref()
                            .and_then(|p| p.location)
                            .map_or(false, |loc| loc == location_id)
                    );
                }
                
                // Apply sorting
                match sort_by.to_lowercase().as_str() {
                    "name" => pets_response.data.sort_by(|a, b| a.name.cmp(&b.name)),
                    "activity" => pets_response.data.sort_by(|a, b| {
                        let empty_string = String::new();
                        let a_time = a.position.as_ref().map(|p| &p.since).unwrap_or(&empty_string);
                        let b_time = b.position.as_ref().map(|p| &p.since).unwrap_or(&empty_string);
                        b_time.cmp(a_time) // Most recent first
                    }),
                    "location" => pets_response.data.sort_by(|a, b| {
                        let a_loc = a.position.as_ref().and_then(|p| p.location).unwrap_or(0);
                        let b_loc = b.position.as_ref().and_then(|p| p.location).unwrap_or(0);
                        a_loc.cmp(&b_loc)
                    }),
                    _ => {} // Already validated above
                }
                
                let output = self.formatter.format_pets(&pets_response);
                print!("{}", output);
                Ok(())
            }
            Err(e) if e.status() == Some(reqwest::StatusCode::UNAUTHORIZED) => {
                let _ = token::delete_token();
                let error_msg = "Authentication failed. Please run the command again to re-authenticate.";
                let output = self.formatter.format_error(error_msg);
                print!("{}", output);
                Err(io::Error::new(io::ErrorKind::PermissionDenied, error_msg))
            }
            Err(e) => {
                let error_msg = format!("Failed to get pets: {}", e);
                let output = self.formatter.format_error(&error_msg);
                print!("{}", output);
                Err(io::Error::new(io::ErrorKind::Other, error_msg))
            }
        }
    }

    async fn handle_set_location(&self, pet_identifier: &str, location: &str, token: &str) -> io::Result<()> {
        debug!("Executing set-location command for pet {} to {}", pet_identifier, location);
        
        let location_id = match parse_location(location) {
            Ok(id) => id,
            Err(e) => {
                let help = generate_help_examples("set-location", &e);
                let output = self.formatter.format_error(&help);
                print!("{}", output);
                return Err(io::Error::new(io::ErrorKind::InvalidInput, e));
            }
        };

        let pet_id = match resolve_pet_id(&self.client, token, pet_identifier).await {
            Ok(id) => id,
            Err(e) => {
                let help = generate_help_examples("set-location", &e);
                let output = self.formatter.format_error(&help);
                print!("{}", output);
                return Err(io::Error::new(io::ErrorKind::InvalidInput, e));
            }
        };

        match self.client.set_pet_location(token, pet_id, location_id).await {
            Ok(()) => {
                let location_name = if location_id == 1 { "inside" } else { "outside" };
                let success_msg = format!("Pet {} location set to {}", pet_identifier, location_name);
                let output = self.formatter.format_success_message(&success_msg);
                print!("{}", output);
                Ok(())
            }
            Err(e) if e.status() == Some(reqwest::StatusCode::UNAUTHORIZED) => {
                let _ = token::delete_token();
                let error_msg = "Authentication failed. Please run the command again to re-authenticate.";
                let output = self.formatter.format_error(error_msg);
                print!("{}", output);
                Err(io::Error::new(io::ErrorKind::PermissionDenied, error_msg))
            }
            Err(e) => {
                let error_msg = format!("Failed to set pet location: {}", e);
                let output = self.formatter.format_error(&error_msg);
                print!("{}", output);
                Err(io::Error::new(io::ErrorKind::Other, error_msg))
            }
        }
    }

    async fn handle_lock(&self, device_identifier: &str, token: &str) -> io::Result<()> {
        debug!("Executing lock command for device {}", device_identifier);
        
        let device_id = match resolve_device_id(&self.client, token, device_identifier).await {
            Ok(id) => id,
            Err(e) => {
                let help = generate_help_examples("lock", &e);
                let output = self.formatter.format_error(&help);
                print!("{}", output);
                return Err(io::Error::new(io::ErrorKind::InvalidInput, e));
            }
        };
        
        match self.client.lock(token, device_id).await {
            Ok(()) => {
                let success_msg = format!("Device {} locked (no access allowed)", device_identifier);
                let output = self.formatter.format_success_message(&success_msg);
                print!("{}", output);
                Ok(())
            }
            Err(e) if e.status() == Some(reqwest::StatusCode::UNAUTHORIZED) => {
                let _ = token::delete_token();
                let error_msg = "Authentication failed. Please run the command again to re-authenticate.";
                let output = self.formatter.format_error(error_msg);
                print!("{}", output);
                Err(io::Error::new(io::ErrorKind::PermissionDenied, error_msg))
            }
            Err(e) => {
                let error_msg = format!("Failed to lock device: {}", e);
                let output = self.formatter.format_error(&error_msg);
                print!("{}", output);
                Err(io::Error::new(io::ErrorKind::Other, error_msg))
            }
        }
    }

    async fn handle_lock_in(&self, device_identifier: &str, token: &str) -> io::Result<()> {
        debug!("Executing lock-in command for device {}", device_identifier);
        
        let device_id = match resolve_device_id(&self.client, token, device_identifier).await {
            Ok(id) => id,
            Err(e) => {
                let help = generate_help_examples("lock-in", &e);
                let output = self.formatter.format_error(&help);
                print!("{}", output);
                return Err(io::Error::new(io::ErrorKind::InvalidInput, e));
            }
        };
        
        match self.client.lock_in(token, device_id).await {
            Ok(()) => {
                let success_msg = format!("Device {} set to 'Keep In' mode (pets can exit but not enter)", device_identifier);
                let output = self.formatter.format_success_message(&success_msg);
                print!("{}", output);
                Ok(())
            }
            Err(e) if e.status() == Some(reqwest::StatusCode::UNAUTHORIZED) => {
                let _ = token::delete_token();
                let error_msg = "Authentication failed. Please run the command again to re-authenticate.";
                let output = self.formatter.format_error(error_msg);
                print!("{}", output);
                Err(io::Error::new(io::ErrorKind::PermissionDenied, error_msg))
            }
            Err(e) => {
                let error_msg = format!("Failed to set lock-in mode: {}", e);
                let output = self.formatter.format_error(&error_msg);
                print!("{}", output);
                Err(io::Error::new(io::ErrorKind::Other, error_msg))
            }
        }
    }

    async fn handle_lock_out(&self, device_identifier: &str, token: &str) -> io::Result<()> {
        debug!("Executing lock-out command for device {}", device_identifier);
        
        let device_id = match resolve_device_id(&self.client, token, device_identifier).await {
            Ok(id) => id,
            Err(e) => {
                let help = generate_help_examples("lock-out", &e);
                let output = self.formatter.format_error(&help);
                print!("{}", output);
                return Err(io::Error::new(io::ErrorKind::InvalidInput, e));
            }
        };
        
        match self.client.lock_out(token, device_id).await {
            Ok(()) => {
                let success_msg = format!("Device {} set to 'Keep Out' mode (pets can enter but not exit)", device_identifier);
                let output = self.formatter.format_success_message(&success_msg);
                print!("{}", output);
                Ok(())
            }
            Err(e) if e.status() == Some(reqwest::StatusCode::UNAUTHORIZED) => {
                let _ = token::delete_token();
                let error_msg = "Authentication failed. Please run the command again to re-authenticate.";
                let output = self.formatter.format_error(error_msg);
                print!("{}", output);
                Err(io::Error::new(io::ErrorKind::PermissionDenied, error_msg))
            }
            Err(e) => {
                let error_msg = format!("Failed to set lock-out mode: {}", e);
                let output = self.formatter.format_error(&error_msg);
                print!("{}", output);
                Err(io::Error::new(io::ErrorKind::Other, error_msg))
            }
        }
    }

    async fn handle_unlock(&self, device_identifier: &str, token: &str) -> io::Result<()> {
        debug!("Executing unlock command for device {}", device_identifier);
        
        let device_id = match resolve_device_id(&self.client, token, device_identifier).await {
            Ok(id) => id,
            Err(e) => {
                let help = generate_help_examples("unlock", &e);
                let output = self.formatter.format_error(&help);
                print!("{}", output);
                return Err(io::Error::new(io::ErrorKind::InvalidInput, e));
            }
        };
        
        match self.client.unlock(token, device_id).await {
            Ok(()) => {
                let success_msg = format!("Device {} unlocked (free access allowed)", device_identifier);
                let output = self.formatter.format_success_message(&success_msg);
                print!("{}", output);
                Ok(())
            }
            Err(e) if e.status() == Some(reqwest::StatusCode::UNAUTHORIZED) => {
                let _ = token::delete_token();
                let error_msg = "Authentication failed. Please run the command again to re-authenticate.";
                let output = self.formatter.format_error(error_msg);
                print!("{}", output);
                Err(io::Error::new(io::ErrorKind::PermissionDenied, error_msg))
            }
            Err(e) => {
                let error_msg = format!("Failed to unlock device: {}", e);
                let output = self.formatter.format_error(&error_msg);
                print!("{}", output);
                Err(io::Error::new(io::ErrorKind::Other, error_msg))
            }
        }
    }

    async fn handle_set_curfew(&self, device_identifier: &str, lock_time: Option<&str>, unlock_time: Option<&str>, disable: bool, token: &str) -> io::Result<()> {
        debug!("Executing set-curfew command for device {}", device_identifier);
        
        let device_id = match resolve_device_id(&self.client, token, device_identifier).await {
            Ok(id) => id,
            Err(e) => {
                let help = generate_help_examples("set-curfew", &e);
                let output = self.formatter.format_error(&help);
                print!("{}", output);
                return Err(io::Error::new(io::ErrorKind::InvalidInput, e));
            }
        };
        
        let curfew_times = if disable {
            vec![]
        } else {
            match (lock_time, unlock_time) {
                (Some(lock), Some(unlock)) => {
                    // Validate time formats
                    if let Err(e) = validate_time_format(lock) {
                        let help = generate_help_examples("set-curfew", &e);
                        let output = self.formatter.format_error(&help);
                        print!("{}", output);
                        return Err(io::Error::new(io::ErrorKind::InvalidInput, e));
                    }
                    if let Err(e) = validate_time_format(unlock) {
                        let help = generate_help_examples("set-curfew", &e);
                        let output = self.formatter.format_error(&help);
                        print!("{}", output);
                        return Err(io::Error::new(io::ErrorKind::InvalidInput, e));
                    }
                    
                    vec![CurfewTime {
                        enabled: true,
                        lock_time: lock.to_string(),
                        unlock_time: unlock.to_string(),
                    }]
                }
                _ => {
                    let error_msg = "Both lock-time and unlock-time are required when setting curfew (or use --disable to disable curfew)";
                    let help = generate_help_examples("set-curfew", error_msg);
                    let output = self.formatter.format_error(&help);
                    print!("{}", output);
                    return Err(io::Error::new(io::ErrorKind::InvalidInput, error_msg));
                }
            }
        };

        match self.client.set_curfew(token, device_id, curfew_times).await {
            Ok(()) => {
                let success_msg = if disable {
                    format!("Curfew disabled for device {}", device_identifier)
                } else {
                    format!("Curfew set for device {} ({} - {})", device_identifier, lock_time.unwrap(), unlock_time.unwrap())
                };
                let output = self.formatter.format_success_message(&success_msg);
                print!("{}", output);
                Ok(())
            }
            Err(e) if e.status() == Some(reqwest::StatusCode::UNAUTHORIZED) => {
                let _ = token::delete_token();
                let error_msg = "Authentication failed. Please run the command again to re-authenticate.";
                let output = self.formatter.format_error(error_msg);
                print!("{}", output);
                Err(io::Error::new(io::ErrorKind::PermissionDenied, error_msg))
            }
            Err(e) => {
                let error_msg = format!("Failed to set curfew: {}", e);
                let output = self.formatter.format_error(&error_msg);
                print!("{}", output);
                Err(io::Error::new(io::ErrorKind::Other, error_msg))
            }
        }
    }

    async fn handle_set_indoor(&self, pet_identifier: &str, token: &str) -> io::Result<()> {
        debug!("Executing set-indoor command for pet {}", pet_identifier);
        
        let pet_id = match resolve_pet_id(&self.client, token, pet_identifier).await {
            Ok(id) => id,
            Err(e) => {
                let help = generate_help_examples("set-indoor", &e);
                let output = self.formatter.format_error(&help);
                print!("{}", output);
                return Err(io::Error::new(io::ErrorKind::InvalidInput, e));
            }
        };
        
        match self.client.set_pet_indoor_mode(token, pet_id).await {
            Ok(()) => {
                let success_msg = format!("Pet {} marked as indoor", pet_identifier);
                let output = self.formatter.format_success_message(&success_msg);
                print!("{}", output);
                Ok(())
            }
            Err(e) if e.status() == Some(reqwest::StatusCode::UNAUTHORIZED) => {
                let _ = token::delete_token();
                let error_msg = "Authentication failed. Please run the command again to re-authenticate.";
                let output = self.formatter.format_error(error_msg);
                print!("{}", output);
                Err(io::Error::new(io::ErrorKind::PermissionDenied, error_msg))
            }
            Err(e) => {
                let error_msg = format!("Failed to set pet indoor: {}", e);
                let output = self.formatter.format_error(&error_msg);
                print!("{}", output);
                Err(io::Error::new(io::ErrorKind::Other, error_msg))
            }
        }
    }

    async fn handle_set_outdoor(&self, pet_identifier: &str, token: &str) -> io::Result<()> {
        debug!("Executing set-outdoor command for pet {}", pet_identifier);
        
        let pet_id = match resolve_pet_id(&self.client, token, pet_identifier).await {
            Ok(id) => id,
            Err(e) => {
                let help = generate_help_examples("set-outdoor", &e);
                let output = self.formatter.format_error(&help);
                print!("{}", output);
                return Err(io::Error::new(io::ErrorKind::InvalidInput, e));
            }
        };
        
        match self.client.set_pet_outdoor_mode(token, pet_id).await {
            Ok(()) => {
                let success_msg = format!("Pet {} marked as outdoor", pet_identifier);
                let output = self.formatter.format_success_message(&success_msg);
                print!("{}", output);
                Ok(())
            }
            Err(e) if e.status() == Some(reqwest::StatusCode::UNAUTHORIZED) => {
                let _ = token::delete_token();
                let error_msg = "Authentication failed. Please run the command again to re-authenticate.";
                let output = self.formatter.format_error(error_msg);
                print!("{}", output);
                Err(io::Error::new(io::ErrorKind::PermissionDenied, error_msg))
            }
            Err(e) => {
                let error_msg = format!("Failed to set pet outdoor: {}", e);
                let output = self.formatter.format_error(&error_msg);
                print!("{}", output);
                Err(io::Error::new(io::ErrorKind::Other, error_msg))
            }
        }
    }

    async fn handle_logout(&self) -> io::Result<()> {
        debug!("Executing logout command");
        
        match token::delete_token() {
            Ok(_) => {
                let success_msg = "Successfully logged out. Authentication token has been cleared.";
                let output = self.formatter.format_success_message(success_msg);
                print!("{}", output);
                Ok(())
            }
            Err(e) => {
                let error_msg = format!("Failed to clear authentication token: {}", e);
                let output = self.formatter.format_error(&error_msg);
                print!("{}", output);
                Err(io::Error::new(io::ErrorKind::Other, error_msg))
            }
        }
    }

    async fn handle_reset_config(&self, skip_confirmation: bool) -> io::Result<()> {
        debug!("Executing reset-config command");
        
        if !skip_confirmation {
            if self.args.is_json_output() {
                // For JSON output, require --yes flag
                let error_msg = "Configuration reset requires confirmation. Use --yes flag to proceed.";
                let output = self.formatter.format_error(error_msg);
                print!("{}", output);
                return Err(io::Error::new(io::ErrorKind::InvalidInput, error_msg));
            } else {
                // For human output, show warning and require --yes flag
                let warning_msg = "This will reset all preferences to default values.\nUse --yes flag to confirm: rusty_pet reset-config --yes";
                let output = self.formatter.format_error(warning_msg);
                print!("{}", output);
                return Err(io::Error::new(io::ErrorKind::InvalidInput, "Confirmation required"));
            }
        }

        match UserPreferences::reset() {
            Ok(_) => {
                let success_msg = "Configuration has been reset to default values.";
                let output = self.formatter.format_success_message(success_msg);
                print!("{}", output);
                Ok(())
            }
            Err(e) => {
                let error_msg = format!("Failed to reset configuration: {}", e.user_message());
                let output = self.formatter.format_error(&error_msg);
                print!("{}", output);
                Err(io::Error::new(io::ErrorKind::Other, error_msg))
            }
        }
    }
    
    async fn handle_feeding_history(&self, pet_identifier: &str, range: &str, token: &str) -> io::Result<()> {
        debug!("Executing feeding-history command for pet {} with range {}", pet_identifier, range);
        
        let pet_id = match resolve_pet_id(&self.client, token, pet_identifier).await {
            Ok(id) => id,
            Err(e) => {
                let help = generate_help_examples("feeding-history", &e);
                let output = self.formatter.format_error(&help);
                print!("{}", output);
                return Err(io::Error::new(io::ErrorKind::InvalidInput, e));
            }
        };
        
        let date_range = match parse_date_range(range) {
            Ok(range) => range,
            Err(e) => {
                let help = generate_help_examples("feeding-history", &e);
                let output = self.formatter.format_error(&help);
                print!("{}", output);
                return Err(io::Error::new(io::ErrorKind::InvalidInput, e));
            }
        };
        
        match self.client.get_feeding_history(token, pet_id, date_range).await {
            Ok(history) => {
                // Perform data completeness validation for feeding history (Requirement 4.5)
                if history.events.is_empty() {
                    warn!("No feeding history available for pet {}", pet_identifier);
                } else {
                    // Validate data completeness
                    let mut incomplete_events = 0;
                    for (i, event) in history.events.iter().enumerate() {
                        if event.amount <= 0.0 {
                            warn!("Feeding event {} has invalid amount: {}", i + 1, event.amount);
                            incomplete_events += 1;
                        }
                        if event.duration.is_none() {
                            warn!("Feeding event {} missing duration information", i + 1);
                            incomplete_events += 1;
                        }
                    }
                    
                    if incomplete_events > 0 {
                        warn!("Found {} incomplete feeding events for pet {}", incomplete_events, pet_identifier);
                    }
                    
                    // Check for data freshness
                    if let Some(latest_event) = history.events.first() {
                        let now = chrono::Utc::now();
                        let time_diff = now.signed_duration_since(latest_event.timestamp);
                        if time_diff > chrono::Duration::days(7) {
                            warn!("Latest feeding data for pet {} is {} days old", pet_identifier, time_diff.num_days());
                        }
                    }
                }
                
                // For now, use a simple JSON output since we don't have a specific formatter method
                if self.args.is_json_output() {
                    let json_output = serde_json::to_string_pretty(&history)
                        .unwrap_or_else(|_| "Failed to serialize feeding history".to_string());
                    print!("{}", json_output);
                } else {
                    println!("Feeding History for Pet {}:", pet_identifier);
                    println!("Events: {}", history.events.len());
                    for event in &history.events {
                        let duration_str = event.duration
                            .map(|d| format!("{}s", d))
                            .unwrap_or_else(|| "N/A".to_string());
                        println!("  {} - Amount: {:.2}g, Duration: {}", 
                            event.timestamp.format("%Y-%m-%d %H:%M:%S"),
                            event.amount,
                            duration_str
                        );
                    }
                }
                Ok(())
            }
            Err(e) if e.status() == Some(reqwest::StatusCode::UNAUTHORIZED) => {
                let _ = token::delete_token();
                let error_msg = "Authentication failed. Please run the command again to re-authenticate.";
                let output = self.formatter.format_error(error_msg);
                print!("{}", output);
                Err(io::Error::new(io::ErrorKind::PermissionDenied, error_msg))
            }
            Err(e) => {
                let error_msg = format!("Failed to get feeding history: {}", e);
                let output = self.formatter.format_error(&error_msg);
                print!("{}", output);
                Err(io::Error::new(io::ErrorKind::Other, error_msg))
            }
        }
    }
    
    async fn handle_drinking_history(&self, pet_identifier: &str, range: &str, token: &str) -> io::Result<()> {
        debug!("Executing drinking-history command for pet {} with range {}", pet_identifier, range);
        
        let pet_id = match resolve_pet_id(&self.client, token, pet_identifier).await {
            Ok(id) => id,
            Err(e) => {
                let help = generate_help_examples("drinking-history", &e);
                let output = self.formatter.format_error(&help);
                print!("{}", output);
                return Err(io::Error::new(io::ErrorKind::InvalidInput, e));
            }
        };
        
        let date_range = match parse_date_range(range) {
            Ok(range) => range,
            Err(e) => {
                let help = generate_help_examples("drinking-history", &e);
                let output = self.formatter.format_error(&help);
                print!("{}", output);
                return Err(io::Error::new(io::ErrorKind::InvalidInput, e));
            }
        };
        
        match self.client.get_drinking_history(token, pet_id, date_range).await {
            Ok(history) => {
                // For now, use a simple JSON output since we don't have a specific formatter method
                if self.args.is_json_output() {
                    let json_output = serde_json::to_string_pretty(&history)
                        .unwrap_or_else(|_| "Failed to serialize drinking history".to_string());
                    print!("{}", json_output);
                } else {
                    println!("Drinking History for Pet {}:", pet_identifier);
                    println!("Events: {}", history.events.len());
                    for event in &history.events {
                        let duration_str = event.duration
                            .map(|d| format!("{}s", d))
                            .unwrap_or_else(|| "N/A".to_string());
                        println!("  {} - Volume: {:.2}ml, Duration: {}", 
                            event.timestamp.format("%Y-%m-%d %H:%M:%S"),
                            event.volume,
                            duration_str
                        );
                    }
                }
                Ok(())
            }
            Err(e) if e.status() == Some(reqwest::StatusCode::UNAUTHORIZED) => {
                let _ = token::delete_token();
                let error_msg = "Authentication failed. Please run the command again to re-authenticate.";
                let output = self.formatter.format_error(error_msg);
                print!("{}", output);
                Err(io::Error::new(io::ErrorKind::PermissionDenied, error_msg))
            }
            Err(e) => {
                let error_msg = format!("Failed to get drinking history: {}", e);
                let output = self.formatter.format_error(&error_msg);
                print!("{}", output);
                Err(io::Error::new(io::ErrorKind::Other, error_msg))
            }
        }
    }
    
    async fn handle_activity_history(&self, pet_identifier: &str, range: &str, token: &str) -> io::Result<()> {
        debug!("Executing activity-history command for pet {} with range {}", pet_identifier, range);
        
        let pet_id = match resolve_pet_id(&self.client, token, pet_identifier).await {
            Ok(id) => id,
            Err(e) => {
                let help = generate_help_examples("activity-history", &e);
                let output = self.formatter.format_error(&help);
                print!("{}", output);
                return Err(io::Error::new(io::ErrorKind::InvalidInput, e));
            }
        };
        
        let date_range = match parse_date_range(range) {
            Ok(range) => range,
            Err(e) => {
                let help = generate_help_examples("activity-history", &e);
                let output = self.formatter.format_error(&help);
                print!("{}", output);
                return Err(io::Error::new(io::ErrorKind::InvalidInput, e));
            }
        };
        
        match self.client.get_activity_history(token, pet_id, date_range).await {
            Ok(history) => {
                // For now, use a simple JSON output since we don't have a specific formatter method
                if self.args.is_json_output() {
                    let json_output = serde_json::to_string_pretty(&history)
                        .unwrap_or_else(|_| "Failed to serialize activity history".to_string());
                    print!("{}", json_output);
                } else {
                    println!("Activity History for Pet {}:", pet_identifier);
                    println!("Events: {}", history.events.len());
                    for event in &history.events {
                        println!("  {} - Type: {:?}, Location: {:?}", 
                            event.timestamp.format("%Y-%m-%d %H:%M:%S"),
                            event.event_type,
                            event.location
                        );
                    }
                }
                Ok(())
            }
            Err(e) if e.status() == Some(reqwest::StatusCode::UNAUTHORIZED) => {
                let _ = token::delete_token();
                let error_msg = "Authentication failed. Please run the command again to re-authenticate.";
                let output = self.formatter.format_error(error_msg);
                print!("{}", output);
                Err(io::Error::new(io::ErrorKind::PermissionDenied, error_msg))
            }
            Err(e) => {
                let error_msg = format!("Failed to get activity history: {}", e);
                let output = self.formatter.format_error(&error_msg);
                print!("{}", output);
                Err(io::Error::new(io::ErrorKind::Other, error_msg))
            }
        }
    }
    
    async fn handle_export(&self, format: &str, types: &[String], range: &str, output_path: Option<&str>, _token: &str) -> io::Result<()> {
        debug!("Executing export command with format {} and types {:?}", format, types);
        
        // Validate parameters
        if let Err(e) = validate_export_format(format) {
            let help = generate_help_examples("export", &e);
            let output = self.formatter.format_error(&help);
            print!("{}", output);
            return Err(io::Error::new(io::ErrorKind::InvalidInput, e));
        }
        
        if let Err(e) = validate_export_types(types) {
            let help = generate_help_examples("export", &e);
            let output = self.formatter.format_error(&help);
            print!("{}", output);
            return Err(io::Error::new(io::ErrorKind::InvalidInput, e));
        }
        
        let _date_range = match parse_date_range(range) {
            Ok(range) => range,
            Err(e) => {
                let help = generate_help_examples("export", &e);
                let output = self.formatter.format_error(&help);
                print!("{}", output);
                return Err(io::Error::new(io::ErrorKind::InvalidInput, e));
            }
        };
        
        // Generate output filename if not provided
        let output_file = match output_path {
            Some(path) => path.to_string(),
            None => {
                let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
                let extension = if format == "csv" { "csv" } else { "json" };
                format!("rusty_pet_export_{}_{}.{}", types.join("_"), timestamp, extension)
            }
        };
        
        // This is a simplified implementation - in a real scenario, you'd collect the data
        // based on the types and date range, then use ExportManager to export it
        let success_msg = format!("Export would be saved to: {} (format: {}, types: {:?}, range: {})", 
            output_file, format, types, range);
        let output = self.formatter.format_success_message(&success_msg);
        print!("{}", output);
        
        // TODO: Implement actual export functionality using ExportManager
        println!("Note: Full export functionality requires integration with ExportManager");
        
        Ok(())
    }
    
    async fn handle_search_pets(&self, name: Option<&str>, breed: Option<&str>, location: Option<&str>, active_since: Option<u32>, _token: &str) -> io::Result<()> {
        debug!("Executing search-pets command");
        
        // Validate location if provided
        if let Some(loc) = location {
            if let Err(e) = parse_location(loc) {
                let help = generate_help_examples("search-pets", &e);
                let output = self.formatter.format_error(&help);
                print!("{}", output);
                return Err(io::Error::new(io::ErrorKind::InvalidInput, e));
            }
        }
        
        // This is a simplified implementation - in a real scenario, you'd use SearchManager
        let success_msg = format!("Search pets with criteria - name: {:?}, breed: {:?}, location: {:?}, active_since: {:?}", 
            name, breed, location, active_since);
        let output = self.formatter.format_success_message(&success_msg);
        print!("{}", output);
        
        // TODO: Implement actual search functionality using SearchManager
        println!("Note: Full search functionality requires integration with SearchManager");
        
        Ok(())
    }
    
    async fn handle_search_devices(&self, name: Option<&str>, device_type: Option<&str>, min_battery: Option<u8>, online: Option<bool>, _token: &str) -> io::Result<()> {
        debug!("Executing search-devices command");
        
        // This is a simplified implementation - in a real scenario, you'd use SearchManager
        let success_msg = format!("Search devices with criteria - name: {:?}, type: {:?}, min_battery: {:?}, online: {:?}", 
            name, device_type, min_battery, online);
        let output = self.formatter.format_success_message(&success_msg);
        print!("{}", output);
        
        // TODO: Implement actual search functionality using SearchManager
        println!("Note: Full search functionality requires integration with SearchManager");
        
        Ok(())
    }
    
    async fn handle_batch(&self, operation: &str, targets: &[String], param: Option<&str>, _token: &str) -> io::Result<()> {
        debug!("Executing batch command with operation {} on targets {:?}", operation, targets);
        
        // Validate operation
        if let Err(e) = validate_batch_operation(operation) {
            let help = generate_help_examples("batch", &e);
            let output = self.formatter.format_error(&help);
            print!("{}", output);
            return Err(io::Error::new(io::ErrorKind::InvalidInput, e));
        }
        
        if targets.is_empty() {
            let error_msg = "At least one target must be specified";
            let help = generate_help_examples("batch", error_msg);
            let output = self.formatter.format_error(&help);
            print!("{}", output);
            return Err(io::Error::new(io::ErrorKind::InvalidInput, error_msg));
        }
        
        // Validate param for operations that require it
        match operation.to_lowercase().as_str() {
            "set-location" => {
                if param.is_none() {
                    let error_msg = "set-location operation requires --param with location (inside/outside)";
                    let help = generate_help_examples("batch", error_msg);
                    let output = self.formatter.format_error(&help);
                    print!("{}", output);
                    return Err(io::Error::new(io::ErrorKind::InvalidInput, error_msg));
                }
                if let Err(e) = parse_location(param.unwrap()) {
                    let help = generate_help_examples("batch", &e);
                    let output = self.formatter.format_error(&help);
                    print!("{}", output);
                    return Err(io::Error::new(io::ErrorKind::InvalidInput, e));
                }
            }
            _ => {}
        }
        
        // This is a simplified implementation - in a real scenario, you'd resolve targets and execute batch operations
        let success_msg = format!("Batch {} operation would be executed on {} targets with param: {:?}", 
            operation, targets.len(), param);
        let output = self.formatter.format_success_message(&success_msg);
        print!("{}", output);
        
        // TODO: Implement actual batch functionality using API client batch methods
        println!("Note: Full batch functionality requires resolving targets and executing operations");
        
        Ok(())
    }
}