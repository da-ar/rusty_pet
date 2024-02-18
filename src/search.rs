#![allow(dead_code)] // Module contains future functionality not yet integrated

use crate::api::client::{Pet, Device, FeedingHistory, DrinkingHistory, ActivityHistory, ActivityEvent, FeedingEvent, DrinkingEvent, DateRange};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Search and filtering functionality for pets, devices, and historical data
pub struct SearchManager;

/// Pet search criteria
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PetSearchCriteria {
    pub name_pattern: Option<String>,
    pub breed_pattern: Option<String>,
    pub characteristics: Option<Vec<String>>,
    pub location: Option<u32>,
    pub activity_since: Option<DateTime<Utc>>,
    pub inactive_threshold_hours: Option<u64>,
}

/// Device search criteria
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceSearchCriteria {
    pub name_pattern: Option<String>,
    pub device_type: Option<String>,
    pub online_status: Option<bool>,
    pub battery_threshold: Option<f32>,
}

/// Historical data search criteria
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricalSearchCriteria {
    pub date_range: Option<DateRange>,
    pub event_types: Option<Vec<String>>,
    pub amount_range: Option<(f32, f32)>,
    pub volume_range: Option<(f32, f32)>,
    pub device_ids: Option<Vec<u32>>,
    pub location: Option<u32>,
}

/// Search results with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResults<T> {
    pub results: Vec<T>,
    pub total_count: usize,
    pub active_filters: Vec<String>,
    pub search_metadata: SearchMetadata,
}

/// Metadata about the search operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchMetadata {
    pub search_duration_ms: u64,
    pub filters_applied: usize,
    pub original_count: usize,
}

/// Combined filter criteria for advanced filtering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombinedFilters {
    pub pet_filters: Option<PetSearchCriteria>,
    pub device_filters: Option<DeviceSearchCriteria>,
    pub historical_filters: Option<HistoricalSearchCriteria>,
    pub combine_with_and: bool, // true for AND logic, false for OR logic
}

/// Saved search configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedSearch {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub filters: CombinedFilters,
    pub created_at: DateTime<Utc>,
    pub last_used: Option<DateTime<Utc>>,
    pub use_count: u32,
}

impl SearchManager {
    /// Search pets by name, breed, or characteristics
    pub fn search_pets(pets: &[Pet], criteria: &PetSearchCriteria) -> SearchResults<Pet> {
        let start_time = std::time::Instant::now();
        let original_count = pets.len();
        let mut active_filters = Vec::new();
        
        let mut filtered_pets: Vec<Pet> = pets.to_vec();

        // Filter by name pattern
        if let Some(name_pattern) = &criteria.name_pattern {
            let pattern = name_pattern.to_lowercase();
            filtered_pets.retain(|pet| pet.name.to_lowercase().contains(&pattern));
            active_filters.push(format!("Name contains '{}'", name_pattern));
        }

        // Filter by breed pattern (using breed_id for now, could be enhanced with breed lookup)
        if let Some(breed_pattern) = &criteria.breed_pattern {
            // For now, we'll search in comments field as breed names aren't directly available
            filtered_pets.retain(|pet| {
                if let Some(comments) = &pet.comments {
                    comments.to_lowercase().contains(&breed_pattern.to_lowercase())
                } else {
                    false
                }
            });
            active_filters.push(format!("Breed contains '{}'", breed_pattern));
        }

        // Filter by characteristics (search in comments field)
        if let Some(characteristics) = &criteria.characteristics {
            filtered_pets.retain(|pet| {
                if let Some(comments) = &pet.comments {
                    let comments_lower = comments.to_lowercase();
                    characteristics.iter().any(|char| comments_lower.contains(&char.to_lowercase()))
                } else {
                    false
                }
            });
            active_filters.push(format!("Characteristics: {}", characteristics.join(", ")));
        }

        // Filter by location
        if let Some(location) = criteria.location {
            filtered_pets.retain(|pet| {
                if let Some(position) = &pet.position {
                    position.location == Some(location)
                } else if let Some(status) = &pet.status {
                    if let Some(activity) = &status.activity {
                        activity.location == location
                    } else {
                        false
                    }
                } else {
                    false
                }
            });
            let location_name = match location {
                1 => "Inside",
                2 => "Outside",
                _ => "Unknown",
            };
            active_filters.push(format!("Location: {}", location_name));
        }

        // Filter by activity since timestamp
        if let Some(activity_since) = criteria.activity_since {
            filtered_pets.retain(|pet| {
                Self::get_last_activity_time(pet)
                    .map(|last_activity| last_activity >= activity_since)
                    .unwrap_or(false)
            });
            active_filters.push(format!("Active since: {}", activity_since.format("%Y-%m-%d %H:%M")));
        }

        // Filter by inactivity threshold
        if let Some(threshold_hours) = criteria.inactive_threshold_hours {
            let threshold_time = Utc::now() - chrono::Duration::hours(threshold_hours as i64);
            filtered_pets.retain(|pet| {
                Self::get_last_activity_time(pet)
                    .map(|last_activity| last_activity <= threshold_time)
                    .unwrap_or(true) // Include pets with no activity data
            });
            active_filters.push(format!("Inactive for more than {} hours", threshold_hours));
        }

        let search_duration = start_time.elapsed();
        
        SearchResults {
            total_count: filtered_pets.len(),
            results: filtered_pets,
            active_filters: active_filters.clone(),
            search_metadata: SearchMetadata {
                search_duration_ms: search_duration.as_millis() as u64,
                filters_applied: active_filters.len(),
                original_count,
            },
        }
    }

