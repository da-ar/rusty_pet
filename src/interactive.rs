use crate::api::client::{Client, CurfewTime};
use crate::cache::CacheManager;
use crate::cli::CliArgs;
use crate::config::UserPreferences;
use crate::data_validation::{PetDataValidator, DeviceDataValidator};
use crate::formatters::{OutputFormatter, create_formatter};
use crate::offline_manager::OfflineManager;
use crate::search::{SearchManager, PetSearchCriteria, DeviceSearchCriteria};
use crate::export::{ExportManager, ExportConfig, ExportFormat, DataType};
use crate::token;
use console::style;
use log::{debug, error, info, warn};
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Interactive mode handler for menu-driven interface
pub struct InteractiveMode {
    pub client: Client,
    formatter: Box<dyn OutputFormatter>,
    interrupt_flag: Arc<AtomicBool>,
    cache_manager: Option<Arc<CacheManager>>,
    #[allow(dead_code)] // Future functionality
    offline_manager: Option<Arc<OfflineManager>>,
}

#[derive(Debug, Clone)]
pub enum MenuAction {
    Status,
    List,
    SetPetLocation,
    Lock,
    LockIn,
    LockOut,
    Unlock,
    SetCurfew,
    SetPetIndoor,
    SetPetOutdoor,
    ViewHistory,
    Search,
    ExportData,
    BatchOperations,
    ManageConfig,
    Dashboard,
    Logout,
    Exit,
}

impl InteractiveMode {
    /// Create a new interactive mode handler
    pub fn new(client: Client, args: &CliArgs) -> Self {
        let formatter = create_formatter(args.is_json_output());
        let interrupt_flag = Arc::new(AtomicBool::new(false));
        
        // Set up signal handler for graceful interruption
        let flag_clone = interrupt_flag.clone();
        ctrlc::set_handler(move || {
            info!("Received interrupt signal (Ctrl+C)");
            flag_clone.store(true, Ordering::SeqCst);
        }).unwrap_or_else(|e| {
            error!("Failed to set up interrupt handler: {}", e);
        });
        
        Self {
            client,
            formatter,
            interrupt_flag,
            cache_manager: None,
            offline_manager: None,
        }
    }

    /// Create a new interactive mode handler with cache manager
    pub fn new_with_cache(client: Client, args: &CliArgs, cache_manager: Arc<CacheManager>) -> Self {
        let formatter = create_formatter(args.is_json_output());
        let interrupt_flag = Arc::new(AtomicBool::new(false));
        
        // Set up signal handler for graceful interruption
        let flag_clone = interrupt_flag.clone();
        ctrlc::set_handler(move || {
            info!("Received interrupt signal (Ctrl+C)");
            flag_clone.store(true, Ordering::SeqCst);
        }).unwrap_or_else(|e| {
            error!("Failed to set up interrupt handler: {}", e);
        });
        
        Self {
            client,
            formatter,
            interrupt_flag,
            cache_manager: Some(cache_manager),
            offline_manager: None,
        }
    }

    /// Create a new interactive mode handler with all managers
    pub fn new_with_managers(client: Client, args: &CliArgs, cache_manager: Arc<CacheManager>, offline_manager: Arc<OfflineManager>) -> Self {
        let formatter = create_formatter(args.is_json_output());
        let interrupt_flag = Arc::new(AtomicBool::new(false));
        
        // Set up signal handler for graceful interruption
        let flag_clone = interrupt_flag.clone();
        ctrlc::set_handler(move || {
            info!("Received interrupt signal (Ctrl+C)");
            flag_clone.store(true, Ordering::SeqCst);
        }).unwrap_or_else(|e| {
            error!("Failed to set up interrupt handler: {}", e);
        });
        
        Self {
            client,
            formatter,
            interrupt_flag,
            cache_manager: Some(cache_manager),
            offline_manager: Some(offline_manager),
        }
    }

    /// Run the interactive mode interface
    pub async fn run(&self, token: &mut String) -> io::Result<()> {
        cliclack::clear_screen()?;
        cliclack::intro(style(" RustyPet - Your SurePet CLI ").on_cyan().black())?;

        loop {
            // Check for interrupt signal
            if self.interrupt_flag.load(Ordering::SeqCst) {
                println!("\n🛑 Interrupt signal received.");
                let should_exit = cliclack::confirm("Do you want to exit RustyPet?")
                    .initial_value(true)
                    .interact()?;
                
                if should_exit {
                    println!("👋 Goodbye!");
                    break;
                } else {
                    // Reset the interrupt flag and continue
                    self.interrupt_flag.store(false, Ordering::SeqCst);
                    continue;
                }
            }

            match self.show_main_menu().await? {
                MenuAction::Exit => {
                    println!("👋 Goodbye!");
                    break;
                }
                MenuAction::Logout => {
                    self.handle_logout().await?;
                    break;
                }
                action => {
                    if let Err(e) = self.execute_with_auth_retry(action, token).await {
                        error!("Operation failed: {}", e);
                        
                        // Show user-friendly error message and continue to main menu
                        let error_output = self.formatter.format_error(&format!("Operation failed: {}", e));
                        print!("{}", error_output);
                        
                        // Ask if user wants to continue or exit
                        let should_continue = cliclack::confirm("Would you like to continue?")
                            .initial_value(true)
                            .interact()?;
                        
                        if !should_continue {
                            println!("👋 Goodbye!");
                            break;
                        }
                    }
                    
                    // Always return to main menu after operation completion
                    println!("\n📋 Returning to main menu...");
                }
            }
        }

        Ok(())
    }

    async fn show_main_menu(&self) -> io::Result<MenuAction> {
        let selection = cliclack::select("What would you like to do?")
            .initial_value("st")
            .item("st", "Status", "Check system status")
            .item("ls", "List Pets", "List all pets")
            .item("loc", "Set Pet Location", "Set a pet's location (inside/outside)")
            .item("lock", "Lock Flap", "Lock the pet flap completely")
            .item("lock_in", "Lock In", "Allow pets out but not in")
            .item("lock_out", "Lock Out", "Allow pets in but not out")
            .item("unlock", "Unlock Flap", "Allow free access through flap")
            .item("curfew", "Set Curfew", "Set curfew times for flap")
            .item("indoor", "Set Pet Indoor", "Mark pet as inside")
            .item("outdoor", "Set Pet Outdoor", "Mark pet as outside")
            .item("history", "View History", "View feeding, drinking, or activity history")
            .item("search", "Search", "Search pets or devices")
            .item("export", "Export Data", "Export data to CSV or JSON")
            .item("batch", "Batch Operations", "Perform operations on multiple pets/devices")
            .item("dashboard", "Dashboard Mode", "Live-updating dashboard view")
            .item("config", "Manage Configuration", "View and modify preferences")
            .item("logout", "Logout", "Clear saved authentication token")
            .item("exit", "Exit", "Exit the application")
            .interact()?;

        let action = match selection {
            "st" => MenuAction::Status,
            "ls" => MenuAction::List,
            "loc" => MenuAction::SetPetLocation,
            "lock" => MenuAction::Lock,
            "lock_in" => MenuAction::LockIn,
            "lock_out" => MenuAction::LockOut,
            "unlock" => MenuAction::Unlock,
            "curfew" => MenuAction::SetCurfew,
            "indoor" => MenuAction::SetPetIndoor,
            "outdoor" => MenuAction::SetPetOutdoor,
            "history" => MenuAction::ViewHistory,
            "search" => MenuAction::Search,
            "export" => MenuAction::ExportData,
            "batch" => MenuAction::BatchOperations,
            "dashboard" => MenuAction::Dashboard,
            "config" => MenuAction::ManageConfig,
            "logout" => MenuAction::Logout,
            "exit" => MenuAction::Exit,
            _ => {
                error!("Invalid menu selection: {}", selection);
                return Err(io::Error::new(io::ErrorKind::InvalidInput, "Invalid menu selection"));
            }
        };

        Ok(action)
    }

    /// Execute an operation with automatic authentication retry on failure
    async fn execute_with_auth_retry(&self, action: MenuAction, token: &mut String) -> io::Result<()> {
        // Try the operation with current token
        match self.execute_action(&action, token).await {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == io::ErrorKind::PermissionDenied => {
                // Token is invalid, try to login again
                match self.do_login().await {
                    Ok(new_token) => {
                        *token = new_token;
                        // Retry the operation with new token
                        self.execute_action(&action, token).await
                    }
                    Err(login_err) => {
                        error!("Failed to re-authenticate: {}", login_err);
                        Err(io::Error::new(
                            io::ErrorKind::Other,
                            format!("Re-authentication failed: {}", login_err)
                        ))
                    }
                }
            }
            Err(e) => {
                error!("Operation failed: {}", e);
                Err(e)
            }
        }
    }

