use crate::api::client::{PetsResponse, DevicesResponse, GENDER_FEMALE, GENDER_MALE};
use crate::data_processor::{DataProcessor, InactivityConfig, DeviceHealthConfig, AlertSeverity};
use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use serde_json;

/// Trait for formatting output data in different formats
pub trait OutputFormatter {
    fn format_pets(&self, pets_response: &PetsResponse) -> String;
    fn format_devices(&self, devices_response: &DevicesResponse) -> String;
    fn format_success_message(&self, message: &str) -> String;
    fn format_error(&self, error: &str) -> String;
    fn format_timestamp(&self, timestamp: &str) -> String;
    fn format_feeding_history(&self, history: &crate::api::client::FeedingHistory) -> String;
    fn format_drinking_history(&self, history: &crate::api::client::DrinkingHistory) -> String;
    fn format_activity_history(&self, history: &crate::api::client::ActivityHistory) -> String;
}

/// Human-readable formatter for interactive display
pub struct HumanFormatter {
    timezone: Tz,
    inactivity_config: InactivityConfig,
    device_health_config: DeviceHealthConfig,
}

impl HumanFormatter {
    pub fn new() -> Self {
        // Default to local timezone, fallback to UTC if detection fails
        let timezone = match iana_time_zone::get_timezone() {
            Ok(tz_name) => tz_name.parse().unwrap_or(Tz::UTC),
            Err(_) => Tz::UTC,
        };
        
        Self { 
            timezone,
            inactivity_config: InactivityConfig::default(),
            device_health_config: DeviceHealthConfig::default(),
        }
    }

    fn format_battery_indicator(battery: f32) -> String {
        let percentage = battery as u8;
        let filled_bars = ((percentage + 10) / 20) as usize; // Round to nearest 20% increment
        let filled_bars = filled_bars.min(5); // Cap at 5 bars maximum
        let empty_bars = 5 - filled_bars;
        
        let filled = "●".repeat(filled_bars);
        let empty = "○".repeat(empty_bars);
        
        format!("{}% {}{}", percentage, filled, empty)
    }

    fn format_location(location: u32) -> &'static str {
        match location {
            1 => "🏠 Inside",
            2 => "🌳 Outside",
            _ => "❓ Unknown",
        }
    }

    fn format_lock_state(mode: u32) -> &'static str {
        match mode {
            0 => "🔓 Unlocked",
            1 => "🔒 Keep In",
            2 => "🔒 Keep Out",
            3 => "🔒 Locked",
            4 => "🌙 Curfew",
            _ => "❓ Unknown",
        }
    }

    fn format_online_status(online: bool) -> &'static str {
        if online {
            "✅ Online"
        } else {
            "❌ Offline"
        }
    }

    fn format_gender(gender: u32) -> &'static str {
        match gender {
            GENDER_FEMALE => "♀ Female",
            GENDER_MALE => "♂ Male",
            _ => "❓ Unknown",
        }
    }

    fn format_alert_severity_icon(severity: &AlertSeverity) -> &'static str {
        match severity {
            AlertSeverity::Low => "ℹ️",
            AlertSeverity::Medium => "⚠️",
            AlertSeverity::High => "🚨",
            AlertSeverity::Critical => "🔴",
        }
    }

    fn format_inactive_pet_indicator(is_inactive: bool, is_critical: bool) -> &'static str {
        if is_critical {
            "🔴 CRITICAL INACTIVE"
        } else if is_inactive {
            "⚠️ INACTIVE"
        } else {
            ""
        }
    }
}