    /// Search devices by various criteria
    pub fn search_devices(devices: &[Device], criteria: &DeviceSearchCriteria) -> SearchResults<Device> {
        let start_time = std::time::Instant::now();
        let original_count = devices.len();
        let mut active_filters = Vec::new();
        
        let mut filtered_devices: Vec<Device> = devices.to_vec();

        // Filter by name pattern
        if let Some(name_pattern) = &criteria.name_pattern {
            let pattern = name_pattern.to_lowercase();
            filtered_devices.retain(|device| device.name.to_lowercase().contains(&pattern));
            active_filters.push(format!("Name contains '{}'", name_pattern));
        }

        // Filter by device type (based on product_id or name patterns)
        if let Some(device_type) = &criteria.device_type {
            let type_pattern = device_type.to_lowercase();
            filtered_devices.retain(|device| {
                device.name.to_lowercase().contains(&type_pattern) ||
                // Common device type patterns
                match type_pattern.as_str() {
                    "flap" | "door" => device.name.to_lowercase().contains("flap") || device.name.to_lowercase().contains("door"),
                    "feeder" | "bowl" => device.name.to_lowercase().contains("feeder") || device.name.to_lowercase().contains("bowl"),
                    "fountain" | "water" => device.name.to_lowercase().contains("fountain") || device.name.to_lowercase().contains("water"),
                    _ => false,
                }
            });
            active_filters.push(format!("Device type: {}", device_type));
        }

        // Filter by online status
        if let Some(online_status) = criteria.online_status {
            filtered_devices.retain(|device| {
                device.status
                    .as_ref()
                    .and_then(|s| s.online)
                    .unwrap_or(false) == online_status
            });
            active_filters.push(format!("Online status: {}", if online_status { "Online" } else { "Offline" }));
        }

        // Filter by battery threshold
        if let Some(battery_threshold) = criteria.battery_threshold {
            filtered_devices.retain(|device| {
                device.status
                    .as_ref()
                    .and_then(|s| s.battery)
                    .map(|battery| (battery * 10.0) <= battery_threshold) // Convert API value (0-10) to percentage (0-100)
                    .unwrap_or(false)
            });
            active_filters.push(format!("Battery <= {:.1}%", battery_threshold));
        }

        let search_duration = start_time.elapsed();
        
        SearchResults {
            total_count: filtered_devices.len(),
            results: filtered_devices,
            active_filters: active_filters.clone(),
            search_metadata: SearchMetadata {
                search_duration_ms: search_duration.as_millis() as u64,
                filters_applied: active_filters.len(),
                original_count,
            },
        }
    }

    /// Search historical feeding data with multiple criteria
    pub fn search_feeding_history(
        histories: &[FeedingHistory], 
        criteria: &HistoricalSearchCriteria
    ) -> SearchResults<FeedingEvent> {
        let start_time = std::time::Instant::now();
        let mut active_filters = Vec::new();
        
        // Collect all events from all histories
        let all_events: Vec<FeedingEvent> = histories
            .iter()
            .flat_map(|h| h.events.iter().cloned())
            .collect();
        
        let original_count = all_events.len();
        let mut filtered_events = all_events;

        // Filter by date range
        if let Some(date_range) = &criteria.date_range {
            filtered_events.retain(|event| {
                event.timestamp >= date_range.from && event.timestamp <= date_range.to
            });
            active_filters.push(format!(
                "Date range: {} to {}", 
                date_range.from.format("%Y-%m-%d"), 
                date_range.to.format("%Y-%m-%d")
            ));
        }

        // Filter by amount range
        if let Some((min_amount, max_amount)) = criteria.amount_range {
            filtered_events.retain(|event| event.amount >= min_amount && event.amount <= max_amount);
            active_filters.push(format!("Amount: {:.1} - {:.1}", min_amount, max_amount));
        }

        // Filter by device IDs
        if let Some(device_ids) = &criteria.device_ids {
            filtered_events.retain(|event| device_ids.contains(&event.device_id));
            active_filters.push(format!("Devices: {:?}", device_ids));
        }

        let search_duration = start_time.elapsed();
        
        SearchResults {
            total_count: filtered_events.len(),
            results: filtered_events,
            active_filters: active_filters.clone(),
            search_metadata: SearchMetadata {
                search_duration_ms: search_duration.as_millis() as u64,
                filters_applied: active_filters.len(),
                original_count,
            },
        }
    }

    /// Search historical drinking data with multiple criteria
    pub fn search_drinking_history(
        histories: &[DrinkingHistory], 
        criteria: &HistoricalSearchCriteria
    ) -> SearchResults<DrinkingEvent> {
        let start_time = std::time::Instant::now();
        let mut active_filters = Vec::new();
        
        // Collect all events from all histories
        let all_events: Vec<DrinkingEvent> = histories
            .iter()
            .flat_map(|h| h.events.iter().cloned())
            .collect();
        
        let original_count = all_events.len();
        let mut filtered_events = all_events;

        // Filter by date range
        if let Some(date_range) = &criteria.date_range {
            filtered_events.retain(|event| {
                event.timestamp >= date_range.from && event.timestamp <= date_range.to
            });
            active_filters.push(format!(
                "Date range: {} to {}", 
                date_range.from.format("%Y-%m-%d"), 
                date_range.to.format("%Y-%m-%d")
            ));
        }

        // Filter by volume range
        if let Some((min_volume, max_volume)) = criteria.volume_range {
            filtered_events.retain(|event| event.volume >= min_volume && event.volume <= max_volume);
            active_filters.push(format!("Volume: {:.1} - {:.1}", min_volume, max_volume));
        }

        // Filter by device IDs
        if let Some(device_ids) = &criteria.device_ids {
            filtered_events.retain(|event| device_ids.contains(&event.device_id));
            active_filters.push(format!("Devices: {:?}", device_ids));
        }

        let search_duration = start_time.elapsed();
        
        SearchResults {
            total_count: filtered_events.len(),
            results: filtered_events,
            active_filters: active_filters.clone(),
            search_metadata: SearchMetadata {
                search_duration_ms: search_duration.as_millis() as u64,
                filters_applied: active_filters.len(),
                original_count,
            },
        }
    }