    async fn execute_action(&self, action: &MenuAction, token: &str) -> io::Result<()> {
        match action {
            MenuAction::Status => self.handle_status(token).await,
            MenuAction::List => self.handle_list(token).await,
            MenuAction::SetPetLocation => self.handle_set_pet_location(token).await,
            MenuAction::Lock => self.handle_lock(token).await,
            MenuAction::LockIn => self.handle_lock_in(token).await,
            MenuAction::LockOut => self.handle_lock_out(token).await,
            MenuAction::Unlock => self.handle_unlock(token).await,
            MenuAction::SetCurfew => self.handle_set_curfew(token).await,
            MenuAction::SetPetIndoor => self.handle_set_pet_indoor(token).await,
            MenuAction::SetPetOutdoor => self.handle_set_pet_outdoor(token).await,
            MenuAction::ViewHistory => self.handle_view_history(token).await,
            MenuAction::Search => self.handle_search(token).await,
            MenuAction::ExportData => self.handle_export_data(token).await,
            MenuAction::BatchOperations => self.handle_batch_operations(token).await,
            MenuAction::ManageConfig => self.handle_manage_config().await,
            MenuAction::Dashboard => self.handle_dashboard(token).await,
            MenuAction::Logout | MenuAction::Exit => {
                // These are handled in the main loop
                Ok(())
            }
        }
    }

    async fn handle_status(&self, token: &str) -> io::Result<()> {
        debug!("Performing status operation");
        
        // Try to get cached devices first if cache manager is available
        let devices_response = if let Some(cache_manager) = &self.cache_manager {
            if let Some(cached_devices) = cache_manager.get_devices().await {
                debug!("Using cached devices data");
                crate::api::client::DevicesResponse {
                    data: cached_devices.data,
                }
            } else {
                debug!("No cached devices, fetching from API");
                match self.client.get_devices(token).await {
                    Ok(response) => {
                        // Cache the response for future use
                        if let Err(e) = cache_manager.cache_devices(response.data.clone()).await {
                            warn!("Failed to cache devices data: {}", e);
                        }
                        response
                    }
                    Err(e) if e.status() == Some(reqwest::StatusCode::UNAUTHORIZED) => {
                        let _ = token::delete_token();
                        return Err(io::Error::new(io::ErrorKind::PermissionDenied, "Authentication failed"));
                    }
                    Err(e) => {
                        return Err(io::Error::new(io::ErrorKind::Other, format!("API error: {}", e)));
                    }
                }
            }
        } else {
            // No cache manager, fetch directly from API
            match self.client.get_devices(token).await {
                Ok(response) => response,
                Err(e) if e.status() == Some(reqwest::StatusCode::UNAUTHORIZED) => {
                    let _ = token::delete_token();
                    return Err(io::Error::new(io::ErrorKind::PermissionDenied, "Authentication failed"));
                }
                Err(e) => {
                    return Err(io::Error::new(io::ErrorKind::Other, format!("API error: {}", e)));
                }
            }
        };

        // Perform data completeness checks for each device (Requirement 4.1)
        let mut device_reports = Vec::new();
        for device in &devices_response.data {
            let status_report = DeviceDataValidator::validate_device_status(device);
            // For now, create a dummy history report since we don't have history data in status view
            let history_report = crate::data_validation::DataCompletenessReport::new();
            device_reports.push((device.name.clone(), status_report, history_report));
            
            // Log any warnings found during validation
            if !device_reports.last().unwrap().1.warnings.is_empty() {
                warn!("Data completeness issues found for device: {}", device.name);
            }
        }
        
        let output = self.formatter.format_devices(&devices_response);
        print!("{}", output);
        
        // Display data completeness summary if there are issues
        self.display_device_completeness_summary(&device_reports);
        
        Ok(())
    }

    async fn handle_list(&self, token: &str) -> io::Result<()> {
        debug!("Performing list operation");
        
        // Try to get cached pets first if cache manager is available
        let pets_response = if let Some(cache_manager) = &self.cache_manager {
            if let Some(cached_pets) = cache_manager.get_pets().await {
                debug!("Using cached pets data");
                crate::api::client::PetsResponse {
                    data: cached_pets.data,
                }
            } else {
                debug!("No cached pets, fetching from API");
                match self.client.get_pets(token).await {
                    Ok(response) => {
                        // Cache the response for future use
                        if let Err(e) = cache_manager.cache_pets(response.data.clone()).await {
                            warn!("Failed to cache pets data: {}", e);
                        }
                        response
                    }
                    Err(e) if e.status() == Some(reqwest::StatusCode::UNAUTHORIZED) => {
                        let _ = token::delete_token();
                        return Err(io::Error::new(io::ErrorKind::PermissionDenied, "Authentication failed"));
                    }
                    Err(e) => {
                        return Err(io::Error::new(io::ErrorKind::Other, format!("API error: {}", e)));
                    }
                }
            }
        } else {
            // No cache manager, fetch directly from API
            match self.client.get_pets(token).await {
                Ok(response) => response,
                Err(e) if e.status() == Some(reqwest::StatusCode::UNAUTHORIZED) => {
                    let _ = token::delete_token();
                    return Err(io::Error::new(io::ErrorKind::PermissionDenied, "Authentication failed"));
                }
                Err(e) => {
                    return Err(io::Error::new(io::ErrorKind::Other, format!("API error: {}", e)));
                }
            }
        };

        // Perform data completeness checks for each pet (Requirement 3.3)
        let mut pet_reports = Vec::new();
        for pet in &pets_response.data {
            let details_report = PetDataValidator::validate_pet_details(pet);
            // For now, we don't have historical data in the list view, so create empty activity report
            let activity_report = PetDataValidator::validate_pet_activity_completeness(pet, None, None, None);
            pet_reports.push((pet.name.clone(), details_report, activity_report));
            
            // Log any warnings found during validation
            if !pet_reports.last().unwrap().1.warnings.is_empty() || !pet_reports.last().unwrap().2.warnings.is_empty() {
                warn!("Data completeness issues found for pet: {}", pet.name);
            }
        }
        
        let output = self.formatter.format_pets(&pets_response);
        print!("{}", output);
        
        // Display data completeness summary if there are issues
        self.display_pet_completeness_summary(&pet_reports);
        
        Ok(())
    }

    async fn handle_set_pet_location(&self, token: &str) -> io::Result<()> {
        debug!("Performing set pet location operation");
        
        // First get the list of pets
        let pets_response = match self.client.get_pets(token).await {
            Ok(response) => response,
            Err(e) if e.status() == Some(reqwest::StatusCode::UNAUTHORIZED) => {
                let _ = token::delete_token();
                return Err(io::Error::new(io::ErrorKind::PermissionDenied, "Authentication failed"));
            }
            Err(e) => {
                return Err(io::Error::new(io::ErrorKind::Other, format!("Failed to get pets: {}", e)));
            }
        };

        if pets_response.data.is_empty() {
            println!("No pets found in your account.");
            return Ok(());
        }

        // Let user select a pet
        let mut select = cliclack::select::<String>("Select a pet:");
        for pet in &pets_response.data {
            select = select.item(pet.id.to_string(), pet.name.as_str(), "");
        }
        let selected_pet_id: String = select.interact()?;
        let pet_id: u32 = selected_pet_id.parse().unwrap();

        // Let user select location
        let location = cliclack::select("Select location:")
            .item("1", "Inside", "Pet is inside")
            .item("2", "Outside", "Pet is outside")
            .interact()?;

        let location_id: u32 = location.parse().unwrap();

        match self.client.set_pet_location(token, pet_id, location_id).await {
            Ok(()) => {
                // Invalidate pets cache since location has changed
                if let Some(cache_manager) = &self.cache_manager {
                    if let Err(e) = cache_manager.clear_all().await {
                        warn!("Failed to clear cache after location update: {}", e);
                    } else {
                        debug!("Cache cleared after successful pet location update");
                    }
                }
                
                let location_name = if location_id == 1 { "inside" } else { "outside" };
                let success_msg = format!("Pet location set to {}", location_name);
                let output = self.formatter.format_success_message(&success_msg);
                print!("{}", output);
                Ok(())
            }
            Err(e) if e.status() == Some(reqwest::StatusCode::UNAUTHORIZED) => {
                let _ = token::delete_token();
                Err(io::Error::new(io::ErrorKind::PermissionDenied, "Authentication failed"))
            }
            Err(e) => {
                Err(io::Error::new(io::ErrorKind::Other, format!("Failed to set pet location: {}", e)))
            }
        }
    }