impl OutputFormatter for HumanFormatter {
    fn format_pets(&self, pets_response: &PetsResponse) -> String {
        let mut output = String::new();
        output.push_str("🐾 Your Pets:\n");
        
        if pets_response.data.is_empty() {
            output.push_str("  No pets found in your account.\n");
            return output;
        }

        // Generate inactive pet alerts
        let inactive_alerts = DataProcessor::identify_inactive_pets(&pets_response.data, &self.inactivity_config);
        
        for pet in &pets_response.data {
            // Check if this pet is inactive
            let is_inactive = DataProcessor::is_pet_inactive(pet, &self.inactivity_config);
            let pet_alerts: Vec<_> = inactive_alerts.iter().filter(|a| a.pet_id == Some(pet.id)).collect();
            let is_critical = pet_alerts.iter().any(|a| matches!(a.severity, AlertSeverity::Critical));
            
            // Pet header with name and basic info
            let inactive_indicator = Self::format_inactive_pet_indicator(is_inactive, is_critical);
            if !inactive_indicator.is_empty() {
                output.push_str(&format!("  • {} (ID: {}) {}\n", pet.name, pet.id, inactive_indicator));
            } else {
                output.push_str(&format!("  • {} (ID: {})\n", pet.name, pet.id));
            }
            
            // Show specific alert messages for this pet
            for alert in &pet_alerts {
                let icon = Self::format_alert_severity_icon(&alert.severity);
                output.push_str(&format!("    {} {}\n", icon, alert.message));
            }
            
            // Gender and basic details
            let gender = Self::format_gender(pet.gender.unwrap_or(0));
            output.push_str(&format!("    Gender: {}\n", gender));
            
            if let Some(date_of_birth) = &pet.date_of_birth {
                if !date_of_birth.is_empty() {
                    output.push_str(&format!("    Born: {}\n", date_of_birth));
                }
            }
            
            if let Some(weight) = &pet.weight {
                if !weight.is_empty() && weight != "0" {
                    output.push_str(&format!("    Weight: {}g\n", weight));
                }
            }
            
            // Current location
            let location = match &pet.position {
                Some(pos) => match pos.location {
                    Some(loc) => Self::format_location(loc),
                    None => "❓ Unknown",
                },
                None => "❓ Unknown",
            };
            output.push_str(&format!("    Location: {}\n", location));
            
            // Activity information
            if let Some(status) = &pet.status {
                if let Some(activity) = &status.activity {
                    let formatted_time = self.format_timestamp(&activity.since);
                    output.push_str(&format!("    Last Activity: {}\n", formatted_time));
                }
                
                if let Some(feeding) = &status.feeding {
                    let formatted_time = self.format_timestamp(&feeding.at);
                    output.push_str(&format!("    Last Fed: {}\n", formatted_time));
                    if let Some(change) = &feeding.change {
                        if !change.is_empty() {
                            output.push_str(&format!("    Food Change: {:.1}g\n", change[0]));
                        }
                    }
                }
                
                if let Some(drinking) = &status.drinking {
                    let formatted_time = self.format_timestamp(&drinking.at);
                    output.push_str(&format!("    Last Drink: {}\n", formatted_time));
                    if let Some(change) = &drinking.change {
                        if !change.is_empty() {
                            output.push_str(&format!("    Water Change: {:.1}ml\n", change[0]));
                        }
                    }
                }
            }
            
            output.push('\n'); // Add spacing between pets
        }
        
        output
    }