    /// Search historical activity data with multiple criteria
    pub fn search_activity_history(
        histories: &[ActivityHistory], 
        criteria: &HistoricalSearchCriteria
    ) -> SearchResults<ActivityEvent> {
        let start_time = std::time::Instant::now();
        let mut active_filters = Vec::new();
        
        // Collect all events from all histories
        let all_events: Vec<ActivityEvent> = histories
            .iter()
            .flat_map(|h| h.events.iter().cloned())
            .collect();
        
        let original_count = all_events.len();
        let mut filtered_events = all_events;

        // Filter by date range
        if let Some(date_range) = &criteria.date_range {
            filtered_events.retain(|event| {
                event.timestamp >= date_range.from && event.timestamp <= date_range.to
            });
            active_filters.push(format!(
                "Date range: {} to {}", 
                date_range.from.format("%Y-%m-%d"), 
                date_range.to.format("%Y-%m-%d")
            ));
        }

        // Filter by event types
        if let Some(event_types) = &criteria.event_types {
            filtered_events.retain(|event| {
                let event_type_str = match event.event_type {
                    crate::api::client::ActivityType::Entry => "entry",
                    crate::api::client::ActivityType::Exit => "exit",
                    crate::api::client::ActivityType::FeedingStart => "feeding_start",
                    crate::api::client::ActivityType::FeedingEnd => "feeding_end",
                    crate::api::client::ActivityType::DrinkingStart => "drinking_start",
                    crate::api::client::ActivityType::DrinkingEnd => "drinking_end",
                };
                event_types.iter().any(|et| et.to_lowercase() == event_type_str)
            });
            active_filters.push(format!("Event types: {}", event_types.join(", ")));
        }

        // Filter by location
        if let Some(location) = criteria.location {
            filtered_events.retain(|event| event.location == location);
            let location_name = match location {
                1 => "Inside",
                2 => "Outside",
                _ => "Unknown",
            };
            active_filters.push(format!("Location: {}", location_name));
        }

        // Filter by device IDs
        if let Some(device_ids) = &criteria.device_ids {
            filtered_events.retain(|event| {
                event.device_id.map(|id| device_ids.contains(&id)).unwrap_or(false)
            });
            active_filters.push(format!("Devices: {:?}", device_ids));
        }

        let search_duration = start_time.elapsed();
        
        SearchResults {
            total_count: filtered_events.len(),
            results: filtered_events,
            active_filters: active_filters.clone(),
            search_metadata: SearchMetadata {
                search_duration_ms: search_duration.as_millis() as u64,
                filters_applied: active_filters.len(),
                original_count,
            },
        }
    }

