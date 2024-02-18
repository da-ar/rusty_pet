#![allow(dead_code)] // Module contains future functionality not yet integrated

use crate::api::client::{FeedingHistory, DrinkingHistory, ActivityHistory, Pet, Device};
use crate::data_processor::{TrendAnalysis, HealthMetrics, DataProcessor};
use crate::errors::CliError;
use chrono::{DateTime, Utc};
use csv::Writer;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::path::{Path, PathBuf};

/// Export manager for handling data export in various formats
pub struct ExportManager;

/// Configuration for data export operations
#[derive(Debug, Clone)]
pub struct ExportConfig {
    pub format: ExportFormat,
    pub date_range: DateRange,
    pub include_pets: Vec<u32>,
    pub data_types: Vec<DataType>,
    #[allow(dead_code)] // Future functionality
    pub output_path: PathBuf,
}

/// Supported export formats
#[derive(Debug, Clone, PartialEq)]
pub enum ExportFormat {
    Csv,
    Json,
}

use crate::api::client::DateRange;

/// Types of data that can be exported
#[derive(Debug, Clone, PartialEq)]
pub enum DataType {
    Feeding,
    Drinking,
    Activity,
    PetStatus,
    DeviceStatus,
}

/// Comprehensive data structure for export operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportData {
    pub metadata: ExportMetadata,
    pub feeding_data: Vec<FeedingExportRecord>,
    pub drinking_data: Vec<DrinkingExportRecord>,
    pub activity_data: Vec<ActivityExportRecord>,
    pub pet_data: Vec<PetExportRecord>,
    pub device_data: Vec<DeviceExportRecord>,
}

/// Metadata about the export operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportMetadata {
    pub export_timestamp: DateTime<Utc>,
    pub date_range: ExportDateRange,
    pub included_pets: Vec<u32>,
    pub data_types: Vec<String>,
    pub total_records: usize,
}

/// Serializable date range for export metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportDateRange {
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
}

/// Flattened feeding record for CSV export
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedingExportRecord {
    pub pet_id: u32,
    pub timestamp: DateTime<Utc>,
    pub device_id: u32,
    pub amount: f32,
    pub duration_seconds: Option<u32>,
}

/// Flattened drinking record for CSV export
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrinkingExportRecord {
    pub pet_id: u32,
    pub timestamp: DateTime<Utc>,
    pub device_id: u32,
    pub volume: f32,
    pub duration_seconds: Option<u32>,
}

/// Flattened activity record for CSV export
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityExportRecord {
    pub pet_id: u32,
    pub timestamp: DateTime<Utc>,
    pub event_type: String,
    pub location: u32,
    pub device_id: Option<u32>,
}

/// Pet status record for export
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PetExportRecord {
    pub pet_id: u32,
    pub name: String,
    pub current_location: Option<u32>,
    pub last_activity: Option<DateTime<Utc>>,
    pub last_feeding: Option<DateTime<Utc>>,
    pub last_drinking: Option<DateTime<Utc>>,
}

/// Device status record for export
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceExportRecord {
    pub device_id: u32,
    pub name: String,
    pub device_type: String,
    pub online: Option<bool>,
    pub battery_level: Option<f32>,
    pub lock_state: Option<u32>,
}

/// Report configuration for generating summary reports
#[derive(Debug, Clone)]
pub struct ReportConfig {
    pub include_trends: bool,
    pub include_health_metrics: bool,
    pub include_alerts: bool,
    pub date_range: DateRange,
}

/// Comprehensive report structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub metadata: ReportMetadata,
    pub summary: ReportSummary,
    pub trends: Option<TrendAnalysis>,
    pub health_metrics: Option<HealthMetrics>,
    pub alerts: Vec<String>,
    pub recommendations: Vec<String>,
}

/// Report metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportMetadata {
    pub generated_at: DateTime<Utc>,
    pub report_period: ExportDateRange,
    pub pets_included: Vec<String>,
    pub total_pets: usize,
    pub total_devices: usize,
}

/// Report summary with key metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportSummary {
    pub feeding_summary: FeedingReportSummary,
    pub drinking_summary: DrinkingReportSummary,
    pub activity_summary: ActivityReportSummary,
    pub device_summary: DeviceReportSummary,
}

/// Feeding summary for reports
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedingReportSummary {
    pub total_feeding_events: u32,
    pub total_food_consumed: f32,
    pub average_daily_intake: f32,
    pub most_active_feeder: Option<String>,
}