    fn format_devices(&self, devices_response: &DevicesResponse) -> String {
        let mut output = String::new();
        output.push_str("🏠 Your Devices:\n");
        
        if devices_response.data.is_empty() {
            output.push_str("  No devices found in your account.\n");
            return output;
        }

        // Generate device health alerts
        let device_alerts = DataProcessor::generate_device_health_alerts(&devices_response.data, &self.device_health_config);

        for device in &devices_response.data {
            // Device header
            output.push_str(&format!("  • {} (ID: {})\n", device.name, device.id));
            output.push_str(&format!("    Serial: {}\n", device.serial_number));
            
            // Show device-specific alerts
            let device_specific_alerts: Vec<_> = device_alerts.iter().filter(|a| a.device_id == Some(device.id)).collect();
            for alert in &device_specific_alerts {
                let icon = Self::format_alert_severity_icon(&alert.severity);
                output.push_str(&format!("    {} {}\n", icon, alert.message));
            }
            
            if let Some(status) = &device.status {
                // Online status
                if let Some(online) = status.online {
                    let online_status = Self::format_online_status(online);
                    output.push_str(&format!("    Status: {}\n", online_status));
                }
                
                // Battery level with visual indicator
                if let Some(battery) = status.battery {
                    let battery_as_percentage = battery * 10.0;
                    output.push_str(&format!("    Battery: {}\n", Self::format_battery_indicator(battery_as_percentage)));
                }
                
                // Lock state and curfew information
                if let Some(locking) = &status.locking {
                    let lock_state = Self::format_lock_state(locking.mode);
                    output.push_str(&format!("    Lock State: {}\n", lock_state));
                    
                    if let Some(curfew_times) = &locking.curfew {
                        let active_curfews: Vec<_> = curfew_times.iter().filter(|c| c.enabled).collect();
                        if !active_curfews.is_empty() {
                            output.push_str("    Active Curfews:\n");
                            for curfew in active_curfews {
                                output.push_str(&format!("      🌙 {} - {}\n", curfew.lock_time, curfew.unlock_time));
                            }
                        }
                    }
                }
                
                // Learn mode indicator
                if let Some(learn_mode) = status.learn_mode {
                    if learn_mode {
                        output.push_str("    🎓 Learn Mode: Active\n");
                    }
                }
            } else {
                output.push_str("    Status: Information not available\n");
            }
            
            output.push('\n'); // Add spacing between devices
        }
        
        output
    }

    fn format_success_message(&self, message: &str) -> String {
        format!("✅ {}\n", message)
    }

    fn format_error(&self, error: &str) -> String {
        format!("❌ Error: {}\n", error)
    }

    fn format_timestamp(&self, timestamp: &str) -> String {
        // Parse the ISO 8601 timestamp
        match timestamp.parse::<DateTime<Utc>>() {
            Ok(utc_time) => {
                // Convert to local timezone
                let local_time = utc_time.with_timezone(&self.timezone);
                // Format in a human-readable way
                local_time.format("%Y-%m-%d %H:%M:%S %Z").to_string()
            }
            Err(_) => {
                // If parsing fails, return the original timestamp
                timestamp.to_string()
            }
        }
    }

    fn format_feeding_history(&self, history: &crate::api::client::FeedingHistory) -> String {
        let mut output = String::new();
        output.push_str("🍽️ Feeding History:\n");
        
        if history.events.is_empty() {
            output.push_str("  No feeding events found for the selected period.\n");
            return output;
        }

        // Show summary if available
        if let Some(summary) = &history.summary {
            output.push_str(&format!("📊 Summary: {} events, {:.1}g total, {:.1}g daily average\n\n", 
                summary.event_count, summary.total_amount, summary.daily_average));
        }

        // Show individual events
        for event in &history.events {
            let formatted_time = self.format_timestamp(&event.timestamp.to_rfc3339());
            output.push_str(&format!("  • {} - {:.1}g (Device: {})", 
                formatted_time, event.amount, event.device_id));
            
            if let Some(duration) = event.duration {
                output.push_str(&format!(" - {}s duration", duration));
            }
            output.push('\n');
        }
        
        output.push('\n');
        output
    }

    fn format_drinking_history(&self, history: &crate::api::client::DrinkingHistory) -> String {
        let mut output = String::new();
        output.push_str("💧 Drinking History:\n");
        
        if history.events.is_empty() {
            output.push_str("  No drinking events found for the selected period.\n");
            return output;
        }

        // Show summary if available
        if let Some(summary) = &history.summary {
            output.push_str(&format!("📊 Summary: {} events, {:.1}ml total, {:.1}ml daily average\n\n", 
                summary.event_count, summary.total_volume, summary.daily_average));
        }

        // Show individual events
        for event in &history.events {
            let formatted_time = self.format_timestamp(&event.timestamp.to_rfc3339());
            output.push_str(&format!("  • {} - {:.1}ml (Device: {})", 
                formatted_time, event.volume, event.device_id));
            
            if let Some(duration) = event.duration {
                output.push_str(&format!(" - {}s duration", duration));
            }
            output.push('\n');
        }
        
        output.push('\n');
        output
    }