    /// Get the most recent activity timestamp for a pet (helper function)
    fn get_last_activity_time(pet: &Pet) -> Option<DateTime<Utc>> {
        let mut latest_time: Option<DateTime<Utc>> = None;
        
        // Check position timestamp
        if let Some(position) = &pet.position {
            if let Ok(timestamp) = position.since.parse::<DateTime<Utc>>() {
                latest_time = Some(latest_time.map_or(timestamp, |t| t.max(timestamp)));
            }
        }
        
        // Check status activity timestamp
        if let Some(status) = &pet.status {
            if let Some(activity) = &status.activity {
                if let Ok(timestamp) = activity.since.parse::<DateTime<Utc>>() {
                    latest_time = Some(latest_time.map_or(timestamp, |t| t.max(timestamp)));
                }
            }
            
            // Check feeding timestamp
            if let Some(feeding) = &status.feeding {
                if let Ok(timestamp) = feeding.at.parse::<DateTime<Utc>>() {
                    latest_time = Some(latest_time.map_or(timestamp, |t| t.max(timestamp)));
                }
            }
            
            // Check drinking timestamp
            if let Some(drinking) = &status.drinking {
                if let Ok(timestamp) = drinking.at.parse::<DateTime<Utc>>() {
                    latest_time = Some(latest_time.map_or(timestamp, |t| t.max(timestamp)));
                }
            }
        }
        
        latest_time
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::client::{Pet, Device, Position, Status, Activity, DeviceStatus, FeedingEvent, Tag, UsageStats};
    use chrono::{Utc, Duration};

    fn create_test_pet(id: u32, name: &str, location: Option<u32>) -> Pet {
        Pet {
            id,
            name: name.to_string(),
            gender: Some(0),
            date_of_birth: Some("2020-01-01".to_string()),
            weight: Some("4500".to_string()),
            breed: Some("Tabby".to_string()),
            comments: Some("Tabby cat, playful".to_string()),
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
                    location: location.unwrap_or(1),
                    since: Utc::now().to_rfc3339(),
                }),
                feeding: None,
                drinking: None,
            }),
            position: Some(Position {
                user_id: Some(1),
                tag_id: 123456,
                location,
                since: Utc::now().to_rfc3339(),
                version: Some(1),
                created_at: Some(Utc::now().to_rfc3339()),
                updated_at: Some(Utc::now().to_rfc3339()),
            }),
            tag: Some(Tag {
                id: 123456,
                index: Some(1),
                profile: Some(1),
            }),
        }
    }

    fn create_test_device(id: u32, name: &str, battery: Option<f32>, online: Option<bool>) -> Device {
        Device {
            id,
            name: name.to_string(),
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
                online,
                battery,
                learn_mode: None,
                signal_strength: Some(75.0),
                usage_stats: Some(UsageStats {
                    total_entries: 20,
                    total_exits: 18,
                    last_entry: None,
                    last_exit: None,
                    daily_average_entries: 4.0,
                }),
            }),
            control: None,
        }
    }

    #[test]
    fn test_search_pets_by_name() {
        let pets = vec![
            create_test_pet(1, "Fluffy", Some(1)),
            create_test_pet(2, "Whiskers", Some(2)),
            create_test_pet(3, "Mittens", Some(1)),
        ];

        let criteria = PetSearchCriteria {
            name_pattern: Some("flu".to_string()),
            breed_pattern: None,
            characteristics: None,
            location: None,
            activity_since: None,
            inactive_threshold_hours: None,
        };

        let results = SearchManager::search_pets(&pets, &criteria);
        assert_eq!(results.results.len(), 1);
        assert_eq!(results.results[0].name, "Fluffy");
        assert_eq!(results.active_filters.len(), 1);
        assert!(results.active_filters[0].contains("flu"));
    }

    #[test]
    fn test_search_pets_by_location() {
        let pets = vec![
            create_test_pet(1, "Indoor Cat", Some(1)),
            create_test_pet(2, "Outdoor Cat", Some(2)),
            create_test_pet(3, "Another Indoor", Some(1)),
        ];

        let criteria = PetSearchCriteria {
            name_pattern: None,
            breed_pattern: None,
            characteristics: None,
            location: Some(1), // Inside
            activity_since: None,
            inactive_threshold_hours: None,
        };

        let results = SearchManager::search_pets(&pets, &criteria);
        assert_eq!(results.results.len(), 2);
        assert!(results.active_filters[0].contains("Inside"));
    }

    #[test]
    fn test_search_devices_by_name() {
        let devices = vec![
            create_test_device(1, "Pet Flap", Some(80.0), Some(true)),
            create_test_device(2, "Water Fountain", Some(60.0), Some(true)),
            create_test_device(3, "Food Bowl", Some(40.0), Some(false)),
        ];

        let criteria = DeviceSearchCriteria {
            name_pattern: Some("flap".to_string()),
            device_type: None,
            online_status: None,
            battery_threshold: None,
        };

        let results = SearchManager::search_devices(&devices, &criteria);
        assert_eq!(results.results.len(), 1);
        assert_eq!(results.results[0].name, "Pet Flap");
    }

    #[test]
    fn test_search_devices_by_battery_threshold() {
        let devices = vec![
            create_test_device(1, "High Battery", Some(8.0), Some(true)), // 80%
            create_test_device(2, "Medium Battery", Some(5.0), Some(true)), // 50%
            create_test_device(3, "Low Battery", Some(2.0), Some(false)), // 20%
        ];

        let criteria = DeviceSearchCriteria {
            name_pattern: None,
            device_type: None,
            online_status: None,
            battery_threshold: Some(30.0), // 30% threshold
        };

        let results = SearchManager::search_devices(&devices, &criteria);
        assert_eq!(results.results.len(), 1);
        assert_eq!(results.results[0].name, "Low Battery");
    }

    #[test]
    fn test_search_feeding_history_by_amount_range() {
        let now = Utc::now();
        let histories = vec![
            FeedingHistory {
                pet_id: 1,
                events: vec![
                    FeedingEvent {
                        timestamp: now - Duration::hours(1),
                        device_id: 1,
                        amount: 50.0,
                        duration: None,
                    },
                    FeedingEvent {
                        timestamp: now - Duration::hours(2),
                        device_id: 1,
                        amount: 150.0,
                        duration: None,
                    },
                ],
                summary: None,
            },
        ];

        let criteria = HistoricalSearchCriteria {
            date_range: None,
            event_types: None,
            amount_range: Some((40.0, 100.0)),
            volume_range: None,
            device_ids: None,
            location: None,
        };

        let results = SearchManager::search_feeding_history(&histories, &criteria);
        assert_eq!(results.results.len(), 1);
        assert_eq!(results.results[0].amount, 50.0);
    }

    #[test]
    fn test_filter_pets_with_and_logic() {
        let pets = vec![
            create_test_pet(1, "Fluffy", Some(2)), // Outdoor - matches name but not location
            create_test_pet(2, "Outdoor Cat", Some(2)), // Outdoor - matches location but not name
            create_test_pet(3, "Indoor Fluffy", Some(1)), // Indoor - matches both name and location
        ];

        let criteria_list = vec![
            PetSearchCriteria {
                name_pattern: Some("fluffy".to_string()),
                breed_pattern: None,
                characteristics: None,
                location: None,
                activity_since: None,
                inactive_threshold_hours: None,
            },
            PetSearchCriteria {
                name_pattern: None,
                breed_pattern: None,
                characteristics: None,
                location: Some(1), // Indoor
                activity_since: None,
                inactive_threshold_hours: None,
            },
        ];

        let results = SearchManager::filter_pets_with_and_logic(&pets, &criteria_list);
        assert_eq!(results.results.len(), 1); // Only "Indoor Fluffy" matches both criteria
        assert_eq!(results.results[0].name, "Indoor Fluffy");
    }

    #[test]
    fn test_filter_pets_with_or_logic() {
        let pets = vec![
            create_test_pet(1, "Fluffy", Some(2)), // Outdoor
            create_test_pet(2, "Whiskers", Some(2)), // Outdoor
            create_test_pet(3, "Indoor Cat", Some(1)), // Indoor
        ];

        let criteria_list = vec![
            PetSearchCriteria {
                name_pattern: Some("fluffy".to_string()),
                breed_pattern: None,
                characteristics: None,
                location: None,
                activity_since: None,
                inactive_threshold_hours: None,
            },
            PetSearchCriteria {
                name_pattern: None,
                breed_pattern: None,
                characteristics: None,
                location: Some(1), // Indoor
                activity_since: None,
                inactive_threshold_hours: None,
            },
        ];

        let results = SearchManager::filter_pets_with_or_logic(&pets, &criteria_list);
        assert_eq!(results.results.len(), 2); // "Fluffy" OR "Indoor Cat"
    }

    #[test]
    fn test_sort_pets_by_name() {
        let mut pets = vec![
            create_test_pet(1, "Zebra", Some(1)),
            create_test_pet(2, "Alpha", Some(1)),
            create_test_pet(3, "Beta", Some(1)),
        ];

        SearchManager::sort_pets(&mut pets, PetSortBy::Name);
        assert_eq!(pets[0].name, "Alpha");
        assert_eq!(pets[1].name, "Beta");
        assert_eq!(pets[2].name, "Zebra");
    }

    #[test]
    fn test_create_filter_display_summary() {
        let pets = vec![create_test_pet(1, "Test", Some(1))];
        let criteria = PetSearchCriteria {
            name_pattern: Some("test".to_string()),
            breed_pattern: None,
            characteristics: None,
            location: None,
            activity_since: None,
            inactive_threshold_hours: None,
        };

        let results = SearchManager::search_pets(&pets, &criteria);
        let summary = SearchManager::create_filter_display_summary(&results);
        
        assert_eq!(summary.result_count, 1);
        assert_eq!(summary.original_count, 1);
        assert_eq!(summary.filters_applied, 1);
        assert_eq!(summary.filter_effectiveness, 100.0);
        assert!(!summary.active_filters.is_empty());
    }
}