    async fn get_device_selection(&self, token: &str) -> io::Result<u32> {
        let devices_response = match self.client.get_devices(token).await {
            Ok(response) => response,
            Err(e) if e.status() == Some(reqwest::StatusCode::UNAUTHORIZED) => {
                let _ = token::delete_token();
                return Err(io::Error::new(io::ErrorKind::PermissionDenied, "Authentication failed"));
            }
            Err(e) => {
                return Err(io::Error::new(io::ErrorKind::Other, format!("Failed to get devices: {}", e)));
            }
        };

        if devices_response.data.is_empty() {
            return Err(io::Error::new(io::ErrorKind::NotFound, "No devices found in your account"));
        }

        let mut select = cliclack::select::<String>("Select a device:");
        for device in &devices_response.data {
            select = select.item(device.id.to_string(), device.name.as_str(), &format!("Serial: {}", device.serial_number));
        }
        let selected_device_id: String = select.interact()?;
        Ok(selected_device_id.parse().unwrap())
    }

    async fn handle_lock(&self, token: &str) -> io::Result<()> {
        debug!("Performing lock operation");
        
        let device_id = self.get_device_selection(token).await?;

        match self.client.lock(token, device_id).await {
            Ok(()) => {
                let success_msg = "Device locked (no access allowed)";
                let output = self.formatter.format_success_message(success_msg);
                print!("{}", output);
                Ok(())
            }
            Err(e) if e.status() == Some(reqwest::StatusCode::UNAUTHORIZED) => {
                let _ = token::delete_token();
                Err(io::Error::new(io::ErrorKind::PermissionDenied, "Authentication failed"))
            }
            Err(e) => {
                Err(io::Error::new(io::ErrorKind::Other, format!("Failed to lock device: {}", e)))
            }
        }
    }

    async fn handle_lock_in(&self, token: &str) -> io::Result<()> {
        debug!("Performing lock in operation");
        
        let device_id = self.get_device_selection(token).await?;

        match self.client.lock_in(token, device_id).await {
            Ok(()) => {
                let success_msg = "Device set to 'Keep In' mode (pets can exit but not enter)";
                let output = self.formatter.format_success_message(success_msg);
                print!("{}", output);
                Ok(())
            }
            Err(e) if e.status() == Some(reqwest::StatusCode::UNAUTHORIZED) => {
                let _ = token::delete_token();
                Err(io::Error::new(io::ErrorKind::PermissionDenied, "Authentication failed"))
            }
            Err(e) => {
                Err(io::Error::new(io::ErrorKind::Other, format!("Failed to set lock in mode: {}", e)))
            }
        }
    }

    async fn handle_lock_out(&self, token: &str) -> io::Result<()> {
        debug!("Performing lock out operation");
        
        let device_id = self.get_device_selection(token).await?;

        match self.client.lock_out(token, device_id).await {
            Ok(()) => {
                let success_msg = "Device set to 'Keep Out' mode (pets can enter but not exit)";
                let output = self.formatter.format_success_message(success_msg);
                print!("{}", output);
                Ok(())
            }
            Err(e) if e.status() == Some(reqwest::StatusCode::UNAUTHORIZED) => {
                let _ = token::delete_token();
                Err(io::Error::new(io::ErrorKind::PermissionDenied, "Authentication failed"))
            }
            Err(e) => {
                Err(io::Error::new(io::ErrorKind::Other, format!("Failed to set lock out mode: {}", e)))
            }
        }
    }

    async fn handle_unlock(&self, token: &str) -> io::Result<()> {
        debug!("Performing unlock operation");
        
        let device_id = self.get_device_selection(token).await?;

        match self.client.unlock(token, device_id).await {
            Ok(()) => {
                let success_msg = "Device unlocked (free access allowed)";
                let output = self.formatter.format_success_message(success_msg);
                print!("{}", output);
                Ok(())
            }
            Err(e) if e.status() == Some(reqwest::StatusCode::UNAUTHORIZED) => {
                let _ = token::delete_token();
                Err(io::Error::new(io::ErrorKind::PermissionDenied, "Authentication failed"))
            }
            Err(e) => {
                Err(io::Error::new(io::ErrorKind::Other, format!("Failed to unlock device: {}", e)))
            }
        }
    }

    async fn handle_set_curfew(&self, token: &str) -> io::Result<()> {
        debug!("Performing set curfew operation");
        
        let device_id = self.get_device_selection(token).await?;

        let enabled = cliclack::confirm("Enable curfew?").interact()?;
        
        if !enabled {
            // Send empty curfew to disable
            let curfew_times = vec![];
            match self.client.set_curfew(token, device_id, curfew_times).await {
                Ok(()) => {
                    let success_msg = "Curfew disabled";
                    let output = self.formatter.format_success_message(success_msg);
                    print!("{}", output);
                    Ok(())
                }
                Err(e) if e.status() == Some(reqwest::StatusCode::UNAUTHORIZED) => {
                    let _ = token::delete_token();
                    Err(io::Error::new(io::ErrorKind::PermissionDenied, "Authentication failed"))
                }
                Err(e) => {
                    Err(io::Error::new(io::ErrorKind::Other, format!("Failed to set curfew: {}", e)))
                }
            }
        } else {
            let lock_time: String = cliclack::input("Enter lock time (HH:MM format, e.g., 22:00):")
                .placeholder("22:00")
                .interact()?;
            
            let unlock_time: String = cliclack::input("Enter unlock time (HH:MM format, e.g., 06:00):")
                .placeholder("06:00")
                .interact()?;

            let curfew_times = vec![CurfewTime {
                enabled: true,
                lock_time,
                unlock_time,
            }];

            match self.client.set_curfew(token, device_id, curfew_times).await {
                Ok(()) => {
                    let success_msg = "Curfew set successfully";
                    let output = self.formatter.format_success_message(success_msg);
                    print!("{}", output);
                    Ok(())
                }
                Err(e) if e.status() == Some(reqwest::StatusCode::UNAUTHORIZED) => {
                    let _ = token::delete_token();
                    Err(io::Error::new(io::ErrorKind::PermissionDenied, "Authentication failed"))
                }
                Err(e) => {
                    Err(io::Error::new(io::ErrorKind::Other, format!("Failed to set curfew: {}", e)))
                }
            }
        }
    }

    async fn handle_set_pet_indoor(&self, token: &str) -> io::Result<()> {
        debug!("Performing set pet indoor operation");
        
        let pets_response = match self.client.get_pets(token).await {
            Ok(response) => response,
            Err(e) if e.status() == Some(reqwest::StatusCode::UNAUTHORIZED) => {
                let _ = token::delete_token();
                return Err(io::Error::new(io::ErrorKind::PermissionDenied, "Authentication failed"));
            }
            Err(e) => {
                return Err(io::Error::new(io::ErrorKind::Other, format!("Failed to get pets: {}", e)));
            }
        };

        if pets_response.data.is_empty() {
            println!("No pets found in your account.");
            return Ok(());
        }

        let mut select = cliclack::select::<String>("Select a pet to mark as indoor:");
        for pet in &pets_response.data {
            select = select.item(pet.id.to_string(), pet.name.as_str(), "");
        }
        let selected_pet_id: String = select.interact()?;
        let pet_id: u32 = selected_pet_id.parse().unwrap();

        match self.client.set_pet_indoor_mode(token, pet_id).await {
            Ok(()) => {
                let success_msg = "Pet marked as indoor";
                let output = self.formatter.format_success_message(success_msg);
                print!("{}", output);
                Ok(())
            }
            Err(e) if e.status() == Some(reqwest::StatusCode::UNAUTHORIZED) => {
                let _ = token::delete_token();
                Err(io::Error::new(io::ErrorKind::PermissionDenied, "Authentication failed"))
            }
            Err(e) => {
                Err(io::Error::new(io::ErrorKind::Other, format!("Failed to set pet indoor: {}", e)))
            }
        }
    }

    async fn handle_set_pet_outdoor(&self, token: &str) -> io::Result<()> {
        debug!("Performing set pet outdoor operation");
        
        let pets_response = match self.client.get_pets(token).await {
            Ok(response) => response,
            Err(e) if e.status() == Some(reqwest::StatusCode::UNAUTHORIZED) => {
                let _ = token::delete_token();
                return Err(io::Error::new(io::ErrorKind::PermissionDenied, "Authentication failed"));
            }
            Err(e) => {
                return Err(io::Error::new(io::ErrorKind::Other, format!("Failed to get pets: {}", e)));
            }
        };

        if pets_response.data.is_empty() {
            println!("No pets found in your account.");
            return Ok(());
        }

        let mut select = cliclack::select::<String>("Select a pet to mark as outdoor:");
        for pet in &pets_response.data {
            select = select.item(pet.id.to_string(), pet.name.as_str(), "");
        }
        let selected_pet_id: String = select.interact()?;
        let pet_id: u32 = selected_pet_id.parse().unwrap();

        match self.client.set_pet_outdoor_mode(token, pet_id).await {
            Ok(()) => {
                let success_msg = "Pet marked as outdoor";
                let output = self.formatter.format_success_message(success_msg);
                print!("{}", output);
                Ok(())
            }
            Err(e) if e.status() == Some(reqwest::StatusCode::UNAUTHORIZED) => {
                let _ = token::delete_token();
                Err(io::Error::new(io::ErrorKind::PermissionDenied, "Authentication failed"))
            }
            Err(e) => {
                Err(io::Error::new(io::ErrorKind::Other, format!("Failed to set pet outdoor: {}", e)))
            }
        }
    }