    fn format_activity_history(&self, history: &crate::api::client::ActivityHistory) -> String {
        let mut output = String::new();
        output.push_str("🏃 Activity History:\n");
        
        if history.events.is_empty() {
            output.push_str("  No activity events found for the selected period.\n");
            return output;
        }

        // Show summary if available
        if let Some(summary) = &history.summary {
            output.push_str(&format!("📊 Summary: {} events, {} entries, {} exits\n\n", 
                summary.total_events, summary.entries, summary.exits));
        }

        // Show individual events
        for event in &history.events {
            let formatted_time = self.format_timestamp(&event.timestamp.to_rfc3339());
            let event_type = match event.event_type {
                crate::api::client::ActivityType::Entry => "🏠 Entry",
                crate::api::client::ActivityType::Exit => "🌳 Exit",
                crate::api::client::ActivityType::FeedingStart => "🍽️ Feeding Start",
                crate::api::client::ActivityType::FeedingEnd => "🍽️ Feeding End",
                crate::api::client::ActivityType::DrinkingStart => "💧 Drinking Start",
                crate::api::client::ActivityType::DrinkingEnd => "💧 Drinking End",
            };
            
            let location = match event.location {
                1 => "Inside",
                2 => "Outside",
                _ => "Unknown",
            };
            
            output.push_str(&format!("  • {} - {} at {}", 
                formatted_time, event_type, location));
            
            if let Some(device_id) = event.device_id {
                output.push_str(&format!(" (Device: {})", device_id));
            }
            output.push('\n');
        }
        
        output.push('\n');
        output
    }
}

/// JSON formatter for machine-readable output
pub struct JsonFormatter;

impl JsonFormatter {
    pub fn new() -> Self {
        Self
    }
}

impl OutputFormatter for JsonFormatter {
    fn format_pets(&self, pets_response: &PetsResponse) -> String {
        serde_json::to_string_pretty(pets_response).unwrap_or_else(|_| "{}".to_string())
    }

    fn format_devices(&self, devices_response: &DevicesResponse) -> String {
        serde_json::to_string_pretty(devices_response).unwrap_or_else(|_| "{}".to_string())
    }

    fn format_success_message(&self, message: &str) -> String {
        let response = serde_json::json!({
            "status": "success",
            "message": message
        });
        serde_json::to_string_pretty(&response).unwrap_or_else(|_| "{}".to_string())
    }

    fn format_error(&self, error: &str) -> String {
        let response = serde_json::json!({
            "status": "error",
            "message": error
        });
        serde_json::to_string_pretty(&response).unwrap_or_else(|_| "{}".to_string())
    }

    fn format_timestamp(&self, timestamp: &str) -> String {
        // For JSON output, keep timestamps in ISO 8601 format
        timestamp.to_string()
    }

    fn format_feeding_history(&self, history: &crate::api::client::FeedingHistory) -> String {
        serde_json::to_string_pretty(history).unwrap_or_else(|_| "{}".to_string())
    }

    fn format_drinking_history(&self, history: &crate::api::client::DrinkingHistory) -> String {
        serde_json::to_string_pretty(history).unwrap_or_else(|_| "{}".to_string())
    }

    fn format_activity_history(&self, history: &crate::api::client::ActivityHistory) -> String {
        serde_json::to_string_pretty(history).unwrap_or_else(|_| "{}".to_string())
    }
}