impl SearchManager {
    /// Apply combined filters with AND/OR logic
    pub fn apply_combined_filters(
        pets: &[Pet],
        devices: &[Device],
        feeding_histories: &[FeedingHistory],
        drinking_histories: &[DrinkingHistory],
        activity_histories: &[ActivityHistory],
        filters: &CombinedFilters,
    ) -> CombinedSearchResults {
        let start_time = std::time::Instant::now();
        
        let mut pet_results = None;
        let mut device_results = None;
        let mut feeding_results = None;
        let mut drinking_results = None;
        let mut activity_results = None;
        
        // Apply individual filters
        if let Some(pet_filters) = &filters.pet_filters {
            pet_results = Some(Self::search_pets(pets, pet_filters));
        }
        
        if let Some(device_filters) = &filters.device_filters {
            device_results = Some(Self::search_devices(devices, device_filters));
        }
        
        if let Some(historical_filters) = &filters.historical_filters {
            feeding_results = Some(Self::search_feeding_history(feeding_histories, historical_filters));
            drinking_results = Some(Self::search_drinking_history(drinking_histories, historical_filters));
            activity_results = Some(Self::search_activity_history(activity_histories, historical_filters));
        }
        
        // Combine active filters from all searches
        let mut all_active_filters = Vec::new();
        let mut total_filters_applied = 0;
        
        if let Some(ref results) = pet_results {
            all_active_filters.extend(results.active_filters.clone());
            total_filters_applied += results.search_metadata.filters_applied;
        }
        
        if let Some(ref results) = device_results {
            all_active_filters.extend(results.active_filters.clone());
            total_filters_applied += results.search_metadata.filters_applied;
        }
        
        if let Some(ref results) = feeding_results {
            all_active_filters.extend(results.active_filters.clone());
            total_filters_applied += results.search_metadata.filters_applied;
        }
        
        if let Some(ref results) = drinking_results {
            all_active_filters.extend(results.active_filters.clone());
            total_filters_applied += results.search_metadata.filters_applied;
        }
        
        if let Some(ref results) = activity_results {
            all_active_filters.extend(results.active_filters.clone());
            total_filters_applied += results.search_metadata.filters_applied;
        }
        
        let search_duration = start_time.elapsed();
        
        CombinedSearchResults {
            pet_results,
            device_results,
            feeding_results,
            drinking_results,
            activity_results,
            combined_filters: all_active_filters,
            combination_logic: if filters.combine_with_and { "AND" } else { "OR" }.to_string(),
            search_metadata: SearchMetadata {
                search_duration_ms: search_duration.as_millis() as u64,
                filters_applied: total_filters_applied,
                original_count: pets.len() + devices.len() + 
                              feeding_histories.iter().map(|h| h.events.len()).sum::<usize>() +
                              drinking_histories.iter().map(|h| h.events.len()).sum::<usize>() +
                              activity_histories.iter().map(|h| h.events.len()).sum::<usize>(),
            },
        }
    }
    