    async fn handle_logout(&self) -> io::Result<()> {
        debug!("Performing logout operation");
        
        match token::delete_token() {
            Ok(_) => {
                let success_msg = "Successfully logged out. Authentication token has been cleared.";
                let output = self.formatter.format_success_message(success_msg);
                print!("{}", output);
                Ok(())
            }
            Err(e) => {
                error!("Failed to clear authentication token: {}", e);
                let error_msg = "Warning: Could not clear saved authentication token.";
                let output = self.formatter.format_error(error_msg);
                print!("{}", output);
                Err(io::Error::new(io::ErrorKind::Other, format!("Failed to clear token: {}", e)))
            }
        }
    }

    async fn do_login(&self) -> io::Result<String> {
        debug!("Prompting for login");
        let username: String = cliclack::input("Provide your username").interact()?;
        let password = cliclack::password("Provide your password")
            .mask('▪')
            .interact()?;

        let resp = self.client
            .login(&username, &password)
            .await
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Login failed: {}", e)))?;

        let new_token = resp.data.token;

        // Save the new token to file
        if let Err(e) = token::save_token(&new_token) {
            error!("Failed to save token to file: {}", e);
            // Continue anyway, just warn the user
            println!("Warning: Could not save authentication token. You'll need to login again next time.");
        } else {
            println!("✓ Authentication token saved. You won't need to login again until it expires.");
        }

        Ok(new_token)
    }

    async fn handle_manage_config(&self) -> io::Result<()> {
        debug!("Performing manage configuration operation");
        
        loop {
            let selection = cliclack::select("Configuration Management")
                .item("view", "View Current Settings", "Display current preferences")
                .item("edit", "Edit Preferences", "Modify configuration settings")
                .item("reset", "Reset to Defaults", "Reset all settings to default values")
                .item("back", "Back to Main Menu", "Return to main menu")
                .interact()?;

            match selection {
                "view" => {
                    self.show_current_preferences().await?;
                }
                "edit" => {
                    self.edit_preferences().await?;
                }
                "reset" => {
                    self.reset_preferences().await?;
                }
                "back" => break,
                _ => {
                    error!("Invalid configuration menu selection: {}", selection);
                }
            }
        }

        Ok(())
    }

    async fn show_current_preferences(&self) -> io::Result<()> {
        match UserPreferences::load() {
            Ok(prefs) => {
                println!("\n📋 Current Configuration:");
                println!("  Date Format: {}", prefs.date_format);
                println!("  Time Format: {}", prefs.time_format);
                println!("  Timezone: {}", prefs.timezone);
                println!("  Default Date Range: {:?}", prefs.default_date_range);
                println!("  Cache TTL: {} hours", prefs.cache_ttl_hours);
                println!("  Auto Refresh: {} seconds", prefs.auto_refresh_interval.map_or("disabled".to_string(), |i| i.to_string()));
                println!("  Show Raw JSON: {}", prefs.show_raw_json);
                println!("  Use Colors: {}", prefs.use_colors);
                println!("  Compact Mode: {}", prefs.compact_mode);
                println!("  Max History Items: {}", prefs.max_history_items);
                println!();
                
                cliclack::note("Press Enter to continue", "")?;
                let _: String = cliclack::input("").interact()?;
                Ok(())
            }
            Err(e) => {
                let error_msg = format!("Failed to load preferences: {}", e.user_message());
                let output = self.formatter.format_error(&error_msg);
                print!("{}", output);
                Err(io::Error::new(io::ErrorKind::Other, error_msg))
            }
        }
    }

    async fn edit_preferences(&self) -> io::Result<()> {
        let mut prefs = match UserPreferences::load() {
            Ok(p) => p,
            Err(e) => {
                println!("Warning: Could not load existing preferences ({}), using defaults", e.user_message());
                UserPreferences::default()
            }
        };

        println!("\n✏️  Edit Preferences (press Enter to keep current value):");

        // Date format
        let new_date_format: String = cliclack::input("Date format:")
            .placeholder(&prefs.date_format)
            .default_input(&prefs.date_format)
            .interact()?;
        prefs.date_format = new_date_format;

        // Time format
        let new_time_format: String = cliclack::input("Time format:")
            .placeholder(&prefs.time_format)
            .default_input(&prefs.time_format)
            .interact()?;
        prefs.time_format = new_time_format;

        // Timezone
        let new_timezone: String = cliclack::input("Timezone:")
            .placeholder(&prefs.timezone)
            .default_input(&prefs.timezone)
            .interact()?;
        prefs.timezone = new_timezone;

        // Cache TTL
        let new_cache_ttl: String = cliclack::input("Cache TTL (hours):")
            .placeholder(&prefs.cache_ttl_hours.to_string())
            .default_input(&prefs.cache_ttl_hours.to_string())
            .interact()?;
        if let Ok(ttl) = new_cache_ttl.parse::<u64>() {
            prefs.cache_ttl_hours = ttl;
        }

        // Auto refresh interval
        let current_refresh = prefs.auto_refresh_interval.map_or("disabled".to_string(), |i| i.to_string());
        let new_refresh: String = cliclack::input("Auto refresh interval (seconds, or 'disabled'):")
            .placeholder(&current_refresh)
            .default_input(&current_refresh)
            .interact()?;
        prefs.auto_refresh_interval = if new_refresh.to_lowercase() == "disabled" {
            None
        } else {
            new_refresh.parse::<u64>().ok()
        };

        // Use colors
        prefs.use_colors = cliclack::confirm("Use colors in output?")
            .initial_value(prefs.use_colors)
            .interact()?;

        // Compact mode
        prefs.compact_mode = cliclack::confirm("Use compact display mode?")
            .initial_value(prefs.compact_mode)
            .interact()?;

        // Max history items
        let new_max_history: String = cliclack::input("Maximum history items:")
            .placeholder(&prefs.max_history_items.to_string())
            .default_input(&prefs.max_history_items.to_string())
            .interact()?;
        if let Ok(max) = new_max_history.parse::<usize>() {
            prefs.max_history_items = max;
        }

        // Validate and save
        match prefs.validate() {
            Ok(feedback) => {
                println!("\n✅ Validation successful:");
                for msg in feedback {
                    println!("  • {}", msg);
                }
                
                match prefs.save() {
                    Ok(_) => {
                        let success_msg = "Preferences saved successfully!";
                        let output = self.formatter.format_success_message(success_msg);
                        print!("{}", output);
                        Ok(())
                    }
                    Err(e) => {
                        let error_msg = format!("Failed to save preferences: {}", e.user_message());
                        let output = self.formatter.format_error(&error_msg);
                        print!("{}", output);
                        Err(io::Error::new(io::ErrorKind::Other, error_msg))
                    }
                }
            }
            Err(e) => {
                let error_msg = format!("Invalid preferences: {}", e.user_message());
                let output = self.formatter.format_error(&error_msg);
                print!("{}", output);
                Err(io::Error::new(io::ErrorKind::InvalidInput, error_msg))
            }
        }
    }

