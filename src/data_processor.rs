#![allow(dead_code)] // Module contains future functionality not yet integrated

use crate::api::client::{FeedingHistory, DrinkingHistory, ActivityHistory, ActivityEvent, Pet, Device};
use chrono::{DateTime, Utc, Datelike, Timelike};
use serde::{Deserialize, Serialize};

/// Data processor for calculating averages, trends, and health metrics
pub struct DataProcessor;

/// Alert system for pet and device monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub alert_type: AlertType,
    pub severity: AlertSeverity,
    pub message: String,
    pub pet_id: Option<u32>,
    pub device_id: Option<u32>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertType {
    InactivePet,
    LowBattery,
    DeviceOffline,
    ConnectivityIssue,
    HealthConcern,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Configuration for inactive pet detection
#[derive(Debug, Clone)]
pub struct InactivityConfig {
    pub inactive_threshold_hours: u64,
    pub critical_threshold_hours: u64,
}

impl Default for InactivityConfig {
    fn default() -> Self {
        Self {
            inactive_threshold_hours: 12, // 12 hours without activity is concerning
            critical_threshold_hours: 24, // 24 hours is critical
        }
    }
}

/// Configuration for device health monitoring
#[derive(Debug, Clone)]
pub struct DeviceHealthConfig {
    pub low_battery_threshold: f32,
    pub critical_battery_threshold: f32,
}

impl Default for DeviceHealthConfig {
    fn default() -> Self {
        Self {
            low_battery_threshold: 20.0, // 20% battery is low
            critical_battery_threshold: 10.0, // 10% battery is critical
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendAnalysis {
    pub feeding_trends: FeedingTrends,
    pub drinking_trends: DrinkingTrends,
    pub activity_trends: ActivityTrends,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedingTrends {
    pub daily_average: f32,
    pub weekly_average: f32,
    pub trend_direction: TrendDirection,
    pub consistency_score: f32, // 0.0 to 1.0, higher means more consistent
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrinkingTrends {
    pub daily_average: f32,
    pub weekly_average: f32,
    pub trend_direction: TrendDirection,
    pub consistency_score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityTrends {
    pub daily_entries: f32,
    pub daily_exits: f32,
    pub activity_pattern: ActivityPattern,
    pub most_active_hours: Vec<u32>, // Hours of day (0-23)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrendDirection {
    Increasing,
    Decreasing,
    Stable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActivityPattern {
    Regular,
    Irregular,
    Nocturnal,
    Diurnal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthMetrics {
    pub feeding_health: FeedingHealthMetrics,
    pub drinking_health: DrinkingHealthMetrics,
    pub activity_health: ActivityHealthMetrics,
    pub overall_score: f32, // 0.0 to 1.0, higher is better
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedingHealthMetrics {
    pub daily_intake_average: f32,
    pub feeding_frequency: f32, // Meals per day
    pub regularity_score: f32, // How regular feeding times are
    pub portion_consistency: f32, // How consistent portion sizes are
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrinkingHealthMetrics {
    pub daily_intake_average: f32,
    pub drinking_frequency: f32, // Drinks per day
    pub hydration_score: f32, // Based on intake vs recommended
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityHealthMetrics {
    pub daily_activity_score: f32, // Based on entries/exits
    pub exercise_consistency: f32, // How consistent daily activity is
    pub indoor_outdoor_balance: f32, // Balance between inside/outside time
}

impl DataProcessor {
    /// Identify inactive pets based on their last activity timestamp
    pub fn identify_inactive_pets(pets: &[Pet], config: &InactivityConfig) -> Vec<Alert> {
        let mut alerts = Vec::new();
        let now = Utc::now();
        
        for pet in pets {
            let last_activity_time = Self::get_last_activity_time(pet);
            
            if let Some(last_activity) = last_activity_time {
                let hours_since_activity = (now - last_activity).num_hours();
                
                if hours_since_activity >= config.critical_threshold_hours as i64 {
                    alerts.push(Alert {
                        alert_type: AlertType::InactivePet,
                        severity: AlertSeverity::Critical,
                        message: format!(
                            "Pet '{}' has been inactive for {} hours (last seen: {})",
                            pet.name,
                            hours_since_activity,
                            last_activity.format("%Y-%m-%d %H:%M:%S UTC")
                        ),
                        pet_id: Some(pet.id),
                        device_id: None,
                        timestamp: now,
                    });
                } else if hours_since_activity >= config.inactive_threshold_hours as i64 {
                    alerts.push(Alert {
                        alert_type: AlertType::InactivePet,
                        severity: AlertSeverity::Medium,
                        message: format!(
                            "Pet '{}' has been inactive for {} hours (last seen: {})",
                            pet.name,
                            hours_since_activity,
                            last_activity.format("%Y-%m-%d %H:%M:%S UTC")
                        ),
                        pet_id: Some(pet.id),
                        device_id: None,
                        timestamp: now,
                    });
                }
            } else {
                // No activity data available
                alerts.push(Alert {
                    alert_type: AlertType::InactivePet,
                    severity: AlertSeverity::High,
                    message: format!("Pet '{}' has no recent activity data available", pet.name),
                    pet_id: Some(pet.id),
                    device_id: None,
                    timestamp: now,
                });
            }
        }
        
        alerts
    }

    /// Generate device health alerts for low battery and connectivity issues
    pub fn generate_device_health_alerts(devices: &[Device], config: &DeviceHealthConfig) -> Vec<Alert> {
        let mut alerts = Vec::new();
        let now = Utc::now();
        
        for device in devices {
            if let Some(status) = &device.status {
                // Check battery level
                if let Some(battery) = status.battery {
                    let battery_percentage = battery * 10.0; // Convert API value (0-10) to percentage (0-100)
                    if battery_percentage <= config.critical_battery_threshold {
                        alerts.push(Alert {
                            alert_type: AlertType::LowBattery,
                            severity: AlertSeverity::Critical,
                            message: format!(
                                "Device '{}' has critically low battery: {:.1}%",
                                device.name, battery_percentage
                            ),
                            pet_id: None,
                            device_id: Some(device.id),
                            timestamp: now,
                        });
                    } else if battery_percentage <= config.low_battery_threshold {
                        alerts.push(Alert {
                            alert_type: AlertType::LowBattery,
                            severity: AlertSeverity::Medium,
                            message: format!(
                                "Device '{}' has low battery: {:.1}%",
                                device.name, battery_percentage
                            ),
                            pet_id: None,
                            device_id: Some(device.id),
                            timestamp: now,
                        });
                    }
                }
                
                // Check connectivity status
                if let Some(online) = status.online {
                    if !online {
                        alerts.push(Alert {
                            alert_type: AlertType::DeviceOffline,
                            severity: AlertSeverity::High,
                            message: format!("Device '{}' is offline", device.name),
                            pet_id: None,
                            device_id: Some(device.id),
                            timestamp: now,
                        });
                    }
                }
            } else {
                // No status information available
                alerts.push(Alert {
                    alert_type: AlertType::ConnectivityIssue,
                    severity: AlertSeverity::Medium,
                    message: format!("Device '{}' status information unavailable", device.name),
                    pet_id: None,
                    device_id: Some(device.id),
                    timestamp: now,
                });
            }
        }
        
        alerts
    }

    /// Check if a pet is inactive based on configuration
    pub fn is_pet_inactive(pet: &Pet, config: &InactivityConfig) -> bool {
        let last_activity_time = Self::get_last_activity_time(pet);
        
        if let Some(last_activity) = last_activity_time {
            let hours_since_activity = (Utc::now() - last_activity).num_hours();
            hours_since_activity >= config.inactive_threshold_hours as i64
        } else {
            true // No activity data means inactive
        }
    }

    /// Get the most recent activity timestamp for a pet
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
    /// Calculate feeding trends from feeding history
    pub fn calculate_feeding_trends(history: &FeedingHistory) -> FeedingTrends {
        if history.events.is_empty() {
            return FeedingTrends {
                daily_average: 0.0,
                weekly_average: 0.0,
                trend_direction: TrendDirection::Stable,
                consistency_score: 0.0,
            };
        }

        let total_amount: f32 = history.events.iter().map(|e| e.amount).sum();
        let days = Self::calculate_days_span(&history.events.iter().map(|e| e.timestamp).collect::<Vec<_>>());
        let daily_average = total_amount / days.max(1.0);
        let weekly_average = daily_average * 7.0;

        // Calculate trend direction by comparing first and second half
        let mid_point = history.events.len() / 2;
        let first_half_avg = if mid_point > 0 {
            history.events[..mid_point].iter().map(|e| e.amount).sum::<f32>() / mid_point as f32
        } else {
            0.0
        };
        let second_half_avg = if history.events.len() > mid_point {
            history.events[mid_point..].iter().map(|e| e.amount).sum::<f32>() / (history.events.len() - mid_point) as f32
        } else {
            0.0
        };

        let trend_direction = if second_half_avg > first_half_avg * 1.1 {
            TrendDirection::Increasing
        } else if second_half_avg < first_half_avg * 0.9 {
            TrendDirection::Decreasing
        } else {
            TrendDirection::Stable
        };

        // Calculate consistency score based on variance
        let consistency_score = Self::calculate_consistency_score(&history.events.iter().map(|e| e.amount).collect::<Vec<_>>());

        FeedingTrends {
            daily_average,
            weekly_average,
            trend_direction,
            consistency_score,
        }
    }

    /// Calculate drinking trends from drinking history
    pub fn calculate_drinking_trends(history: &DrinkingHistory) -> DrinkingTrends {
        if history.events.is_empty() {
            return DrinkingTrends {
                daily_average: 0.0,
                weekly_average: 0.0,
                trend_direction: TrendDirection::Stable,
                consistency_score: 0.0,
            };
        }

        let total_volume: f32 = history.events.iter().map(|e| e.volume).sum();
        let days = Self::calculate_days_span(&history.events.iter().map(|e| e.timestamp).collect::<Vec<_>>());
        let daily_average = total_volume / days.max(1.0);
        let weekly_average = daily_average * 7.0;

        // Calculate trend direction
        let mid_point = history.events.len() / 2;
        let first_half_avg = if mid_point > 0 {
            history.events[..mid_point].iter().map(|e| e.volume).sum::<f32>() / mid_point as f32
        } else {
            0.0
        };
        let second_half_avg = if history.events.len() > mid_point {
            history.events[mid_point..].iter().map(|e| e.volume).sum::<f32>() / (history.events.len() - mid_point) as f32
        } else {
            0.0
        };

        let trend_direction = if second_half_avg > first_half_avg * 1.1 {
            TrendDirection::Increasing
        } else if second_half_avg < first_half_avg * 0.9 {
            TrendDirection::Decreasing
        } else {
            TrendDirection::Stable
        };

        let consistency_score = Self::calculate_consistency_score(&history.events.iter().map(|e| e.volume).collect::<Vec<_>>());

        DrinkingTrends {
            daily_average,
            weekly_average,
            trend_direction,
            consistency_score,
        }
    }

    /// Calculate activity trends from activity history
    pub fn calculate_activity_trends(history: &ActivityHistory) -> ActivityTrends {
        if history.events.is_empty() {
            return ActivityTrends {
                daily_entries: 0.0,
                daily_exits: 0.0,
                activity_pattern: ActivityPattern::Regular,
                most_active_hours: Vec::new(),
            };
        }

        let days = Self::calculate_days_span(&history.events.iter().map(|e| e.timestamp).collect::<Vec<_>>());
        
        let entries = history.events.iter().filter(|e| matches!(e.event_type, crate::api::client::ActivityType::Entry)).count() as f32;
        let exits = history.events.iter().filter(|e| matches!(e.event_type, crate::api::client::ActivityType::Exit)).count() as f32;
        
        let daily_entries = entries / days.max(1.0);
        let daily_exits = exits / days.max(1.0);

        // Determine activity pattern based on time distribution
        let mut hour_counts = vec![0u32; 24];
        for event in &history.events {
            let hour = event.timestamp.hour();
            hour_counts[hour as usize] += 1;
        }

        // Find most active hours (top 3)
        let mut hour_activity: Vec<(usize, u32)> = hour_counts.iter().enumerate().map(|(h, &count)| (h, count)).collect();
        hour_activity.sort_by(|a, b| b.1.cmp(&a.1));
        let most_active_hours: Vec<u32> = hour_activity.iter().take(3).map(|(h, _)| *h as u32).collect();

        // Determine activity pattern
        let night_activity = (22..24).chain(0..6).map(|h| hour_counts[h]).sum::<u32>();
        let day_activity = (6..22).map(|h| hour_counts[h]).sum::<u32>();
        let total_activity = night_activity + day_activity;

        let activity_pattern = if total_activity == 0 {
            ActivityPattern::Regular
        } else if night_activity as f32 / total_activity as f32 > 0.6 {
            ActivityPattern::Nocturnal
        } else if day_activity as f32 / total_activity as f32 > 0.8 {
            ActivityPattern::Diurnal
        } else {
            // Check for regularity by looking at variance in daily activity
            let daily_variance = Self::calculate_daily_activity_variance(&history.events);
            if daily_variance < 0.3 {
                ActivityPattern::Regular
            } else {
                ActivityPattern::Irregular
            }
        };

        ActivityTrends {
            daily_entries,
            daily_exits,
            activity_pattern,
            most_active_hours,
        }
    }

    /// Calculate comprehensive health metrics
    pub fn calculate_health_metrics(
        feeding_history: &FeedingHistory,
        drinking_history: &DrinkingHistory,
        activity_history: &ActivityHistory,
    ) -> HealthMetrics {
        let feeding_health = Self::calculate_feeding_health_metrics(feeding_history);
        let drinking_health = Self::calculate_drinking_health_metrics(drinking_history);
        let activity_health = Self::calculate_activity_health_metrics(activity_history);

        // Calculate overall score as weighted average
        let overall_score = (feeding_health.regularity_score * 0.3 + 
                           drinking_health.hydration_score * 0.3 + 
                           activity_health.daily_activity_score * 0.4).min(1.0);

        HealthMetrics {
            feeding_health,
            drinking_health,
            activity_health,
            overall_score,
        }
    }

    /// Calculate feeding health metrics
    fn calculate_feeding_health_metrics(history: &FeedingHistory) -> FeedingHealthMetrics {
        if history.events.is_empty() {
            return FeedingHealthMetrics {
                daily_intake_average: 0.0,
                feeding_frequency: 0.0,
                regularity_score: 0.0,
                portion_consistency: 0.0,
            };
        }

        let total_amount: f32 = history.events.iter().map(|e| e.amount).sum();
        let days = Self::calculate_days_span(&history.events.iter().map(|e| e.timestamp).collect::<Vec<_>>());
        let daily_intake_average = total_amount / days.max(1.0);
        let feeding_frequency = history.events.len() as f32 / days.max(1.0);

        // Calculate regularity score based on time consistency
        let regularity_score = Self::calculate_time_regularity_score(&history.events.iter().map(|e| e.timestamp).collect::<Vec<_>>());
        
        // Calculate portion consistency
        let portion_consistency = Self::calculate_consistency_score(&history.events.iter().map(|e| e.amount).collect::<Vec<_>>());

        FeedingHealthMetrics {
            daily_intake_average,
            feeding_frequency,
            regularity_score,
            portion_consistency,
        }
    }

    /// Calculate drinking health metrics
    fn calculate_drinking_health_metrics(history: &DrinkingHistory) -> DrinkingHealthMetrics {
        if history.events.is_empty() {
            return DrinkingHealthMetrics {
                daily_intake_average: 0.0,
                drinking_frequency: 0.0,
                hydration_score: 0.0,
            };
        }

        let total_volume: f32 = history.events.iter().map(|e| e.volume).sum();
        let days = Self::calculate_days_span(&history.events.iter().map(|e| e.timestamp).collect::<Vec<_>>());
        let daily_intake_average = total_volume / days.max(1.0);
        let drinking_frequency = history.events.len() as f32 / days.max(1.0);

        // Simple hydration score based on intake (assuming 200ml/day is good for average cat)
        let hydration_score = (daily_intake_average / 200.0).min(1.0);

        DrinkingHealthMetrics {
            daily_intake_average,
            drinking_frequency,
            hydration_score,
        }
    }

    /// Calculate activity health metrics
    fn calculate_activity_health_metrics(history: &ActivityHistory) -> ActivityHealthMetrics {
        if history.events.is_empty() {
            return ActivityHealthMetrics {
                daily_activity_score: 0.0,
                exercise_consistency: 0.0,
                indoor_outdoor_balance: 0.0,
            };
        }

        let days = Self::calculate_days_span(&history.events.iter().map(|e| e.timestamp).collect::<Vec<_>>());
        let total_movements = history.events.len() as f32;
        let daily_activity_score = (total_movements / days.max(1.0) / 10.0).min(1.0); // Normalize to 10 movements per day

        // Calculate exercise consistency
        let exercise_consistency = Self::calculate_daily_activity_variance(&history.events);

        // Calculate indoor/outdoor balance
        let inside_events = history.events.iter().filter(|e| e.location == crate::api::client::LOCATION_INSIDE).count() as f32;
        let outside_events = history.events.iter().filter(|e| e.location == crate::api::client::LOCATION_OUTSIDE).count() as f32;
        let total_location_events = inside_events + outside_events;
        
        let indoor_outdoor_balance = if total_location_events > 0.0 {
            1.0 - ((inside_events - outside_events).abs() / total_location_events)
        } else {
            0.0
        };

        ActivityHealthMetrics {
            daily_activity_score,
            exercise_consistency,
            indoor_outdoor_balance,
        }
    }

    /// Helper function to calculate the span of days from timestamps
    fn calculate_days_span(timestamps: &[DateTime<Utc>]) -> f32 {
        if timestamps.len() < 2 {
            return 1.0;
        }

        let min_time = timestamps.iter().min().unwrap();
        let max_time = timestamps.iter().max().unwrap();
        let duration = *max_time - *min_time;
        duration.num_days().max(1) as f32
    }

    /// Calculate consistency score (0.0 to 1.0, higher is more consistent)
    fn calculate_consistency_score(values: &[f32]) -> f32 {
        if values.len() < 2 {
            return 1.0;
        }

        let mean = values.iter().sum::<f32>() / values.len() as f32;
        let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f32>() / values.len() as f32;
        let std_dev = variance.sqrt();
        
        // Normalize standard deviation to a 0-1 score (lower std_dev = higher consistency)
        if mean > 0.0 {
            (1.0 - (std_dev / mean)).max(0.0)
        } else {
            1.0
        }
    }

    /// Calculate time regularity score based on how consistent feeding/drinking times are
    fn calculate_time_regularity_score(timestamps: &[DateTime<Utc>]) -> f32 {
        if timestamps.len() < 2 {
            return 1.0;
        }

        // Group by day and calculate average time of day for each day
        use std::collections::HashMap;
        let mut daily_times: HashMap<i32, Vec<u32>> = HashMap::new();
        
        for timestamp in timestamps {
            let day = timestamp.ordinal() as i32;
            let minutes_since_midnight = timestamp.hour() * 60 + timestamp.minute();
            daily_times.entry(day).or_insert_with(Vec::new).push(minutes_since_midnight);
        }

        // Calculate average time for each day
        let daily_averages: Vec<f32> = daily_times.values()
            .map(|times| times.iter().sum::<u32>() as f32 / times.len() as f32)
            .collect();

        // Calculate consistency of daily averages
        Self::calculate_consistency_score(&daily_averages)
    }

    /// Calculate variance in daily activity levels
    fn calculate_daily_activity_variance(events: &[ActivityEvent]) -> f32 {
        if events.is_empty() {
            return 1.0;
        }

        use std::collections::HashMap;
        let mut daily_counts: HashMap<i32, u32> = HashMap::new();
        
        for event in events {
            let day = event.timestamp.ordinal() as i32;
            *daily_counts.entry(day).or_insert(0) += 1;
        }

        let counts: Vec<f32> = daily_counts.values().map(|&count| count as f32).collect();
        1.0 - Self::calculate_consistency_score(&counts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Utc, Duration};
    use crate::api::client::{ActivityType, FeedingEvent, DrinkingEvent, ActivityEvent, Pet, Device, Position, Status, Activity, DeviceStatus, Tag, UsageStats, GENDER_FEMALE};

    fn create_test_pet_with_activity(id: u32, name: &str, last_activity_hours_ago: i64) -> Pet {
        let activity_time = Utc::now() - Duration::hours(last_activity_hours_ago);
        
        Pet {
            id,
            name: name.to_string(),
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
                    since: activity_time.to_rfc3339(),
                }),
                feeding: None,
                drinking: None,
            }),
            position: Some(Position {
                user_id: Some(1),
                tag_id: 123456,
                location: Some(1),
                since: activity_time.to_rfc3339(),
                version: Some(1),
                created_at: Some(activity_time.to_rfc3339()),
                updated_at: Some(activity_time.to_rfc3339()),
            }),
            tag: Some(Tag {
                id: 123456,
                index: Some(1),
                profile: Some(1),
            }),
        }
    }

    fn create_test_device_with_battery(id: u32, name: &str, battery_level: f32, online: bool) -> Device {
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
                online: Some(online),
                battery: Some(battery_level),
                learn_mode: None,
                signal_strength: Some(80.0),
                usage_stats: Some(UsageStats {
                    total_entries: 10,
                    total_exits: 8,
                    last_entry: None,
                    last_exit: None,
                    daily_average_entries: 2.5,
                }),
            }),
            control: None,
        }
    }

    #[test]
    fn test_identify_inactive_pets_no_alerts() {
        let pets = vec![
            create_test_pet_with_activity(1, "Active Pet", 2), // 2 hours ago
        ];
        let config = InactivityConfig::default();
        
        let alerts = DataProcessor::identify_inactive_pets(&pets, &config);
        assert_eq!(alerts.len(), 0);
    }

    #[test]
    fn test_identify_inactive_pets_medium_alert() {
        let pets = vec![
            create_test_pet_with_activity(1, "Inactive Pet", 15), // 15 hours ago
        ];
        let config = InactivityConfig::default();
        
        let alerts = DataProcessor::identify_inactive_pets(&pets, &config);
        assert_eq!(alerts.len(), 1);
        assert!(matches!(alerts[0].severity, AlertSeverity::Medium));
        assert_eq!(alerts[0].pet_id, Some(1));
        assert!(alerts[0].message.contains("Inactive Pet"));
        assert!(alerts[0].message.contains("15 hours"));
    }

    #[test]
    fn test_identify_inactive_pets_critical_alert() {
        let pets = vec![
            create_test_pet_with_activity(1, "Critical Pet", 30), // 30 hours ago
        ];
        let config = InactivityConfig::default();
        
        let alerts = DataProcessor::identify_inactive_pets(&pets, &config);
        assert_eq!(alerts.len(), 1);
        assert!(matches!(alerts[0].severity, AlertSeverity::Critical));
        assert_eq!(alerts[0].pet_id, Some(1));
        assert!(alerts[0].message.contains("Critical Pet"));
        assert!(alerts[0].message.contains("30 hours"));
    }

    #[test]
    fn test_generate_device_health_alerts_low_battery() {
        let devices = vec![
            create_test_device_with_battery(1, "Low Battery Device", 1.5, true), // 15% (1.5 * 10)
        ];
        let config = DeviceHealthConfig::default();
        
        let alerts = DataProcessor::generate_device_health_alerts(&devices, &config);
        assert_eq!(alerts.len(), 1);
        assert!(matches!(alerts[0].severity, AlertSeverity::Medium));
        assert_eq!(alerts[0].device_id, Some(1));
        assert!(alerts[0].message.contains("Low Battery Device"));
        assert!(alerts[0].message.contains("15.0%"));
    }

    #[test]
    fn test_generate_device_health_alerts_critical_battery() {
        let devices = vec![
            create_test_device_with_battery(1, "Critical Battery Device", 0.5, true), // 5% (0.5 * 10)
        ];
        let config = DeviceHealthConfig::default();
        
        let alerts = DataProcessor::generate_device_health_alerts(&devices, &config);
        assert_eq!(alerts.len(), 1);
        assert!(matches!(alerts[0].severity, AlertSeverity::Critical));
        assert_eq!(alerts[0].device_id, Some(1));
        assert!(alerts[0].message.contains("Critical Battery Device"));
        assert!(alerts[0].message.contains("5.0%"));
    }

    #[test]
    fn test_generate_device_health_alerts_offline() {
        let devices = vec![
            create_test_device_with_battery(1, "Offline Device", 5.0, false), // 50% (5.0 * 10)
        ];
        let config = DeviceHealthConfig::default();
        
        let alerts = DataProcessor::generate_device_health_alerts(&devices, &config);
        assert_eq!(alerts.len(), 1);
        assert!(matches!(alerts[0].severity, AlertSeverity::High));
        assert_eq!(alerts[0].device_id, Some(1));
        assert!(alerts[0].message.contains("Offline Device"));
        assert!(alerts[0].message.contains("offline"));
    }

    #[test]
    fn test_is_pet_inactive() {
        let active_pet = create_test_pet_with_activity(1, "Active Pet", 2);
        let inactive_pet = create_test_pet_with_activity(2, "Inactive Pet", 15);
        let config = InactivityConfig::default();
        
        assert!(!DataProcessor::is_pet_inactive(&active_pet, &config));
        assert!(DataProcessor::is_pet_inactive(&inactive_pet, &config));
    }

    #[test]
    fn test_calculate_feeding_trends_empty() {
        let history = FeedingHistory {
            pet_id: 1,
            events: vec![],
            summary: None,
        };

        let trends = DataProcessor::calculate_feeding_trends(&history);
        assert_eq!(trends.daily_average, 0.0);
        assert_eq!(trends.weekly_average, 0.0);
        assert!(matches!(trends.trend_direction, TrendDirection::Stable));
    }

    #[test]
    fn test_calculate_feeding_trends_with_data() {
        let now = Utc::now();
        let history = FeedingHistory {
            pet_id: 1,
            events: vec![
                FeedingEvent {
                    timestamp: now - Duration::days(2),
                    device_id: 1,
                    amount: 100.0,
                    duration: None,
                },
                FeedingEvent {
                    timestamp: now - Duration::days(1),
                    device_id: 1,
                    amount: 120.0,
                    duration: None,
                },
                FeedingEvent {
                    timestamp: now,
                    device_id: 1,
                    amount: 110.0,
                    duration: None,
                },
            ],
            summary: None,
        };

        let trends = DataProcessor::calculate_feeding_trends(&history);
        assert!(trends.daily_average > 0.0);
        assert!(trends.weekly_average > 0.0);
        assert!(trends.consistency_score >= 0.0 && trends.consistency_score <= 1.0);
    }

    #[test]
    fn test_calculate_health_metrics() {
        let now = Utc::now();
        
        let feeding_history = FeedingHistory {
            pet_id: 1,
            events: vec![
                FeedingEvent {
                    timestamp: now - Duration::hours(24),
                    device_id: 1,
                    amount: 100.0,
                    duration: None,
                },
            ],
            summary: None,
        };

        let drinking_history = DrinkingHistory {
            pet_id: 1,
            events: vec![
                DrinkingEvent {
                    timestamp: now - Duration::hours(12),
                    device_id: 1,
                    volume: 50.0,
                    duration: None,
                },
            ],
            summary: None,
        };

        let activity_history = ActivityHistory {
            pet_id: 1,
            events: vec![
                ActivityEvent {
                    timestamp: now - Duration::hours(6),
                    event_type: ActivityType::Entry,
                    location: 1,
                    device_id: Some(1),
                },
            ],
            summary: None,
        };

        let metrics = DataProcessor::calculate_health_metrics(&feeding_history, &drinking_history, &activity_history);
        assert!(metrics.overall_score >= 0.0 && metrics.overall_score <= 1.0);
        assert!(metrics.feeding_health.daily_intake_average > 0.0);
        assert!(metrics.drinking_health.daily_intake_average > 0.0);
    }
}