    /// Filter pets with multiple criteria using AND logic
    pub fn filter_pets_with_and_logic(pets: &[Pet], criteria_list: &[PetSearchCriteria]) -> SearchResults<Pet> {
        let start_time = std::time::Instant::now();
        let original_count = pets.len();
        let mut all_active_filters = Vec::new();
        
        let mut filtered_pets: Vec<Pet> = pets.to_vec();
        
        // Apply each criteria set and intersect results
        for criteria in criteria_list {
            let individual_results = Self::search_pets(&filtered_pets, criteria);
            filtered_pets = individual_results.results;
            all_active_filters.extend(individual_results.active_filters);
        }
        
        let search_duration = start_time.elapsed();
        
        SearchResults {
            total_count: filtered_pets.len(),
            results: filtered_pets,
            active_filters: all_active_filters,
            search_metadata: SearchMetadata {
                search_duration_ms: search_duration.as_millis() as u64,
                filters_applied: criteria_list.len(),
                original_count,
            },
        }
    }
    
    /// Filter pets with multiple criteria using OR logic
    pub fn filter_pets_with_or_logic(pets: &[Pet], criteria_list: &[PetSearchCriteria]) -> SearchResults<Pet> {
        let start_time = std::time::Instant::now();
        let original_count = pets.len();
        let mut all_active_filters = Vec::new();
        let mut combined_pet_ids = std::collections::HashSet::new();
        
        // Apply each criteria set and union results
        for criteria in criteria_list {
            let individual_results = Self::search_pets(pets, criteria);
            for pet in &individual_results.results {
                combined_pet_ids.insert(pet.id);
            }
            all_active_filters.extend(individual_results.active_filters);
        }
        
        // Collect pets that match any criteria
        let filtered_pets: Vec<Pet> = pets
            .iter()
            .filter(|pet| combined_pet_ids.contains(&pet.id))
            .cloned()
            .collect();
        
        let search_duration = start_time.elapsed();
        
        SearchResults {
            total_count: filtered_pets.len(),
            results: filtered_pets,
            active_filters: all_active_filters,
            search_metadata: SearchMetadata {
                search_duration_ms: search_duration.as_millis() as u64,
                filters_applied: criteria_list.len(),
                original_count,
            },
        }
    }
    
    /// Create a filter display summary showing active filters and result counts
    pub fn create_filter_display_summary<T>(results: &SearchResults<T>) -> FilterDisplaySummary {
        FilterDisplaySummary {
            active_filters: results.active_filters.clone(),
            result_count: results.total_count,
            original_count: results.search_metadata.original_count,
            filters_applied: results.search_metadata.filters_applied,
            search_duration_ms: results.search_metadata.search_duration_ms,
            filter_effectiveness: if results.search_metadata.original_count > 0 {
                (results.total_count as f32 / results.search_metadata.original_count as f32) * 100.0
            } else {
                0.0
            },
        }
    }
    
    /// Sort pets by various criteria
    pub fn sort_pets(pets: &mut [Pet], sort_by: PetSortBy) {
        match sort_by {
            PetSortBy::Name => {
                pets.sort_by(|a, b| a.name.cmp(&b.name));
            }
            PetSortBy::LastActivity => {
                pets.sort_by(|a, b| {
                    let a_time = Self::get_last_activity_time(a);
                    let b_time = Self::get_last_activity_time(b);
                    b_time.cmp(&a_time) // Most recent first
                });
            }
            PetSortBy::Location => {
                pets.sort_by(|a, b| {
                    let a_location = a.position.as_ref().and_then(|p| p.location).unwrap_or(0);
                    let b_location = b.position.as_ref().and_then(|p| p.location).unwrap_or(0);
                    a_location.cmp(&b_location)
                });
            }
        }
    }
    
    /// Sort devices by various criteria
    pub fn sort_devices(devices: &mut [Device], sort_by: DeviceSortBy) {
        match sort_by {
            DeviceSortBy::Name => {
                devices.sort_by(|a, b| a.name.cmp(&b.name));
            }
            DeviceSortBy::BatteryLevel => {
                devices.sort_by(|a, b| {
                    let a_battery = a.status.as_ref().and_then(|s| s.battery).unwrap_or(0.0);
                    let b_battery = b.status.as_ref().and_then(|s| s.battery).unwrap_or(0.0);
                    a_battery.partial_cmp(&b_battery).unwrap_or(std::cmp::Ordering::Equal)
                });
            }
            DeviceSortBy::OnlineStatus => {
                devices.sort_by(|a, b| {
                    let a_online = a.status.as_ref().and_then(|s| s.online).unwrap_or(false);
                    let b_online = b.status.as_ref().and_then(|s| s.online).unwrap_or(false);
                    b_online.cmp(&a_online) // Online first
                });
            }
        }
    }
}

/// Combined search results from multiple data types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombinedSearchResults {
    pub pet_results: Option<SearchResults<Pet>>,
    pub device_results: Option<SearchResults<Device>>,
    pub feeding_results: Option<SearchResults<FeedingEvent>>,
    pub drinking_results: Option<SearchResults<DrinkingEvent>>,
    pub activity_results: Option<SearchResults<ActivityEvent>>,
    pub combined_filters: Vec<String>,
    pub combination_logic: String, // "AND" or "OR"
    pub search_metadata: SearchMetadata,
}

/// Filter display summary for showing active filters and results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterDisplaySummary {
    pub active_filters: Vec<String>,
    pub result_count: usize,
    pub original_count: usize,
    pub filters_applied: usize,
    pub search_duration_ms: u64,
    pub filter_effectiveness: f32, // Percentage of original results remaining
}

/// Pet sorting options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PetSortBy {
    Name,
    LastActivity,
    Location,
}