    async fn reset_preferences(&self) -> io::Result<()> {
        let confirmed = cliclack::confirm("Are you sure you want to reset all preferences to default values?")
            .initial_value(false)
            .interact()?;

        if !confirmed {
            println!("Reset cancelled.");
            return Ok(());
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

    /// Handle dashboard mode with live updates
    async fn handle_dashboard(&self, token: &str) -> io::Result<()> {
        debug!("Entering dashboard mode");
        
        // Load user preferences to get refresh interval
        let prefs = UserPreferences::load().unwrap_or_default();
        let refresh_interval = prefs.auto_refresh_interval.unwrap_or(30); // Default 30 seconds
        
        println!("🖥️  Entering Dashboard Mode");
        println!("📊 Refresh interval: {} seconds", refresh_interval);
        println!("💡 Press Ctrl+C to exit dashboard and return to main menu\n");
        
        // Store previous data for change highlighting
        let mut previous_pets: Option<Vec<crate::api::client::Pet>> = None;
        let mut previous_devices: Option<Vec<crate::api::client::Device>> = None;
        
        // Reset interrupt flag for dashboard-specific handling
        self.interrupt_flag.store(false, Ordering::SeqCst);
        
        loop {
            // Check for interrupt signal to exit dashboard
            if self.interrupt_flag.load(Ordering::SeqCst) {
                println!("\n🛑 Exiting dashboard mode...");
                self.interrupt_flag.store(false, Ordering::SeqCst);
                break;
            }
            
            // Clear screen and show header
            cliclack::clear_screen()?;
            println!("🖥️  {} Dashboard Mode - Live View", 
                style("RustyPet").cyan().bold());
            println!("⏰ Last updated: {}", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"));
            println!("🔄 Auto-refresh: {} seconds | Press Ctrl+C to exit\n", refresh_interval);
            
            // Fetch current data
            let current_pets = match self.client.get_pets(token).await {
                Ok(response) => Some(response.data),
                Err(e) => {
                    println!("❌ Failed to fetch pets: {}", e);
                    None
                }
            };
            
            let current_devices = match self.client.get_devices(token).await {
                Ok(response) => Some(response.data),
                Err(e) => {
                    println!("❌ Failed to fetch devices: {}", e);
                    None
                }
            };
            
            // Display pets with change highlighting
            if let Some(ref pets) = current_pets {
                println!("🐾 {} Pets:", style("PETS").green().bold());
                for pet in pets {
                    let mut status_line = format!("  • {} (ID: {})", pet.name, pet.id);
                    
                    // Check for changes if we have previous data
                    if let Some(ref prev_pets) = previous_pets {
                        if let Some(prev_pet) = prev_pets.iter().find(|p| p.id == pet.id) {
                            // Highlight if location changed
                            let prev_location = prev_pet.position.as_ref().and_then(|p| p.location);
                            let curr_location = pet.position.as_ref().and_then(|p| p.location);
                            if prev_location != curr_location {
                                status_line = format!("{} {}", 
                                    status_line, 
                                    style("🔄 LOCATION CHANGED").yellow().bold()
                                );
                            }
                        } else {
                            // New pet
                            status_line = format!("{} {}", 
                                status_line, 
                                style("✨ NEW").green().bold()
                            );
                        }
                    }
                    
                    println!("{}", status_line);
                    
                    // Display location information
                    if let Some(position) = &pet.position {
                        println!("    Location: {} | Since: {}", 
                            match position.location {
                                Some(1) => "🏠 Inside",
                                Some(2) => "🌳 Outside", 
                                _ => "❓ Unknown"
                            },
                            position.since
                        );
                    } else {
                        println!("    Location: ❓ Unknown | Since: N/A");
                    }
                }
                println!();
            }
            
            // Display devices with change highlighting
            if let Some(ref devices) = current_devices {
                println!("🚪 {} Devices:", style("DEVICES").blue().bold());
                for device in devices {
                    let mut status_line = format!("  • {} (ID: {})", device.name, device.id);
                    
                    // Check for changes if we have previous data
                    if let Some(ref prev_devices) = previous_devices {
                        if let Some(prev_device) = prev_devices.iter().find(|d| d.id == device.id) {
                            // Highlight if status changed
                            let prev_mode = prev_device.status.as_ref()
                                .and_then(|s| s.locking.as_ref())
                                .map(|l| l.mode);
                            let curr_mode = device.status.as_ref()
                                .and_then(|s| s.locking.as_ref())
                                .map(|l| l.mode);
                            if prev_mode != curr_mode {
                                status_line = format!("{} {}", 
                                    status_line, 
                                    style("🔄 STATUS CHANGED").yellow().bold()
                                );
                            }
                            // Highlight if battery changed significantly
                            let prev_battery = prev_device.status.as_ref().and_then(|s| s.battery);
                            let curr_battery = device.status.as_ref().and_then(|s| s.battery);
                            if let (Some(prev_battery), Some(curr_battery)) = (prev_battery, curr_battery) {
                                let battery_diff = (curr_battery - prev_battery).abs();
                                if battery_diff > 5.0 {
                                    status_line = format!("{} {}", 
                                        status_line, 
                                        style("🔋 BATTERY CHANGED").yellow().bold()
                                    );
                                }
                            }
                        } else {
                            // New device
                            status_line = format!("{} {}", 
                                status_line, 
                                style("✨ NEW").green().bold()
                            );
                        }
                    }
                    
                    println!("{}", status_line);
                    
                    // Display device status
                    let lock_status = device.status.as_ref()
                        .and_then(|s| s.locking.as_ref())
                        .map(|l| l.mode)
                        .unwrap_or(0);
                    let battery_info = device.status.as_ref()
                        .and_then(|s| s.battery)
                        .map(|b| format!("{}% 🔋", (b * 10.0) as u8)) // Convert API value (0-10) to percentage (0-100)
                        .unwrap_or_else(|| "N/A".to_string());
                    
                    println!("    Status: {} | Battery: {}", 
                        match lock_status {
                            0 => "🔓 Unlocked",
                            1 => "🔒 Locked",
                            2 => "🔒 Keep In",
                            3 => "🔒 Keep Out",
                            _ => "❓ Unknown"
                        },
                        battery_info
                    );
                    
                    // Show alerts for low battery or connectivity issues
                    if let Some(status) = &device.status {
                        if let Some(battery) = status.battery {
                            let battery_percentage = battery * 10.0; // Convert API value (0-10) to percentage (0-100)
                            if battery_percentage < 20.0 {
                                println!("    {} Low battery warning!", 
                                    style("⚠️").red().bold()
                                );
                            }
                        }
                        
                        if let Some(online) = status.online {
                            if !online {
                                println!("    {} Device offline!", 
                                    style("🔴").red().bold()
                                );
                            }
                        }
                    }
                }
                println!();
            }
            
            // Show quick actions menu
            println!("🎮 {} Quick Actions:", style("AVAILABLE").cyan().bold());
            println!("  • Press Ctrl+C to exit dashboard");
            println!("  • Dashboard will auto-refresh in {} seconds", refresh_interval);
            
            // Store current data as previous for next iteration
            previous_pets = current_pets;
            previous_devices = current_devices;
            
            // Wait for refresh interval or interrupt
            for _ in 0..refresh_interval {
                if self.interrupt_flag.load(Ordering::SeqCst) {
                    break;
                }
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        }
        
        println!("📋 Returning to main menu...");
        Ok(())
    }
    
    async fn handle_view_history(&self, token: &str) -> io::Result<()> {
        debug!("Performing view history operation");
        
        loop {
            let history_type = cliclack::select("Select history type:")
                .item("feeding", "Feeding History", "View feeding events")
                .item("drinking", "Drinking History", "View drinking events")
                .item("activity", "Activity History", "View movement events")
                .item("back", "Back to Main Menu", "Return to main menu")
                .interact()?;

            match history_type {
                "feeding" => {
                    if let Err(e) = self.handle_feeding_history(token).await {
                        let error_output = self.formatter.format_error(&format!("Failed to get feeding history: {}", e));
                        print!("{}", error_output);
                    }
                }
                "drinking" => {
                    if let Err(e) = self.handle_drinking_history(token).await {
                        let error_output = self.formatter.format_error(&format!("Failed to get drinking history: {}", e));
                        print!("{}", error_output);
                    }
                }
                "activity" => {
                    if let Err(e) = self.handle_activity_history(token).await {
                        let error_output = self.formatter.format_error(&format!("Failed to get activity history: {}", e));
                        print!("{}", error_output);
                    }
                }
                "back" => break,
                _ => {
                    error!("Invalid history type selection: {}", history_type);
                }
            }
        }

        Ok(())
    }
    
    async fn handle_feeding_history(&self, token: &str) -> io::Result<()> {
        // Get pet selection
        let pets_response = self.client.get_pets(token).await
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to get pets: {}", e)))?;

        if pets_response.data.is_empty() {
            println!("No pets found in your account.");
            return Ok(());
        }

        let mut select = cliclack::select::<String>("Select a pet:");
        for pet in &pets_response.data {
            select = select.item(pet.id.to_string(), pet.name.as_str(), "");
        }
        let selected_pet_id: String = select.interact()?;
        let pet_id: u32 = selected_pet_id.parse().unwrap();

        // Get date range
        let range = cliclack::select("Select date range:")
            .item("today", "Today", "Today's feeding events")
            .item("week", "This Week", "Past 7 days")
            .item("month", "This Month", "Past 30 days")
            .interact()?;

        let date_range = crate::cli::parse_date_range(range)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

        // Try to get cached feeding history first if cache manager is available
        let history = if let Some(cache_manager) = &self.cache_manager {
            if let Some(cached_history) = cache_manager.get_feeding_history(pet_id, &date_range).await {
                debug!("Using cached feeding history for pet {}", pet_id);
                cached_history.data
            } else {
                debug!("No cached feeding history, fetching from API");
                match self.client.get_feeding_history(token, pet_id, date_range.clone()).await {
                    Ok(history) => {
                        // Cache the response for future use
                        if let Err(e) = cache_manager.cache_feeding_history(history.clone(), &date_range).await {
                            warn!("Failed to cache feeding history: {}", e);
                        }
                        history
                    }
                    Err(e) => return Err(io::Error::new(io::ErrorKind::Other, format!("API error: {}", e)))
                }
            }
        } else {
            // No cache manager, fetch directly from API
            match self.client.get_feeding_history(token, pet_id, date_range.clone()).await {
                Ok(history) => history,
                Err(e) => return Err(io::Error::new(io::ErrorKind::Other, format!("API error: {}", e)))
            }
        };

        let output = self.formatter.format_feeding_history(&history);
        print!("{}", output);
        Ok(())
    }
    
    async fn handle_drinking_history(&self, token: &str) -> io::Result<()> {
        // Get pet selection
        let pets_response = self.client.get_pets(token).await
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to get pets: {}", e)))?;

        if pets_response.data.is_empty() {
            println!("No pets found in your account.");
            return Ok(());
        }

        let mut select = cliclack::select::<String>("Select a pet:");
        for pet in &pets_response.data {
            select = select.item(pet.id.to_string(), pet.name.as_str(), "");
        }
        let selected_pet_id: String = select.interact()?;
        let pet_id: u32 = selected_pet_id.parse().unwrap();

        // Get date range
        let range = cliclack::select("Select date range:")
            .item("today", "Today", "Today's drinking events")
            .item("week", "This Week", "Past 7 days")
            .item("month", "This Month", "Past 30 days")
            .interact()?;

        let date_range = crate::cli::parse_date_range(range)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

        // Try to get cached drinking history first if cache manager is available
        let history = if let Some(cache_manager) = &self.cache_manager {
            if let Some(cached_history) = cache_manager.get_drinking_history(pet_id, &date_range).await {
                debug!("Using cached drinking history for pet {}", pet_id);
                cached_history.data
            } else {
                debug!("No cached drinking history, fetching from API");
                match self.client.get_drinking_history(token, pet_id, date_range.clone()).await {
                    Ok(history) => {
                        // Cache the response for future use
                        if let Err(e) = cache_manager.cache_drinking_history(history.clone(), &date_range).await {
                            warn!("Failed to cache drinking history: {}", e);
                        }
                        history
                    }
                    Err(e) => return Err(io::Error::new(io::ErrorKind::Other, format!("API error: {}", e)))
                }
            }
        } else {
            // No cache manager, fetch directly from API
            match self.client.get_drinking_history(token, pet_id, date_range.clone()).await {
                Ok(history) => history,
                Err(e) => return Err(io::Error::new(io::ErrorKind::Other, format!("API error: {}", e)))
            }
        };

        let output = self.formatter.format_drinking_history(&history);
        print!("{}", output);
        Ok(())
    }
    
    async fn handle_activity_history(&self, token: &str) -> io::Result<()> {
        // Get pet selection
        let pets_response = self.client.get_pets(token).await
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to get pets: {}", e)))?;

        if pets_response.data.is_empty() {
            println!("No pets found in your account.");
            return Ok(());
        }

        let mut select = cliclack::select::<String>("Select a pet:");
        for pet in &pets_response.data {
            select = select.item(pet.id.to_string(), pet.name.as_str(), "");
        }
        let selected_pet_id: String = select.interact()?;
        let pet_id: u32 = selected_pet_id.parse().unwrap();

        // Get date range
        let range = cliclack::select("Select date range:")
            .item("today", "Today", "Today's activity events")
            .item("week", "This Week", "Past 7 days")
            .item("month", "This Month", "Past 30 days")
            .interact()?;

        let date_range = crate::cli::parse_date_range(range)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

        // Try to get cached activity history first if cache manager is available
        let history = if let Some(cache_manager) = &self.cache_manager {
            if let Some(cached_history) = cache_manager.get_activity_history(pet_id, &date_range).await {
                debug!("Using cached activity history for pet {}", pet_id);
                cached_history.data
            } else {
                debug!("No cached activity history, fetching from API");
                match self.client.get_activity_history(token, pet_id, date_range.clone()).await {
                    Ok(history) => {
                        // Cache the response for future use
                        if let Err(e) = cache_manager.cache_activity_history(history.clone(), &date_range).await {
                            warn!("Failed to cache activity history: {}", e);
                        }
                        history
                    }
                    Err(e) => return Err(io::Error::new(io::ErrorKind::Other, format!("API error: {}", e)))
                }
            }
        } else {
            // No cache manager, fetch directly from API
            match self.client.get_activity_history(token, pet_id, date_range.clone()).await {
                Ok(history) => history,
                Err(e) => return Err(io::Error::new(io::ErrorKind::Other, format!("API error: {}", e)))
            }
        };

        let output = self.formatter.format_activity_history(&history);
        print!("{}", output);
        Ok(())
    }
    
    async fn handle_search(&self, token: &str) -> io::Result<()> {
        debug!("Performing search operation");
        
        let search_type = cliclack::select("What would you like to search?")
            .item("pets", "Search Pets", "Search pets by name, breed, or location")
            .item("devices", "Search Devices", "Search devices by name or status")
            .interact()?;

        match search_type {
            "pets" => {
                // Get current pets data
                let pets_response = self.client.get_pets(token).await
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to get pets: {}", e)))?;

                if pets_response.data.is_empty() {
                    println!("No pets found in your account.");
                    return Ok(());
                }

                // Build search criteria
                let name_input: String = cliclack::input("Pet name (partial match, or press Enter to skip):")
                    .placeholder("e.g., Fluffy")
                    .required(false)
                    .interact()?;
                
                let name = if name_input.trim().is_empty() { None } else { Some(name_input) };
                
                let location = cliclack::select("Filter by location:")
                    .item("any", "Any Location", "Don't filter by location")
                    .item("inside", "Inside", "Only pets currently inside")
                    .item("outside", "Outside", "Only pets currently outside")
                    .interact()?;
                
                let location_filter = match location {
                    "inside" => Some(1),
                    "outside" => Some(2),
                    _ => None,
                };

                // Create search criteria
                let criteria = PetSearchCriteria {
                    name_pattern: name,
                    breed_pattern: None,
                    characteristics: None,
                    location: location_filter,
                    activity_since: None,
                    inactive_threshold_hours: None,
                };

                // Perform search
                let search_results = SearchManager::search_pets(&pets_response.data, &criteria);
                
                // Display results
                println!("🔍 Search Results:");
                println!("  Found {} pets (filtered from {} total)", 
                    search_results.total_count, search_results.search_metadata.original_count);
                
                if !search_results.active_filters.is_empty() {
                    println!("  Active filters: {}", search_results.active_filters.join(", "));
                }
                
                println!("  Search took {}ms\n", search_results.search_metadata.search_duration_ms);

                if search_results.results.is_empty() {
                    println!("No pets match your search criteria.");
                } else {
                    // Format and display matching pets
                    let filtered_response = crate::api::client::PetsResponse {
                        data: search_results.results,
                    };
                    let output = self.formatter.format_pets(&filtered_response);
                    print!("{}", output);
                }
            }
            "devices" => {
                // Get current devices data
                let devices_response = self.client.get_devices(token).await
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to get devices: {}", e)))?;

                if devices_response.data.is_empty() {
                    println!("No devices found in your account.");
                    return Ok(());
                }

                // Build search criteria
                let name_input: String = cliclack::input("Device name (partial match, or press Enter to skip):")
                    .placeholder("e.g., Pet Door")
                    .required(false)
                    .interact()?;
                
                let name = if name_input.trim().is_empty() { None } else { Some(name_input) };

                let status_filter = cliclack::select("Filter by status:")
                    .item("any", "Any Status", "Don't filter by online status")
                    .item("online", "Online Only", "Only online devices")
                    .item("offline", "Offline Only", "Only offline devices")
                    .interact()?;

                let online_status = match status_filter {
                    "online" => Some(true),
                    "offline" => Some(false),
                    _ => None,
                };

                let battery_input: String = cliclack::input("Battery threshold (show devices below this %, or press Enter to skip):")
                    .placeholder("e.g., 20")
                    .required(false)
                    .interact()?;

                let battery_threshold = if battery_input.trim().is_empty() {
                    None
                } else {
                    battery_input.parse::<f32>().ok()
                };

                // Create search criteria
                let criteria = DeviceSearchCriteria {
                    name_pattern: name,
                    device_type: None,
                    online_status,
                    battery_threshold,
                };

                // Perform search
                let search_results = SearchManager::search_devices(&devices_response.data, &criteria);
                
                // Display results
                println!("🔍 Search Results:");
                println!("  Found {} devices (filtered from {} total)", 
                    search_results.total_count, search_results.search_metadata.original_count);
                
                if !search_results.active_filters.is_empty() {
                    println!("  Active filters: {}", search_results.active_filters.join(", "));
                }
                
                println!("  Search took {}ms\n", search_results.search_metadata.search_duration_ms);

                if search_results.results.is_empty() {
                    println!("No devices match your search criteria.");
                } else {
                    // Format and display matching devices
                    let filtered_response = crate::api::client::DevicesResponse {
                        data: search_results.results,
                    };
                    let output = self.formatter.format_devices(&filtered_response);
                    print!("{}", output);
                }
            }
            _ => {
                error!("Invalid search type selection: {}", search_type);
            }
        }

        Ok(())
    }
    
    async fn handle_export_data(&self, token: &str) -> io::Result<()> {
        debug!("Performing export data operation");
        
        let format = cliclack::select("Select export format:")
            .item("csv", "CSV", "Comma-separated values")
            .item("json", "JSON", "JavaScript Object Notation")
            .interact()?;
        
        let data_types = cliclack::multiselect("Select data types to export:")
            .item("pets", "Pets", "Pet information and status")
            .item("devices", "Devices", "Device information and status")
            .item("feeding", "Feeding History", "Feeding events")
            .item("drinking", "Drinking History", "Drinking events")
            .item("activity", "Activity History", "Movement events")
            .interact()?;
        
        if data_types.is_empty() {
            println!("No data types selected for export.");
            return Ok(());
        }
        
        let range = cliclack::select("Select date range for historical data:")
            .item("today", "Today", "Today's data only")
            .item("week", "This Week", "Past 7 days")
            .item("month", "This Month", "Past 30 days")
            .interact()?;

        // Parse date range
        let date_range = crate::cli::parse_date_range(range)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

        // Convert format selection
        let export_format = match format {
            "csv" => ExportFormat::Csv,
            "json" => ExportFormat::Json,
            _ => ExportFormat::Json,
        };

        // Convert data type selections
        let mut export_data_types = Vec::new();
        for data_type in &data_types {
            match data_type.as_ref() {
                "pets" => export_data_types.push(DataType::PetStatus),
                "devices" => export_data_types.push(DataType::DeviceStatus),
                "feeding" => export_data_types.push(DataType::Feeding),
                "drinking" => export_data_types.push(DataType::Drinking),
                "activity" => export_data_types.push(DataType::Activity),
                _ => {}
            }
        }

        // Generate filename
        let filename = ExportManager::generate_filename("rusty_pet_export", &export_format, &date_range);
        let output_path = std::path::PathBuf::from(&filename);

        // Create export config
        let config = ExportConfig {
            format: export_format,
            date_range: date_range.clone(),
            include_pets: vec![], // Export all pets
            data_types: export_data_types,
            output_path: output_path.clone(),
        };

        println!("📁 Export configuration:");
        println!("  • Format: {}", format.to_uppercase());
        println!("  • Data types: {}", data_types.join(", "));
        println!("  • Date range: {} to {}", 
            date_range.from.format("%Y-%m-%d"), 
            date_range.to.format("%Y-%m-%d"));
        println!("  • Output file: {}", filename);

        // Collect data based on selected types
        let mut pets_data = Vec::new();
        let mut devices_data = Vec::new();
        let mut feeding_histories = Vec::new();
        let mut drinking_histories = Vec::new();
        let mut activity_histories = Vec::new();

        // Fetch pets data if needed
        if data_types.iter().any(|dt| *dt == "pets") || 
           data_types.iter().any(|dt| *dt == "feeding") || 
           data_types.iter().any(|dt| *dt == "drinking") || 
           data_types.iter().any(|dt| *dt == "activity") {
            match self.client.get_pets(token).await {
                Ok(pets_response) => {
                    if data_types.iter().any(|dt| *dt == "pets") {
                        pets_data = pets_response.data.clone();
                    }
                    
                    // Fetch historical data for each pet if requested
                    for pet in &pets_response.data {
                        if data_types.iter().any(|dt| *dt == "feeding") {
                            match self.client.get_feeding_history(token, pet.id, date_range.clone()).await {
                                Ok(history) => feeding_histories.push(history),
                                Err(e) => warn!("Failed to get feeding history for pet {}: {}", pet.id, e),
                            }
                        }
                        
                        if data_types.iter().any(|dt| *dt == "drinking") {
                            match self.client.get_drinking_history(token, pet.id, date_range.clone()).await {
                                Ok(history) => drinking_histories.push(history),
                                Err(e) => warn!("Failed to get drinking history for pet {}: {}", pet.id, e),
                            }
                        }
                        
                        if data_types.iter().any(|dt| *dt == "activity") {
                            match self.client.get_activity_history(token, pet.id, date_range.clone()).await {
                                Ok(history) => activity_histories.push(history),
                                Err(e) => warn!("Failed to get activity history for pet {}: {}", pet.id, e),
                            }
                        }
                    }
                }
                Err(e) => {
                    let error_msg = format!("Failed to fetch pets data: {}", e);
                    let output = self.formatter.format_error(&error_msg);
                    print!("{}", output);
                    return Err(io::Error::new(io::ErrorKind::Other, error_msg));
                }
            }
        }

        // Fetch devices data if needed
        if data_types.iter().any(|dt| *dt == "devices") {
            match self.client.get_devices(token).await {
                Ok(devices_response) => {
                    devices_data = devices_response.data.clone();
                }
                Err(e) => {
                    let error_msg = format!("Failed to fetch devices data: {}", e);
                    let output = self.formatter.format_error(&error_msg);
                    print!("{}", output);
                    return Err(io::Error::new(io::ErrorKind::Other, error_msg));
                }
            }
        }

        // Create export data structure
        let export_data = ExportManager::create_export_data(
            feeding_histories.iter().collect(),
            drinking_histories.iter().collect(),
            activity_histories.iter().collect(),
            pets_data.iter().collect(),
            devices_data.iter().collect(),
            &config,
        );

        // Perform export
        let export_result = match config.format {
            ExportFormat::Csv => ExportManager::export_to_csv(&export_data, &output_path),
            ExportFormat::Json => ExportManager::export_to_json(&export_data, &output_path),
        };

        match export_result {
            Ok(()) => {
                let success_msg = format!("Data exported successfully to {}", filename);
                let output = self.formatter.format_success_message(&success_msg);
                print!("{}", output);
                
                println!("📊 Export Summary:");
                println!("  • Total records: {}", export_data.metadata.total_records);
                println!("  • Feeding events: {}", export_data.feeding_data.len());
                println!("  • Drinking events: {}", export_data.drinking_data.len());
                println!("  • Activity events: {}", export_data.activity_data.len());
                println!("  • Pet records: {}", export_data.pet_data.len());
                println!("  • Device records: {}", export_data.device_data.len());
            }
            Err(e) => {
                let error_msg = format!("Export failed: {}", e);
                let output = self.formatter.format_error(&error_msg);
                print!("{}", output);
                return Err(io::Error::new(io::ErrorKind::Other, error_msg));
            }
        }
        
        Ok(())
    }
    
    async fn handle_batch_operations(&self, token: &str) -> io::Result<()> {
        debug!("Performing batch operations");
        
        let operation = cliclack::select("Select batch operation:")
            .item("set-location", "Set Location", "Set location for multiple pets")
            .item("set-indoor", "Set Indoor", "Mark multiple pets as indoor")
            .item("set-outdoor", "Set Outdoor", "Mark multiple pets as outdoor")
            .item("lock", "Lock Devices", "Lock multiple devices")
            .item("unlock", "Unlock Devices", "Unlock multiple devices")
            .interact()?;
        
        match operation {
            "set-location" => {
                let location = cliclack::select("Select location:")
                    .item("inside", "Inside", "Set pets to inside")
                    .item("outside", "Outside", "Set pets to outside")
                    .interact()?;
                
                let location_id = match location {
                    "inside" => 1,
                    "outside" => 2,
                    _ => return Err(io::Error::new(io::ErrorKind::InvalidInput, "Invalid location")),
                };
                
                // Get pets for selection
                let pets_response = self.client.get_pets(token).await
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to get pets: {}", e)))?;

                if pets_response.data.is_empty() {
                    println!("No pets found in your account.");
                    return Ok(());
                }

                let pet_options: Vec<String> = pets_response.data.iter()
                    .map(|pet| format!("{} ({})", pet.name, pet.id))
                    .collect();
                
                let selected_pets = cliclack::multiselect("Select pets:")
                    .items(&pet_options.iter().map(|s| (s.as_str(), s.as_str(), "")).collect::<Vec<_>>())
                    .interact()?;
                
                if selected_pets.is_empty() {
                    println!("No pets selected.");
                    return Ok(());
                }
                
                // Extract pet IDs from selections
                let mut pet_updates = Vec::new();
                for selection in &selected_pets {
                    // Extract ID from "Name (ID)" format
                    if let Some(start) = selection.rfind('(') {
                        if let Some(end) = selection.rfind(')') {
                            if let Ok(pet_id) = selection[start+1..end].parse::<u32>() {
                                pet_updates.push(crate::api::client::PetLocationUpdate {
                                    pet_id,
                                    location: location_id,
                                });
                            }
                        }
                    }
                }
                
                if pet_updates.is_empty() {
                    return Err(io::Error::new(io::ErrorKind::InvalidInput, "Failed to parse pet IDs"));
                }
                
                println!("🔄 Executing batch operation...");
                match self.client.batch_set_pet_locations(token, pet_updates).await {
                    Ok(result) => {
                        let success_msg = format!("Batch operation completed: {} successful, {} failed", 
                            result.successful.len(), result.failed.len());
                        let output = self.formatter.format_success_message(&success_msg);
                        print!("{}", output);
                        
                        if !result.failed.is_empty() {
                            println!("❌ Failed operations:");
                            for error in &result.failed {
                                println!("  • Pet ID {}: {}", error.id, error.error);
                            }
                        }
                    }
                    Err(e) => {
                        let error_msg = format!("Batch operation failed: {}", e);
                        let output = self.formatter.format_error(&error_msg);
                        print!("{}", output);
                        return Err(io::Error::new(io::ErrorKind::Other, error_msg));
                    }
                }
            }
            "set-indoor" | "set-outdoor" => {
                let location_id = if operation == "set-indoor" { 1 } else { 2 };
                let mode = if operation == "set-indoor" { "indoor" } else { "outdoor" };
                
                // Get pets for selection
                let pets_response = self.client.get_pets(token).await
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to get pets: {}", e)))?;

                if pets_response.data.is_empty() {
                    println!("No pets found in your account.");
                    return Ok(());
                }

                let pet_options: Vec<String> = pets_response.data.iter()
                    .map(|pet| format!("{} ({})", pet.name, pet.id))
                    .collect();
                
                let selected_pets = cliclack::multiselect("Select pets:")
                    .items(&pet_options.iter().map(|s| (s.as_str(), s.as_str(), "")).collect::<Vec<_>>())
                    .interact()?;
                
                if selected_pets.is_empty() {
                    println!("No pets selected.");
                    return Ok(());
                }
                
                // Extract pet IDs and create batch updates
                let mut pet_updates = Vec::new();
                for selection in &selected_pets {
                    if let Some(start) = selection.rfind('(') {
                        if let Some(end) = selection.rfind(')') {
                            if let Ok(pet_id) = selection[start+1..end].parse::<u32>() {
                                pet_updates.push(crate::api::client::PetLocationUpdate {
                                    pet_id,
                                    location: location_id,
                                });
                            }
                        }
                    }
                }
                
                if pet_updates.is_empty() {
                    return Err(io::Error::new(io::ErrorKind::InvalidInput, "Failed to parse pet IDs"));
                }
                
                println!("🔄 Executing batch operation...");
                match self.client.batch_set_pet_locations(token, pet_updates).await {
                    Ok(result) => {
                        let success_msg = format!("Batch {} operation completed: {} successful, {} failed", 
                            mode, result.successful.len(), result.failed.len());
                        let output = self.formatter.format_success_message(&success_msg);
                        print!("{}", output);
                        
                        if !result.failed.is_empty() {
                            println!("❌ Failed operations:");
                            for error in &result.failed {
                                println!("  • Pet ID {}: {}", error.id, error.error);
                            }
                        }
                    }
                    Err(e) => {
                        let error_msg = format!("Batch {} operation failed: {}", mode, e);
                        let output = self.formatter.format_error(&error_msg);
                        print!("{}", output);
                        return Err(io::Error::new(io::ErrorKind::Other, error_msg));
                    }
                }
            }
            "lock" | "unlock" => {
                let lock_state = match operation {
                    "lock" => 3, // LOCK_STATE_LOCKED
                    "unlock" => 0, // LOCK_STATE_UNLOCKED
                    _ => return Err(io::Error::new(io::ErrorKind::InvalidInput, "Invalid operation")),
                };
                let action = operation;
                
                // Get devices for selection
                let devices_response = self.client.get_devices(token).await
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to get devices: {}", e)))?;

                if devices_response.data.is_empty() {
                    println!("No devices found in your account.");
                    return Ok(());
                }

                let device_options: Vec<String> = devices_response.data.iter()
                    .map(|device| format!("{} ({})", device.name, device.id))
                    .collect();
                
                let selected_devices = cliclack::multiselect("Select devices:")
                    .items(&device_options.iter().map(|s| (s.as_str(), s.as_str(), "")).collect::<Vec<_>>())
                    .interact()?;
                
                if selected_devices.is_empty() {
                    println!("No devices selected.");
                    return Ok(());
                }
                
                // Extract device IDs and create batch commands
                let mut device_commands = Vec::new();
                for selection in &selected_devices {
                    if let Some(start) = selection.rfind('(') {
                        if let Some(end) = selection.rfind(')') {
                            if let Ok(device_id) = selection[start+1..end].parse::<u32>() {
                                device_commands.push(crate::api::client::DeviceCommand {
                                    device_id,
                                    command_type: crate::api::client::DeviceCommandType::SetLockState { 
                                        lock_state 
                                    },
                                });
                            }
                        }
                    }
                }
                
                if device_commands.is_empty() {
                    return Err(io::Error::new(io::ErrorKind::InvalidInput, "Failed to parse device IDs"));
                }
                
                println!("🔄 Executing batch operation...");
                match self.client.batch_device_control(token, device_commands).await {
                    Ok(result) => {
                        let success_msg = format!("Batch {} operation completed: {} successful, {} failed", 
                            action, result.successful.len(), result.failed.len());
                        let output = self.formatter.format_success_message(&success_msg);
                        print!("{}", output);
                        
                        if !result.failed.is_empty() {
                            println!("❌ Failed operations:");
                            for error in &result.failed {
                                println!("  • Device ID {}: {}", error.id, error.error);
                            }
                        }
                    }
                    Err(e) => {
                        let error_msg = format!("Batch {} operation failed: {}", action, e);
                        let output = self.formatter.format_error(&error_msg);
                        print!("{}", output);
                        return Err(io::Error::new(io::ErrorKind::Other, error_msg));
                    }
                }
            }
            _ => {
                error!("Invalid batch operation selection: {}", operation);
            }
        }
        
        Ok(())
    }

    /// Display pet data completeness summary
    fn display_pet_completeness_summary(&self, pet_reports: &[(String, crate::data_validation::DataCompletenessReport, crate::data_validation::DataCompletenessReport)]) {
        // Only display completeness issues if log level is debug or above
        if !log::log_enabled!(log::Level::Debug) {
            return;
        }
        
        let incomplete_pets: Vec<_> = pet_reports.iter()
            .filter(|(_, details_report, activity_report)| !details_report.is_complete || !activity_report.is_complete)
            .collect();
        
        if !incomplete_pets.is_empty() {
            println!("\n⚠️  Data Completeness Issues:");
            for (pet_name, details_report, activity_report) in &incomplete_pets {
                println!("  Pet: {}", style(pet_name).yellow().bold());
                
                if !details_report.is_complete {
                    println!("    Missing details: {}", details_report.missing_fields.join(", "));
                }
                
                if !activity_report.is_complete {
                    println!("    Missing activity data: {}", activity_report.missing_fields.join(", "));
                }
                
                // Display warnings
                for warning in &details_report.warnings {
                    println!("    ⚠️  {}", warning);
                }
                for warning in &activity_report.warnings {
                    println!("    ⚠️  {}", warning);
                }
                
                let avg_score = (details_report.completeness_score + activity_report.completeness_score) / 2.0;
                println!("    Completeness: {:.1}%", avg_score * 100.0);
            }
            println!();
        }
    }

    /// Display device data completeness summary
    fn display_device_completeness_summary(&self, device_reports: &[(String, crate::data_validation::DataCompletenessReport, crate::data_validation::DataCompletenessReport)]) {
        // Only display completeness issues if log level is debug or above
        if !log::log_enabled!(log::Level::Debug) {
            return;
        }
        
        let incomplete_devices: Vec<_> = device_reports.iter()
            .filter(|(_, status_report, history_report)| !status_report.is_complete || !history_report.is_complete)
            .collect();
        
        if !incomplete_devices.is_empty() {
            println!("\n⚠️  Device Data Completeness Issues:");
            for (device_name, status_report, history_report) in &incomplete_devices {
                println!("  Device: {}", style(device_name).yellow().bold());
                
                if !status_report.is_complete {
                    println!("    Missing status info: {}", status_report.missing_fields.join(", "));
                }
                
                if !history_report.is_complete {
                    println!("    Missing history data: {}", history_report.missing_fields.join(", "));
                }
                
                // Display warnings
                for warning in &status_report.warnings {
                    println!("    ⚠️  {}", warning);
                }
                for warning in &history_report.warnings {
                    println!("    ⚠️  {}", warning);
                }
                
                let avg_score = (status_report.completeness_score + history_report.completeness_score) / 2.0;
                println!("    Completeness: {:.1}%", avg_score * 100.0);
            }
            println!();
        }
    }
}