/// Factory function to create the appropriate formatter based on output mode
pub fn create_formatter(json_output: bool) -> Box<dyn OutputFormatter> {
    if json_output {
        Box::new(JsonFormatter::new())
    } else {
        Box::new(HumanFormatter::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::client::{Pet, Position, Status, Activity, Feeding, Drinking, Device, DeviceStatus, Tag, UsageStats};
    use crate::data_processor::AlertSeverity;

    fn create_test_pet() -> Pet {
        Pet {
            id: 1,
            name: "Fluffy".to_string(),
            gender: Some(GENDER_FEMALE),
            date_of_birth: Some("2020-01-01".to_string()),
            weight: Some("4500".to_string()),
            breed: Some("Test Breed".to_string()),
            comments: Some("".to_string()),
            household_id: 1,
            breed_id: 1,
            colour_id: Some(1),
            species_id: 1,
            tag_id: 123456,
            version: 1,
            created_at: "2020-01-01T00:00:00Z".to_string(),
            updated_at: "2020-01-01T00:00:00Z".to_string(),
            photo: None,
            status: Some(Status {
                activity: Some(Activity {
                    tag_id: 123456,
                    device_id: Some(1),
                    location: 1,
                    since: "2024-01-01T12:00:00Z".to_string(),
                }),
                feeding: Some(Feeding {
                    tag_id: 123456,
                    device_id: 1,
                    at: "2024-01-01T08:00:00Z".to_string(),
                    change: Some(vec![-1.0, 0.0]),
                }),
                drinking: Some(Drinking {
                    tag_id: 123456,
                    device_id: 1,
                    at: "2024-01-01T08:00:00Z".to_string(),
                    change: Some(vec![-0.5, 0.0]),
                }),
            }),
            position: Some(Position {
                user_id: Some(1),
                tag_id: 123456,
                location: Some(1),
                since: "2024-01-01T12:00:00Z".to_string(),
                version: Some(1),
                created_at: Some("2024-01-01T12:00:00Z".to_string()),
                updated_at: Some("2024-01-01T12:00:00Z".to_string()),
            }),
            tag: Some(Tag {
                id: 123456,
                index: Some(1),
                profile: Some(1),
            }),
        }
    }

    fn create_test_device() -> Device {
        Device {
            id: 1,
            name: "Pet Flap".to_string(),
            serial_number: "TEST123".to_string(),
            mac_address: "00:11:22:33:44:55".to_string(),
            product_id: 1,
            household_id: 1,
            parent_device_id: None,
            version: 1,
            created_at: "2020-01-01T00:00:00Z".to_string(),
            updated_at: "2020-01-01T00:00:00Z".to_string(),
            status: Some(DeviceStatus {
                locking: None,
                version: None,
                online: Some(true),
                battery: Some(7.5), // 75% (7.5 * 10)
                learn_mode: None,
                signal_strength: Some(85.0),
                usage_stats: Some(UsageStats {
                    total_entries: 15,
                    total_exits: 12,
                    last_entry: None,
                    last_exit: None,
                    daily_average_entries: 3.0,
                }),
            }),
            control: None,
        }
    }

    #[test]
    fn test_human_formatter_pets() {
        let formatter = HumanFormatter::new();
        let pets_response = PetsResponse {
            data: vec![create_test_pet()],
        };
        
        let output = formatter.format_pets(&pets_response);
        assert!(output.contains("🐾 Your Pets:"));
        assert!(output.contains("Fluffy"));
        assert!(output.contains("🏠 Inside"));
        assert!(output.contains("♀ Female"));
        assert!(!output.contains("{")); // No raw JSON
        assert!(!output.contains("\"")); // No quoted field names
    }

    #[test]
    fn test_human_formatter_devices() {
        let formatter = HumanFormatter::new();
        let devices_response = DevicesResponse {
            data: vec![create_test_device()],
        };
        
        let output = formatter.format_devices(&devices_response);
        assert!(output.contains("🏠 Your Devices:"));
        assert!(output.contains("Pet Flap"));
        assert!(output.contains("75% ●●●●○"));
        assert!(output.contains("✅ Online"));
    }

    #[test]
    fn test_json_formatter_pets() {
        let formatter = JsonFormatter::new();
        let pets_response = PetsResponse {
            data: vec![create_test_pet()],
        };
        
        let output = formatter.format_pets(&pets_response);
        assert!(output.contains("\"name\": \"Fluffy\""));
        assert!(serde_json::from_str::<serde_json::Value>(&output).is_ok());
    }

    #[test]
    fn test_battery_indicator() {
        assert_eq!(HumanFormatter::format_battery_indicator(100.0), "100% ●●●●●");
        assert_eq!(HumanFormatter::format_battery_indicator(60.0), "60% ●●●○○");
        assert_eq!(HumanFormatter::format_battery_indicator(0.0), "0% ○○○○○");
    }

    #[test]
    fn test_location_formatting() {
        assert_eq!(HumanFormatter::format_location(1), "🏠 Inside");
        assert_eq!(HumanFormatter::format_location(2), "🌳 Outside");
        assert_eq!(HumanFormatter::format_location(99), "❓ Unknown");
    }

    #[test]
    fn test_gender_formatting() {
        assert_eq!(HumanFormatter::format_gender(GENDER_FEMALE), "♀ Female");
        assert_eq!(HumanFormatter::format_gender(GENDER_MALE), "♂ Male");
        assert_eq!(HumanFormatter::format_gender(99), "❓ Unknown");
    }

    #[test]
    fn test_online_status_formatting() {
        assert_eq!(HumanFormatter::format_online_status(true), "✅ Online");
        assert_eq!(HumanFormatter::format_online_status(false), "❌ Offline");
    }

    #[test]
    fn test_alert_severity_icon() {
        assert_eq!(HumanFormatter::format_alert_severity_icon(&AlertSeverity::Low), "ℹ️");
        assert_eq!(HumanFormatter::format_alert_severity_icon(&AlertSeverity::Medium), "⚠️");
        assert_eq!(HumanFormatter::format_alert_severity_icon(&AlertSeverity::High), "🚨");
        assert_eq!(HumanFormatter::format_alert_severity_icon(&AlertSeverity::Critical), "🔴");
    }

    #[test]
    fn test_inactive_pet_indicator() {
        assert_eq!(HumanFormatter::format_inactive_pet_indicator(false, false), "");
        assert_eq!(HumanFormatter::format_inactive_pet_indicator(true, false), "⚠️ INACTIVE");
        assert_eq!(HumanFormatter::format_inactive_pet_indicator(true, true), "🔴 CRITICAL INACTIVE");
    }

    #[test]
    fn test_timestamp_formatting() {
        let formatter = HumanFormatter::new();
        let timestamp = "2024-01-01T12:00:00Z";
        let formatted = formatter.format_timestamp(timestamp);
        assert!(formatted.contains("2024-01-01"));
        assert!(formatted.contains("12:00:00"));
        // The formatted timestamp should be human-readable with timezone info
        assert!(formatted.contains("UTC") || formatted.contains("GMT") || formatted.contains("Z"));
    }

    #[test]
    fn test_create_formatter() {
        let human_formatter = create_formatter(false);
        let json_formatter = create_formatter(true);
        
        let message = "Test message";
        let human_output = human_formatter.format_success_message(message);
        let json_output = json_formatter.format_success_message(message);
        
        assert!(human_output.contains("✅"));
        assert!(json_output.contains("\"status\": \"success\""));
    }

    #[test]
    fn test_no_raw_json_in_human_output() {
        let formatter = HumanFormatter::new();
        let pets_response = PetsResponse {
            data: vec![create_test_pet()],
        };
        
        let output = formatter.format_pets(&pets_response);
        // Ensure no raw JSON markers are present
        assert!(!output.contains("{"));
        assert!(!output.contains("}"));
        assert!(!output.contains("\"location\":1"));
        assert!(!output.contains("\"name\":"));
    }
}