/// Device sorting options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeviceSortBy {
    Name,
    BatteryLevel,
    OnlineStatus,
}

/// Search persistence manager for saving and loading search configurations
pub struct SearchPersistenceManager {
    storage_path: std::path::PathBuf,
}

impl SearchPersistenceManager {
    /// Create a new search persistence manager
    pub fn new() -> Result<Self, std::io::Error> {
        let mut storage_path = dirs::home_dir().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, "Could not find home directory")
        })?;
        storage_path.push(".rusty_pet");
        storage_path.push("saved_searches.json");
        
        // Create directory if it doesn't exist
        if let Some(parent) = storage_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        Ok(Self { storage_path })
    }
    
    /// Save a search configuration
    pub fn save_search(&self, search: SavedSearch) -> Result<(), Box<dyn std::error::Error>> {
        let mut saved_searches = self.load_all_searches().unwrap_or_default();
        
        // Update existing search or add new one
        if let Some(existing) = saved_searches.iter_mut().find(|s| s.id == search.id) {
            *existing = search;
        } else {
            saved_searches.push(search);
        }
        
        let json_data = serde_json::to_string_pretty(&saved_searches)?;
        std::fs::write(&self.storage_path, json_data)?;
        
        Ok(())
    }
    
    /// Load a specific search by ID
    pub fn load_search(&self, search_id: &str) -> Result<Option<SavedSearch>, Box<dyn std::error::Error>> {
        let saved_searches = self.load_all_searches()?;
        Ok(saved_searches.into_iter().find(|s| s.id == search_id))
    }
    
    /// Load all saved searches
    pub fn load_all_searches(&self) -> Result<Vec<SavedSearch>, Box<dyn std::error::Error>> {
        if !self.storage_path.exists() {
            return Ok(Vec::new());
        }
        
        let json_data = std::fs::read_to_string(&self.storage_path)?;
        let saved_searches: Vec<SavedSearch> = serde_json::from_str(&json_data)?;
        Ok(saved_searches)
    }
    
    /// Delete a saved search
    pub fn delete_search(&self, search_id: &str) -> Result<bool, Box<dyn std::error::Error>> {
        let mut saved_searches = self.load_all_searches().unwrap_or_default();
        let original_len = saved_searches.len();
        
        saved_searches.retain(|s| s.id != search_id);
        
        if saved_searches.len() < original_len {
            let json_data = serde_json::to_string_pretty(&saved_searches)?;
            std::fs::write(&self.storage_path, json_data)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
    
    /// Update search usage statistics
    pub fn update_search_usage(&self, search_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut saved_searches = self.load_all_searches().unwrap_or_default();
        
        if let Some(search) = saved_searches.iter_mut().find(|s| s.id == search_id) {
            search.last_used = Some(Utc::now());
            search.use_count += 1;
            
            let json_data = serde_json::to_string_pretty(&saved_searches)?;
            std::fs::write(&self.storage_path, json_data)?;
        }
        
        Ok(())
    }
    
    /// Get frequently used searches (sorted by use count)
    pub fn get_frequently_used_searches(&self, limit: usize) -> Result<Vec<SavedSearch>, Box<dyn std::error::Error>> {
        let mut saved_searches = self.load_all_searches()?;
        saved_searches.sort_by(|a, b| b.use_count.cmp(&a.use_count));
        saved_searches.truncate(limit);
        Ok(saved_searches)
    }
    
    /// Get recently used searches (sorted by last used date)
    pub fn get_recently_used_searches(&self, limit: usize) -> Result<Vec<SavedSearch>, Box<dyn std::error::Error>> {
        let mut saved_searches = self.load_all_searches()?;
        saved_searches.sort_by(|a, b| {
            let a_time = a.last_used.unwrap_or(a.created_at);
            let b_time = b.last_used.unwrap_or(b.created_at);
            b_time.cmp(&a_time)
        });
        saved_searches.truncate(limit);
        Ok(saved_searches)
    }
    
    /// Create a new saved search with generated ID
    pub fn create_saved_search(
        name: String,
        description: Option<String>,
        filters: CombinedFilters,
    ) -> SavedSearch {
        let id = format!("search_{}", Utc::now().timestamp_millis());
        
        SavedSearch {
            id,
            name,
            description,
            filters,
            created_at: Utc::now(),
            last_used: None,
            use_count: 0,
        }
    }
    
    /// Export saved searches to a file
    pub fn export_searches(&self, export_path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
        let saved_searches = self.load_all_searches()?;
        let json_data = serde_json::to_string_pretty(&saved_searches)?;
        std::fs::write(export_path, json_data)?;
        Ok(())
    }
    
    /// Import saved searches from a file
    pub fn import_searches(&self, import_path: &std::path::Path) -> Result<usize, Box<dyn std::error::Error>> {
        let json_data = std::fs::read_to_string(import_path)?;
        let imported_searches: Vec<SavedSearch> = serde_json::from_str(&json_data)?;
        
        let mut existing_searches = self.load_all_searches().unwrap_or_default();
        let mut imported_count = 0;
        
        for imported_search in imported_searches {
            // Check if search with same ID already exists
            if !existing_searches.iter().any(|s| s.id == imported_search.id) {
                existing_searches.push(imported_search);
                imported_count += 1;
            }
        }
        
        let json_data = serde_json::to_string_pretty(&existing_searches)?;
        std::fs::write(&self.storage_path, json_data)?;
        
        Ok(imported_count)
    }
}

impl SearchManager {
    /// Execute a saved search on current data
    pub fn execute_saved_search(
        pets: &[Pet],
        devices: &[Device],
        feeding_histories: &[FeedingHistory],
        drinking_histories: &[DrinkingHistory],
        activity_histories: &[ActivityHistory],
        saved_search: &SavedSearch,
        persistence_manager: &SearchPersistenceManager,
    ) -> Result<CombinedSearchResults, Box<dyn std::error::Error>> {
        // Update usage statistics
        persistence_manager.update_search_usage(&saved_search.id)?;
        
        // Execute the search
        let results = Self::apply_combined_filters(
            pets,
            devices,
            feeding_histories,
            drinking_histories,
            activity_histories,
            &saved_search.filters,
        );
        
        Ok(results)
    }
    
    /// Create a quick search from simple criteria
    pub fn create_quick_search(
        name_pattern: Option<String>,
        location: Option<u32>,
        device_type: Option<String>,
    ) -> CombinedFilters {
        let pet_filters = if name_pattern.is_some() || location.is_some() {
            Some(PetSearchCriteria {
                name_pattern,
                breed_pattern: None,
                characteristics: None,
                location,
                activity_since: None,
                inactive_threshold_hours: None,
            })
        } else {
            None
        };
        
        let device_filters = if device_type.is_some() {
            Some(DeviceSearchCriteria {
                name_pattern: None,
                device_type,
                online_status: None,
                battery_threshold: None,
            })
        } else {
            None
        };
        
        CombinedFilters {
            pet_filters,
            device_filters,
            historical_filters: None,
            combine_with_and: true,
        }
    }
}

#[cfg(test)]
mod persistence_tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_persistence_manager() -> (SearchPersistenceManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let mut storage_path = temp_dir.path().to_path_buf();
        storage_path.push("saved_searches.json");
        
        let manager = SearchPersistenceManager {
            storage_path,
        };
        
        (manager, temp_dir)
    }

    #[test]
    fn test_save_and_load_search() {
        let (manager, _temp_dir) = create_test_persistence_manager();
        
        let filters = CombinedFilters {
            pet_filters: Some(PetSearchCriteria {
                name_pattern: Some("test".to_string()),
                breed_pattern: None,
                characteristics: None,
                location: None,
                activity_since: None,
                inactive_threshold_hours: None,
            }),
            device_filters: None,
            historical_filters: None,
            combine_with_and: true,
        };
        
        let saved_search = SearchPersistenceManager::create_saved_search(
            "Test Search".to_string(),
            Some("A test search".to_string()),
            filters,
        );
        
        let search_id = saved_search.id.clone();
        
        // Save the search
        manager.save_search(saved_search).unwrap();
        
        // Load the search
        let loaded_search = manager.load_search(&search_id).unwrap();
        assert!(loaded_search.is_some());
        
        let loaded_search = loaded_search.unwrap();
        assert_eq!(loaded_search.name, "Test Search");
        assert_eq!(loaded_search.description, Some("A test search".to_string()));
    }

    #[test]
    fn test_delete_search() {
        let (manager, _temp_dir) = create_test_persistence_manager();
        
        let filters = CombinedFilters {
            pet_filters: Some(PetSearchCriteria {
                name_pattern: Some("test".to_string()),
                breed_pattern: None,
                characteristics: None,
                location: None,
                activity_since: None,
                inactive_threshold_hours: None,
            }),
            device_filters: None,
            historical_filters: None,
            combine_with_and: true,
        };
        
        let saved_search = SearchPersistenceManager::create_saved_search(
            "Test Search".to_string(),
            None,
            filters,
        );
        
        let search_id = saved_search.id.clone();
        
        // Save and then delete
        manager.save_search(saved_search).unwrap();
        let deleted = manager.delete_search(&search_id).unwrap();
        assert!(deleted);
        
        // Verify it's gone
        let loaded_search = manager.load_search(&search_id).unwrap();
        assert!(loaded_search.is_none());
    }

    #[test]
    fn test_update_search_usage() {
        let (manager, _temp_dir) = create_test_persistence_manager();
        
        let filters = CombinedFilters {
            pet_filters: Some(PetSearchCriteria {
                name_pattern: Some("test".to_string()),
                breed_pattern: None,
                characteristics: None,
                location: None,
                activity_since: None,
                inactive_threshold_hours: None,
            }),
            device_filters: None,
            historical_filters: None,
            combine_with_and: true,
        };
        
        let saved_search = SearchPersistenceManager::create_saved_search(
            "Test Search".to_string(),
            None,
            filters,
        );
        
        let search_id = saved_search.id.clone();
        
        // Save the search
        manager.save_search(saved_search).unwrap();
        
        // Update usage
        manager.update_search_usage(&search_id).unwrap();
        
        // Verify usage was updated
        let loaded_search = manager.load_search(&search_id).unwrap().unwrap();
        assert_eq!(loaded_search.use_count, 1);
        assert!(loaded_search.last_used.is_some());
    }

    #[test]
    fn test_create_quick_search() {
        let filters = SearchManager::create_quick_search(
            Some("fluffy".to_string()),
            Some(1),
            Some("flap".to_string()),
        );
        
        assert!(filters.pet_filters.is_some());
        assert!(filters.device_filters.is_some());
        assert!(filters.combine_with_and);
        
        let pet_filters = filters.pet_filters.unwrap();
        assert_eq!(pet_filters.name_pattern, Some("fluffy".to_string()));
        assert_eq!(pet_filters.location, Some(1));
        
        let device_filters = filters.device_filters.unwrap();
        assert_eq!(device_filters.device_type, Some("flap".to_string()));
    }
}