/// Drinking summary for reports
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrinkingReportSummary {
    pub total_drinking_events: u32,
    pub total_water_consumed: f32,
    pub average_daily_intake: f32,
    pub most_active_drinker: Option<String>,
}

/// Activity summary for reports
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityReportSummary {
    pub total_movements: u32,
    pub total_entries: u32,
    pub total_exits: u32,
    pub most_active_pet: Option<String>,
}

/// Device summary for reports
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceReportSummary {
    pub total_devices: u32,
    pub online_devices: u32,
    pub devices_with_low_battery: u32,
    pub average_battery_level: Option<f32>,
}

impl ExportManager {
    /// Export data to CSV format
    pub fn export_to_csv(data: &ExportData, path: &Path) -> Result<(), CliError> {
        // Create the directory if it doesn't exist
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                CliError::system_error_with_source(
                    &format!("Failed to create export directory: {}", parent.display()),
                    Some("Check directory permissions and try again".to_string()),
                    Box::new(e),
                )
            })?;
        }

        // Determine what data to export based on what's available
        let has_feeding = !data.feeding_data.is_empty();
        let has_drinking = !data.drinking_data.is_empty();
        let has_activity = !data.activity_data.is_empty();
        let has_pets = !data.pet_data.is_empty();
        let has_devices = !data.device_data.is_empty();

        if has_feeding || has_drinking || has_activity {
            // Export historical data to a single CSV file
            Self::export_historical_data_to_csv(data, path)?;
        } else if has_pets || has_devices {
            // Export status data to CSV
            Self::export_status_data_to_csv(data, path)?;
        } else {
            return Err(CliError::validation_error(
                "No data available for export",
                vec!["Ensure you have feeding, drinking, activity, pet, or device data to export".to_string()],
                None,
            ));
        }

        Ok(())
    }

    /// Export historical data (feeding, drinking, activity) to CSV
    fn export_historical_data_to_csv(data: &ExportData, path: &Path) -> Result<(), CliError> {
        let file = File::create(path).map_err(|e| {
            CliError::system_error_with_source(
                &format!("Failed to create CSV file: {}", path.display()),
                Some("Check file permissions and disk space".to_string()),
                Box::new(e),
            )
        })?;

        let mut writer = Writer::from_writer(file);

        // Write header for combined historical data
        writer.write_record(&[
            "data_type",
            "pet_id", 
            "timestamp",
            "device_id",
            "amount_or_volume",
            "event_type_or_location",
            "duration_seconds",
        ]).map_err(|e| {
            CliError::system_error_with_source(
                "Failed to write CSV header",
                Some("Check disk space and file permissions".to_string()),
                Box::new(e),
            )
        })?;

        // Write feeding data
        for record in &data.feeding_data {
            writer.write_record(&[
                "feeding",
                &record.pet_id.to_string(),
                &record.timestamp.to_rfc3339(),
                &record.device_id.to_string(),
                &record.amount.to_string(),
                "", // No event type for feeding
                &record.duration_seconds.map_or(String::new(), |d| d.to_string()),
            ]).map_err(|e| {
                CliError::system_error_with_source(
                    "Failed to write feeding data to CSV",
                    Some("Check disk space and file permissions".to_string()),
                    Box::new(e),
                )
            })?;
        }

        // Write drinking data
        for record in &data.drinking_data {
            writer.write_record(&[
                "drinking",
                &record.pet_id.to_string(),
                &record.timestamp.to_rfc3339(),
                &record.device_id.to_string(),
                &record.volume.to_string(),
                "", // No event type for drinking
                &record.duration_seconds.map_or(String::new(), |d| d.to_string()),
            ]).map_err(|e| {
                CliError::system_error_with_source(
                    "Failed to write drinking data to CSV",
                    Some("Check disk space and file permissions".to_string()),
                    Box::new(e),
                )
            })?;
        }

        // Write activity data
        for record in &data.activity_data {
            writer.write_record(&[
                "activity",
                &record.pet_id.to_string(),
                &record.timestamp.to_rfc3339(),
                &record.device_id.map_or(String::new(), |d| d.to_string()),
                "", // No amount for activity
                &format!("{}:{}", record.event_type, record.location),
                "", // No duration for activity
            ]).map_err(|e| {
                CliError::system_error_with_source(
                    "Failed to write activity data to CSV",
                    Some("Check disk space and file permissions".to_string()),
                    Box::new(e),
                )
            })?;
        }

        writer.flush().map_err(|e| {
            CliError::system_error_with_source(
                "Failed to finalize CSV file",
                Some("Check disk space and file permissions".to_string()),
                Box::new(e),
            )
        })?;

        Ok(())
    }

    /// Export status data (pets, devices) to CSV
    fn export_status_data_to_csv(data: &ExportData, path: &Path) -> Result<(), CliError> {
        let file = File::create(path).map_err(|e| {
            CliError::system_error_with_source(
                &format!("Failed to create CSV file: {}", path.display()),
                Some("Check file permissions and disk space".to_string()),
                Box::new(e),
            )
        })?;

        let mut writer = Writer::from_writer(file);

        if !data.pet_data.is_empty() {
            // Export pet status data
            writer.write_record(&[
                "pet_id",
                "name",
                "current_location",
                "last_activity",
                "last_feeding",
                "last_drinking",
            ]).map_err(|e| {
                CliError::system_error_with_source(
                    "Failed to write pet CSV header",
                    Some("Check disk space and file permissions".to_string()),
                    Box::new(e),
                )
            })?;

            for record in &data.pet_data {
                writer.write_record(&[
                    &record.pet_id.to_string(),
                    &record.name,
                    &record.current_location.map_or(String::new(), |l| l.to_string()),
                    &record.last_activity.map_or(String::new(), |t| t.to_rfc3339()),
                    &record.last_feeding.map_or(String::new(), |t| t.to_rfc3339()),
                    &record.last_drinking.map_or(String::new(), |t| t.to_rfc3339()),
                ]).map_err(|e| {
                    CliError::system_error_with_source(
                        "Failed to write pet data to CSV",
                        Some("Check disk space and file permissions".to_string()),
                        Box::new(e),
                    )
                })?;
            }
        } else if !data.device_data.is_empty() {
            // Export device status data
            writer.write_record(&[
                "device_id",
                "name",
                "device_type",
                "online",
                "battery_level",
                "lock_state",
            ]).map_err(|e| {
                CliError::system_error_with_source(
                    "Failed to write device CSV header",
                    Some("Check disk space and file permissions".to_string()),
                    Box::new(e),
                )
            })?;

            for record in &data.device_data {
                writer.write_record(&[
                    &record.device_id.to_string(),
                    &record.name,
                    &record.device_type,
                    &record.online.map_or(String::new(), |o| o.to_string()),
                    &record.battery_level.map_or(String::new(), |b| b.to_string()),
                    &record.lock_state.map_or(String::new(), |l| l.to_string()),
                ]).map_err(|e| {
                    CliError::system_error_with_source(
                        "Failed to write device data to CSV",
                        Some("Check disk space and file permissions".to_string()),
                        Box::new(e),
                    )
                })?;
            }
        }

        writer.flush().map_err(|e| {
            CliError::system_error_with_source(
                "Failed to finalize CSV file",
                Some("Check disk space and file permissions".to_string()),
                Box::new(e),
            )
        })?;

        Ok(())
    }

    /// Export data to JSON format
    pub fn export_to_json(data: &ExportData, path: &Path) -> Result<(), CliError> {
        // Create the directory if it doesn't exist
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                CliError::system_error_with_source(
                    &format!("Failed to create export directory: {}", parent.display()),
                    Some("Check directory permissions and try again".to_string()),
                    Box::new(e),
                )
            })?;
        }

        let json_data = serde_json::to_string_pretty(data).map_err(|e| {
            CliError::system_error_with_source(
                "Failed to serialize data to JSON",
                Some("Data may be corrupted or too large".to_string()),
                Box::new(e),
            )
        })?;

        std::fs::write(path, json_data).map_err(|e| {
            CliError::system_error_with_source(
                &format!("Failed to write JSON file: {}", path.display()),
                Some("Check file permissions and disk space".to_string()),
                Box::new(e),
            )
        })?;

        Ok(())
    }

    /// Generate a descriptive filename with timestamp
    pub fn generate_filename(
        base_name: &str,
        format: &ExportFormat,
        date_range: &DateRange,
    ) -> String {
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let from_date = date_range.from.format("%Y%m%d");
        let to_date = date_range.to.format("%Y%m%d");
        let extension = match format {
            ExportFormat::Csv => "csv",
            ExportFormat::Json => "json",
        };

        format!(
            "{}_{}_to_{}_{}.{}",
            base_name, from_date, to_date, timestamp, extension
        )
    }

    /// Convert feeding history to export records
    pub fn convert_feeding_history(history: &FeedingHistory) -> Vec<FeedingExportRecord> {
        history
            .events
            .iter()
            .map(|event| FeedingExportRecord {
                pet_id: history.pet_id,
                timestamp: event.timestamp,
                device_id: event.device_id,
                amount: event.amount,
                duration_seconds: event.duration,
            })
            .collect()
    }

    /// Convert drinking history to export records
    pub fn convert_drinking_history(history: &DrinkingHistory) -> Vec<DrinkingExportRecord> {
        history
            .events
            .iter()
            .map(|event| DrinkingExportRecord {
                pet_id: history.pet_id,
                timestamp: event.timestamp,
                device_id: event.device_id,
                volume: event.volume,
                duration_seconds: event.duration,
            })
            .collect()
    }

    /// Convert activity history to export records
    pub fn convert_activity_history(history: &ActivityHistory) -> Vec<ActivityExportRecord> {
        history
            .events
            .iter()
            .map(|event| ActivityExportRecord {
                pet_id: history.pet_id,
                timestamp: event.timestamp,
                event_type: format!("{:?}", event.event_type),
                location: event.location,
                device_id: event.device_id,
            })
            .collect()
    }

    /// Convert pets to export records
    pub fn convert_pets(pets: &[&Pet]) -> Vec<PetExportRecord> {
        pets.iter()
            .map(|pet| {
                let current_location = pet.position.as_ref().and_then(|p| p.location);
                
                let last_activity = pet.status.as_ref()
                    .and_then(|s| s.activity.as_ref())
                    .and_then(|a| a.since.parse::<DateTime<Utc>>().ok());
                
                let last_feeding = pet.status.as_ref()
                    .and_then(|s| s.feeding.as_ref())
                    .and_then(|f| f.at.parse::<DateTime<Utc>>().ok());
                
                let last_drinking = pet.status.as_ref()
                    .and_then(|s| s.drinking.as_ref())
                    .and_then(|d| d.at.parse::<DateTime<Utc>>().ok());

                PetExportRecord {
                    pet_id: pet.id,
                    name: pet.name.clone(),
                    current_location,
                    last_activity,
                    last_feeding,
                    last_drinking,
                }
            })
            .collect()
    }

    /// Convert devices to export records
    pub fn convert_devices(devices: &[&Device]) -> Vec<DeviceExportRecord> {
        devices
            .iter()
            .map(|device| {
                let device_type = match device.product_id {
                    1 => "Pet Flap",
                    6 => "Feeder",
                    8 => "Water Fountain",
                    _ => "Unknown",
                };

                let online = device.status.as_ref().and_then(|s| s.online);
                let battery_level = device.status.as_ref().and_then(|s| s.battery);
                let lock_state = device.status.as_ref()
                    .and_then(|s| s.locking.as_ref())
                    .map(|l| l.mode);

                DeviceExportRecord {
                    device_id: device.id,
                    name: device.name.clone(),
                    device_type: device_type.to_string(),
                    online,
                    battery_level,
                    lock_state,
                }
            })
            .collect()
    }

    /// Create export data structure from various inputs
    pub fn create_export_data(
        feeding_histories: Vec<&FeedingHistory>,
        drinking_histories: Vec<&DrinkingHistory>,
        activity_histories: Vec<&ActivityHistory>,
        pets: Vec<&Pet>,
        devices: Vec<&Device>,
        config: &ExportConfig,
    ) -> ExportData {
        let feeding_data: Vec<FeedingExportRecord> = feeding_histories
            .into_iter()
            .flat_map(Self::convert_feeding_history)
            .collect();

        let drinking_data: Vec<DrinkingExportRecord> = drinking_histories
            .into_iter()
            .flat_map(Self::convert_drinking_history)
            .collect();

        let activity_data: Vec<ActivityExportRecord> = activity_histories
            .into_iter()
            .flat_map(Self::convert_activity_history)
            .collect();

        let pet_data = Self::convert_pets(&pets);
        let device_data = Self::convert_devices(&devices);

        let total_records = feeding_data.len() + drinking_data.len() + activity_data.len() + pet_data.len() + device_data.len();

        let metadata = ExportMetadata {
            export_timestamp: Utc::now(),
            date_range: ExportDateRange {
                from: config.date_range.from,
                to: config.date_range.to,
            },
            included_pets: config.include_pets.clone(),
            data_types: config.data_types.iter().map(|dt| format!("{:?}", dt)).collect(),
            total_records,
        };

        ExportData {
            metadata,
            feeding_data,
            drinking_data,
            activity_data,
            pet_data,
            device_data,
        }
    }

    /// Generate a comprehensive report with key metrics and trends
    pub fn generate_report(
        feeding_histories: Vec<&FeedingHistory>,
        drinking_histories: Vec<&DrinkingHistory>,
        activity_histories: Vec<&ActivityHistory>,
        pets: Vec<&Pet>,
        devices: Vec<&Device>,
        config: &ReportConfig,
    ) -> Result<Report, CliError> {
        // Generate report metadata
        let pet_names: Vec<String> = pets.iter().map(|p| p.name.clone()).collect();
        let metadata = ReportMetadata {
            generated_at: Utc::now(),
            report_period: ExportDateRange {
                from: config.date_range.from,
                to: config.date_range.to,
            },
            pets_included: pet_names,
            total_pets: pets.len(),
            total_devices: devices.len(),
        };

        // Generate summary
        let summary = Self::generate_report_summary(
            &feeding_histories,
            &drinking_histories,
            &activity_histories,
            &pets,
            &devices,
        );

        // Generate trends if requested
        let trends = if config.include_trends && !feeding_histories.is_empty() {
            // Combine all feeding histories for trend analysis
            let combined_feeding = Self::combine_feeding_histories(&feeding_histories);
            let combined_drinking = Self::combine_drinking_histories(&drinking_histories);
            let combined_activity = Self::combine_activity_histories(&activity_histories);

            Some(TrendAnalysis {
                feeding_trends: DataProcessor::calculate_feeding_trends(&combined_feeding),
                drinking_trends: DataProcessor::calculate_drinking_trends(&combined_drinking),
                activity_trends: DataProcessor::calculate_activity_trends(&combined_activity),
            })
        } else {
            None
        };

        // Generate health metrics if requested
        let health_metrics = if config.include_health_metrics && !feeding_histories.is_empty() {
            let combined_feeding = Self::combine_feeding_histories(&feeding_histories);
            let combined_drinking = Self::combine_drinking_histories(&drinking_histories);
            let combined_activity = Self::combine_activity_histories(&activity_histories);

            Some(DataProcessor::calculate_health_metrics(
                &combined_feeding,
                &combined_drinking,
                &combined_activity,
            ))
        } else {
            None
        };

        // Generate alerts if requested
        let alerts = if config.include_alerts {
            Self::generate_report_alerts(&pets, &devices)
        } else {
            Vec::new()
        };

        // Generate recommendations based on the data
        let recommendations = Self::generate_recommendations(&summary, &trends, &health_metrics);

        Ok(Report {
            metadata,
            summary,
            trends,
            health_metrics,
            alerts,
            recommendations,
        })
    }

    /// Generate report summary with key metrics
    fn generate_report_summary(
        feeding_histories: &[&FeedingHistory],
        drinking_histories: &[&DrinkingHistory],
        activity_histories: &[&ActivityHistory],
        pets: &[&Pet],
        devices: &[&Device],
    ) -> ReportSummary {
        // Feeding summary
        let total_feeding_events: u32 = feeding_histories.iter().map(|h| h.events.len() as u32).sum();
        let total_food_consumed: f32 = feeding_histories
            .iter()
            .flat_map(|h| &h.events)
            .map(|e| e.amount)
            .sum();
        let average_daily_intake = if !feeding_histories.is_empty() {
            feeding_histories
                .iter()
                .filter_map(|h| h.summary.as_ref())
                .map(|s| s.daily_average)
                .sum::<f32>() / feeding_histories.len() as f32
        } else {
            0.0
        };

        // Find most active feeder
        let most_active_feeder = feeding_histories
            .iter()
            .max_by_key(|h| h.events.len())
            .and_then(|h| pets.iter().find(|p| p.id == h.pet_id))
            .map(|p| p.name.clone());

        let feeding_summary = FeedingReportSummary {
            total_feeding_events,
            total_food_consumed,
            average_daily_intake,
            most_active_feeder,
        };

        // Drinking summary
        let total_drinking_events: u32 = drinking_histories.iter().map(|h| h.events.len() as u32).sum();
        let total_water_consumed: f32 = drinking_histories
            .iter()
            .flat_map(|h| &h.events)
            .map(|e| e.volume)
            .sum();
        let average_daily_intake = if !drinking_histories.is_empty() {
            drinking_histories
                .iter()
                .filter_map(|h| h.summary.as_ref())
                .map(|s| s.daily_average)
                .sum::<f32>() / drinking_histories.len() as f32
        } else {
            0.0
        };

        let most_active_drinker = drinking_histories
            .iter()
            .max_by_key(|h| h.events.len())
            .and_then(|h| pets.iter().find(|p| p.id == h.pet_id))
            .map(|p| p.name.clone());

        let drinking_summary = DrinkingReportSummary {
            total_drinking_events,
            total_water_consumed,
            average_daily_intake,
            most_active_drinker,
        };

        // Activity summary
        let total_movements: u32 = activity_histories.iter().map(|h| h.events.len() as u32).sum();
        let total_entries: u32 = activity_histories
            .iter()
            .flat_map(|h| &h.events)
            .filter(|e| matches!(e.event_type, crate::api::client::ActivityType::Entry))
            .count() as u32;
        let total_exits: u32 = activity_histories
            .iter()
            .flat_map(|h| &h.events)
            .filter(|e| matches!(e.event_type, crate::api::client::ActivityType::Exit))
            .count() as u32;

        let most_active_pet = activity_histories
            .iter()
            .max_by_key(|h| h.events.len())
            .and_then(|h| pets.iter().find(|p| p.id == h.pet_id))
            .map(|p| p.name.clone());

        let activity_summary = ActivityReportSummary {
            total_movements,
            total_entries,
            total_exits,
            most_active_pet,
        };

        // Device summary
        let total_devices = devices.len() as u32;
        let online_devices = devices
            .iter()
            .filter(|d| d.status.as_ref().and_then(|s| s.online).unwrap_or(false))
            .count() as u32;
        let devices_with_low_battery = devices
            .iter()
            .filter(|d| {
                d.status
                    .as_ref()
                    .and_then(|s| s.battery)
                    .map_or(false, |b| b < 20.0)
            })
            .count() as u32;

        let battery_levels: Vec<f32> = devices
            .iter()
            .filter_map(|d| d.status.as_ref().and_then(|s| s.battery))
            .collect();
        let average_battery_level = if !battery_levels.is_empty() {
            Some(battery_levels.iter().sum::<f32>() / battery_levels.len() as f32)
        } else {
            None
        };

        let device_summary = DeviceReportSummary {
            total_devices,
            online_devices,
            devices_with_low_battery,
            average_battery_level,
        };

        ReportSummary {
            feeding_summary,
            drinking_summary,
            activity_summary,
            device_summary,
        }
    }

    /// Generate alerts for the report
    fn generate_report_alerts(pets: &[&Pet], devices: &[&Device]) -> Vec<String> {
        let mut alerts = Vec::new();

        // Check for inactive pets
        let inactive_config = crate::data_processor::InactivityConfig::default();
        let pets_owned: Vec<Pet> = pets.iter().map(|&p| p.clone()).collect();
        let pet_alerts = DataProcessor::identify_inactive_pets(
            &pets_owned,
            &inactive_config,
        );
        for alert in pet_alerts {
            alerts.push(alert.message);
        }

        // Check for device health issues
        let device_config = crate::data_processor::DeviceHealthConfig::default();
        let devices_owned: Vec<Device> = devices.iter().map(|&d| d.clone()).collect();
        let device_alerts = DataProcessor::generate_device_health_alerts(
            &devices_owned,
            &device_config,
        );
        for alert in device_alerts {
            alerts.push(alert.message);
        }

        alerts
    }

    /// Generate recommendations based on report data
    fn generate_recommendations(
        summary: &ReportSummary,
        trends: &Option<TrendAnalysis>,
        health_metrics: &Option<HealthMetrics>,
    ) -> Vec<String> {
        let mut recommendations = Vec::new();

        // Device recommendations
        if summary.device_summary.devices_with_low_battery > 0 {
            recommendations.push(format!(
                "Consider replacing batteries in {} device(s) with low battery levels",
                summary.device_summary.devices_with_low_battery
            ));
        }

        if summary.device_summary.online_devices < summary.device_summary.total_devices {
            let offline_count = summary.device_summary.total_devices - summary.device_summary.online_devices;
            recommendations.push(format!(
                "Check connectivity for {} offline device(s)",
                offline_count
            ));
        }

        // Health-based recommendations
        if let Some(health) = health_metrics {
            if health.overall_score < 0.7 {
                recommendations.push("Overall pet health metrics suggest monitoring feeding and activity patterns more closely".to_string());
            }

            if health.drinking_health.hydration_score < 0.6 {
                recommendations.push("Consider encouraging more water consumption - current levels may be below optimal".to_string());
            }

            if health.activity_health.daily_activity_score < 0.5 {
                recommendations.push("Pet activity levels appear low - consider encouraging more outdoor time or play".to_string());
            }
        }

        // Trend-based recommendations
        if let Some(trends) = trends {
            match trends.feeding_trends.trend_direction {
                crate::data_processor::TrendDirection::Decreasing => {
                    recommendations.push("Feeding amounts are trending downward - monitor pet appetite and consult vet if concerned".to_string());
                }
                crate::data_processor::TrendDirection::Increasing => {
                    recommendations.push("Feeding amounts are increasing - ensure portion control to maintain healthy weight".to_string());
                }
                _ => {}
            }

            if trends.feeding_trends.consistency_score < 0.5 {
                recommendations.push("Feeding patterns are inconsistent - consider establishing regular meal times".to_string());
            }
        }

        // Activity recommendations
        if summary.activity_summary.total_movements == 0 {
            recommendations.push("No activity recorded - ensure pet flap is functioning and pet has access".to_string());
        } else if summary.activity_summary.total_entries > summary.activity_summary.total_exits + 5 {
            recommendations.push("Pet appears to be spending more time indoors - consider encouraging outdoor activity".to_string());
        }

        recommendations
    }

    /// Combine multiple feeding histories into one for analysis
    fn combine_feeding_histories(histories: &[&FeedingHistory]) -> FeedingHistory {
        let mut all_events = Vec::new();
        let mut total_amount = 0.0;
        let mut total_events = 0;

        for history in histories {
            all_events.extend(history.events.clone());
            if let Some(summary) = &history.summary {
                total_amount += summary.total_amount;
                total_events += summary.event_count;
            }
        }

        // Sort events by timestamp
        all_events.sort_by_key(|e| e.timestamp);

        let summary = if total_events > 0 {
            Some(crate::api::client::FeedingSummary {
                total_amount,
                event_count: total_events,
                daily_average: total_amount / 7.0, // Assume week-long period
            })
        } else {
            None
        };

        FeedingHistory {
            pet_id: 0, // Combined data, no specific pet
            events: all_events,
            summary,
        }
    }

    /// Combine multiple drinking histories into one for analysis
    fn combine_drinking_histories(histories: &[&DrinkingHistory]) -> DrinkingHistory {
        let mut all_events = Vec::new();
        let mut total_volume = 0.0;
        let mut total_events = 0;

        for history in histories {
            all_events.extend(history.events.clone());
            if let Some(summary) = &history.summary {
                total_volume += summary.total_volume;
                total_events += summary.event_count;
            }
        }

        // Sort events by timestamp
        all_events.sort_by_key(|e| e.timestamp);

        let summary = if total_events > 0 {
            Some(crate::api::client::DrinkingSummary {
                total_volume,
                event_count: total_events,
                daily_average: total_volume / 7.0, // Assume week-long period
            })
        } else {
            None
        };

        DrinkingHistory {
            pet_id: 0, // Combined data, no specific pet
            events: all_events,
            summary,
        }
    }

    /// Combine multiple activity histories into one for analysis
    fn combine_activity_histories(histories: &[&ActivityHistory]) -> ActivityHistory {
        let mut all_events = Vec::new();
        let mut total_events = 0;
        let mut total_entries = 0;
        let mut total_exits = 0;

        for history in histories {
            all_events.extend(history.events.clone());
            if let Some(summary) = &history.summary {
                total_events += summary.total_events;
                total_entries += summary.entries;
                total_exits += summary.exits;
            }
        }

        // Sort events by timestamp
        all_events.sort_by_key(|e| e.timestamp);

        let summary = if total_events > 0 {
            Some(crate::api::client::ActivitySummary {
                total_events,
                entries: total_entries,
                exits: total_exits,
                feeding_sessions: 0, // Not tracked in combined summary
                drinking_sessions: 0, // Not tracked in combined summary
            })
        } else {
            None
        };

        ActivityHistory {
            pet_id: 0, // Combined data, no specific pet
            events: all_events,
            summary,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};
    use tempfile::tempdir;

    #[test]
    fn test_generate_filename() {
        let date_range = DateRange {
            from: Utc::now() - Duration::days(7),
            to: Utc::now(),
        };

        let filename = ExportManager::generate_filename("pet_data", &ExportFormat::Csv, &date_range);
        
        assert!(filename.starts_with("pet_data_"));
        assert!(filename.ends_with(".csv"));
        assert!(filename.contains("_to_"));
    }

    #[test]
    fn test_generate_filename_json() {
        let date_range = DateRange {
            from: Utc::now() - Duration::days(30),
            to: Utc::now(),
        };

        let filename = ExportManager::generate_filename("export", &ExportFormat::Json, &date_range);
        
        assert!(filename.starts_with("export_"));
        assert!(filename.ends_with(".json"));
        assert!(filename.contains("_to_"));
    }

    #[test]
    fn test_export_data_creation() {
        let config = ExportConfig {
            format: ExportFormat::Json,
            date_range: DateRange {
                from: Utc::now() - Duration::days(1),
                to: Utc::now(),
            },
            include_pets: vec![1, 2],
            data_types: vec![DataType::Feeding, DataType::Activity],
            output_path: PathBuf::from("test.json"),
        };

        let export_data = ExportManager::create_export_data(
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
            &config,
        );

        assert_eq!(export_data.metadata.included_pets, vec![1, 2]);
        assert_eq!(export_data.metadata.data_types.len(), 2);
        assert_eq!(export_data.metadata.total_records, 0);
    }

    #[test]
    fn test_export_to_json_empty_data() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test_export.json");

        let export_data = ExportData {
            metadata: ExportMetadata {
                export_timestamp: Utc::now(),
                date_range: ExportDateRange {
                    from: Utc::now() - Duration::days(1),
                    to: Utc::now(),
                },
                included_pets: vec![],
                data_types: vec![],
                total_records: 0,
            },
            feeding_data: vec![],
            drinking_data: vec![],
            activity_data: vec![],
            pet_data: vec![],
            device_data: vec![],
        };

        let result = ExportManager::export_to_json(&export_data, &file_path);
        assert!(result.is_ok());
        assert!(file_path.exists());

        // Verify the JSON content can be read back
        let content = std::fs::read_to_string(&file_path).unwrap();
        let parsed: ExportData = serde_json::from_str(&content).unwrap();
        assert_eq!(parsed.metadata.total_records, 0);
    }

    #[test]
    fn test_generate_report_empty_data() {
        let config = ReportConfig {
            include_trends: true,
            include_health_metrics: true,
            include_alerts: true,
            date_range: DateRange {
                from: Utc::now() - Duration::days(7),
                to: Utc::now(),
            },
        };

        let result = ExportManager::generate_report(
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
            &config,
        );

        assert!(result.is_ok());
        let report = result.unwrap();
        assert_eq!(report.metadata.total_pets, 0);
        assert_eq!(report.metadata.total_devices, 0);
        assert_eq!(report.summary.feeding_summary.total_feeding_events, 0);
        assert!(report.trends.is_none()); // No trends with empty data
        assert!(report.health_metrics.is_none()); // No health metrics with empty data
    }

    #[test]
    fn test_report_config_creation() {
        let config = ReportConfig {
            include_trends: true,
            include_health_metrics: false,
            include_alerts: true,
            date_range: DateRange {
                from: Utc::now() - Duration::days(30),
                to: Utc::now(),
            },
        };

        assert!(config.include_trends);
        assert!(!config.include_health_metrics);
        assert!(config.include_alerts);
    }

    #[test]
    fn test_combine_feeding_histories_empty() {
        let combined = ExportManager::combine_feeding_histories(&[]);
        assert_eq!(combined.events.len(), 0);
        assert!(combined.summary.is_none());
    }

    #[test]
    fn test_generate_recommendations_low_battery() {
        let summary = ReportSummary {
            feeding_summary: FeedingReportSummary {
                total_feeding_events: 0,
                total_food_consumed: 0.0,
                average_daily_intake: 0.0,
                most_active_feeder: None,
            },
            drinking_summary: DrinkingReportSummary {
                total_drinking_events: 0,
                total_water_consumed: 0.0,
                average_daily_intake: 0.0,
                most_active_drinker: None,
            },
            activity_summary: ActivityReportSummary {
                total_movements: 0,
                total_entries: 0,
                total_exits: 0,
                most_active_pet: None,
            },
            device_summary: DeviceReportSummary {
                total_devices: 2,
                online_devices: 2,
                devices_with_low_battery: 1,
                average_battery_level: Some(50.0),
            },
        };

        let recommendations = ExportManager::generate_recommendations(&summary, &None, &None);
        assert!(!recommendations.is_empty());
        assert!(recommendations.iter().any(|r| r.contains("battery")));
    }
}