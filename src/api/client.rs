use crate::config;
use log::debug;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc, Timelike};

#[derive(Serialize, Deserialize, Debug)]
pub struct LoginResp {
    pub data: Data,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Data {
    pub user: User,
    pub token: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct User {
    pub id: u32,
    pub email_address: String,
    pub first_name: String,
    pub last_name: String,
    pub country_id: u32,
    pub language_id: u32,
    pub marketing_opt_in: bool,
    pub terms_accepted: String,
    pub weight_units: u32,
    pub time_format: u32,
    pub version: u32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Pet {
    pub id: u32,
    pub name: String,
    pub gender: Option<u32>,
    pub date_of_birth: Option<String>,
    pub weight: Option<String>,
    pub breed: Option<String>, // Added for requirement 3.3
    pub comments: Option<String>,
    pub household_id: u32,
    pub breed_id: u32,
    pub colour_id: Option<u32>,
    pub species_id: u32,
    pub tag_id: u64,
    pub version: u32,
    pub created_at: String,
    pub updated_at: String,
    pub photo: Option<Photo>,
    pub status: Option<Status>,
    pub position: Option<Position>,
    pub tag: Option<Tag>, // Added for microchip validation
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Photo {
    pub id: u32,
    pub location: String,
    pub full_location: Option<String>,
    pub version: u32,
    pub created_at: String,
    pub updated_at: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Status {
    pub activity: Option<Activity>,
    pub feeding: Option<Feeding>,
    pub drinking: Option<Drinking>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Activity {
    pub tag_id: u64,
    pub device_id: Option<u32>,
    #[serde(rename = "where")]
    pub location: u32,
    pub since: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Feeding {
    pub tag_id: u64,
    pub device_id: u32,
    pub at: String,
    pub change: Option<Vec<f32>>,
}


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Drinking {
    pub tag_id: u64,
    pub device_id: u32,
    pub at: String,
    pub change: Option<Vec<f32>>,
}



#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Position {
    pub user_id: Option<u32>,
    pub tag_id: u64,
    #[serde(rename = "where")]
    pub location: Option<u32>,
    pub since: String,
    pub version: Option<u32>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PetsResponse {
    pub data: Vec<Pet>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Device {
    pub id: u32,
    pub name: String,
    pub serial_number: String,
    pub mac_address: String,
    pub product_id: u32,
    pub household_id: u32,
    pub parent_device_id: Option<u32>,
    pub version: u32,
    pub created_at: String,
    pub updated_at: String,
    pub status: Option<DeviceStatus>,
    pub control: Option<DeviceControl>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DeviceStatus {
    pub locking: Option<LockingStatus>,
    pub version: Option<DeviceVersion>, // Changed from serde_json::Value for better validation
    pub online: Option<bool>,
    pub battery: Option<f32>,
    pub learn_mode: Option<bool>,
    pub signal_strength: Option<f32>, // Added for requirement 4.1
    pub usage_stats: Option<UsageStats>, // Added for requirement 4.5
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LockingStatus {
    pub mode: u32,
    pub curfew: Option<Vec<CurfewTime>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CurfewTime {
    pub enabled: bool,
    pub lock_time: String,
    pub unlock_time: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Tag {
    pub id: u64,
    pub index: Option<u32>,
    pub profile: Option<u32>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DeviceVersion {
    pub hardware: Option<String>,
    pub firmware: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UsageStats {
    pub total_entries: u32,
    pub total_exits: u32,
    pub last_entry: Option<String>,
    pub last_exit: Option<String>,
    pub daily_average_entries: f32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DeviceControl {
    pub locking: Option<u32>,
    pub curfew: Option<Vec<CurfewTime>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DevicesResponse {
    pub data: Vec<Device>,
}



// Lock states
pub const LOCK_STATE_UNLOCKED: u32 = 0;
pub const LOCK_STATE_KEEP_IN: u32 = 1;
pub const LOCK_STATE_KEEP_OUT: u32 = 2;
pub const LOCK_STATE_LOCKED: u32 = 3;

// Pet locations
pub const LOCATION_INSIDE: u32 = 1;
pub const LOCATION_OUTSIDE: u32 = 2;

// Pet genders
pub const GENDER_FEMALE: u32 = 0;
pub const GENDER_MALE: u32 = 1;

// Batch operation structures
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BatchResult {
    pub successful: Vec<u32>,
    pub failed: Vec<BatchError>,
    pub total_processed: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BatchError {
    pub id: u32,
    pub error: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PetLocationUpdate {
    pub pet_id: u32,
    pub location: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DeviceCommand {
    pub device_id: u32,
    pub command_type: DeviceCommandType,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum DeviceCommandType {
    SetLockState { lock_state: u32 },
    SetCurfew { curfew_times: Vec<CurfewTime> },
}

// Historical data structures
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FeedingHistory {
    pub pet_id: u32,
    pub events: Vec<FeedingEvent>,
    pub summary: Option<FeedingSummary>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FeedingEvent {
    pub timestamp: DateTime<Utc>,
    pub device_id: u32,
    pub amount: f32,
    pub duration: Option<u32>, // Duration in seconds
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FeedingSummary {
    pub total_amount: f32,
    pub event_count: u32,
    pub daily_average: f32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DrinkingHistory {
    pub pet_id: u32,
    pub events: Vec<DrinkingEvent>,
    pub summary: Option<DrinkingSummary>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DrinkingEvent {
    pub timestamp: DateTime<Utc>,
    pub device_id: u32,
    pub volume: f32,
    pub duration: Option<u32>, // Duration in seconds
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DrinkingSummary {
    pub total_volume: f32,
    pub event_count: u32,
    pub daily_average: f32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ActivityHistory {
    pub pet_id: u32,
    pub events: Vec<ActivityEvent>,
    pub summary: Option<ActivitySummary>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ActivityEvent {
    pub timestamp: DateTime<Utc>,
    pub event_type: ActivityType,
    pub location: u32,
    pub device_id: Option<u32>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ActivityType {
    Entry,
    Exit,
    FeedingStart,
    FeedingEnd,
    DrinkingStart,
    DrinkingEnd,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ActivitySummary {
    pub total_events: u32,
    pub entries: u32,
    pub exits: u32,
    pub feeding_sessions: u32,
    pub drinking_sessions: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateRange {
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
}

/// SurePet API client that maintains a single HTTP client instance for the session.
/// This enables connection reuse, connection pooling, and efficient resource management.
#[derive(Clone)]
pub struct Client {
    /// Reusable HTTP client for all API requests during the session
    pub client: reqwest::Client,
    /// Application configuration containing API endpoints and settings
    pub cfg: config::Config,
}

impl Client {
    /// Creates a new API client with a single reqwest::Client instance.
    /// This client should be reused for all API calls during the session
    /// to enable connection pooling and efficient resource usage.
    pub fn new(cfg: config::Config) -> Self {
        Client {
            client: reqwest::Client::new(),
            cfg,
        }
    }

    /// Login to the SurePet API and get an authentication token
    pub async fn login(
        &self,
        username: &String,
        password: &String,
    ) -> Result<LoginResp, reqwest::Error> {
        let uuid: String = "a1b96664-399d-4c2f-8eaa-b6b5e47c6f31".to_string();
        let post_url: String = self.cfg.api.surehub_url.to_owned() + "/auth/login";

        debug!("Posting to: {}", post_url);

        let mut map = HashMap::new();
        map.insert("email_address", username);
        map.insert("password", password);
        map.insert("device_id", &uuid);

        debug!("Body to post: {:?}", map);

        let resp = self
            .client
            .post(post_url)
            .header("Host", "app.api.surehub.io")
            .header("Accept-Encoding", "gzip, deflate, br")
            .header("Content-Type", "application/json")
            .header("Accept", "*/*")
            .header("User-Agent", "RustyPet")
            .header("Connection", "keep-alive")
            .header("X-Device-Id", &uuid)
            .json(&map)
            .send()
            .await?;

        debug!("Response Status: {:?}", resp.status());

        if resp.status() == StatusCode::OK {
            let text = resp.text().await?;
            debug!("Response Text: {}", &text);
            let login_resp: LoginResp = serde_json::from_str(&text)
                .expect("Failed to parse login response JSON");

            return Ok(login_resp);
        }

        return Err(resp.error_for_status().err()
            .expect("Failed to get error status from response"));
    }

    /// Get all pets associated with the account
    pub async fn get_pets(&self, token: &str) -> Result<PetsResponse, reqwest::Error> {
        // Get the start data which includes all pets
        let start_url = format!("{}/me/start", self.cfg.api.surehub_url);
        debug!("Getting start data from: {}", start_url);

        let start_resp = self
            .client
            .get(&start_url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .header("User-Agent", "RustyPet")
            .send()
            .await?;

        debug!("Start response status: {:?}", start_resp.status());

        if !start_resp.status().is_success() {
            return Err(start_resp.error_for_status().err()
                .expect("Failed to get error status from start response"));
        }

        let start_text = start_resp.text().await?;
        debug!("Start response: {}", &start_text);

        // Parse the start response to get pets directly
        let start_data: serde_json::Value = serde_json::from_str(&start_text)
            .expect("Failed to parse start response JSON");
        
        // Extract pets from the start response
        let pets_json = &start_data["data"]["pets"];
        debug!("Pets JSON: {}", serde_json::to_string_pretty(pets_json)
            .unwrap_or_else(|_| "Failed to serialize pets JSON".to_string()));
        
        let pets_resp: PetsResponse = serde_json::from_value(serde_json::json!({
            "data": pets_json
        })).expect("Failed to parse pets response JSON");

        Ok(pets_resp)
    }

    /// Set a pet's location (inside or outside)
    pub async fn set_pet_location(&self, token: &str, pet_id: u32, location: u32) -> Result<(), reqwest::Error> {
        let url = format!("{}/pet/{}/position", self.cfg.api.surehub_url, pet_id);
        debug!("Setting pet {} location to {} at: {}", pet_id, location, url);

        let request_body = serde_json::json!({
            "where": location,
            "since": chrono::Utc::now().to_rfc3339()
        });

        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .header("User-Agent", "RustyPet")
            .json(&request_body)
            .send()
            .await?;

        debug!("Set pet location response status: {:?}", resp.status());

        if resp.status().is_success() {
            return Ok(());
        }

        Err(resp.error_for_status().err()
            .expect("Failed to get error status from response"))
    }

    /// Get all devices (flaps, feeders, etc.) associated with the account
    pub async fn get_devices(&self, token: &str) -> Result<DevicesResponse, reqwest::Error> {
        // Get the start data which includes all devices
        let start_url = format!("{}/me/start", self.cfg.api.surehub_url);
        debug!("Getting start data from: {}", start_url);

        let start_resp = self
            .client
            .get(&start_url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .header("User-Agent", "RustyPet")
            .send()
            .await?;

        debug!("Start response status: {:?}", start_resp.status());

        if !start_resp.status().is_success() {
            return Err(start_resp.error_for_status().err()
                .expect("Failed to get error status from start response"));
        }

        let start_text = start_resp.text().await?;
        debug!("Start response: {}", &start_text);

        // Parse the start response to get devices directly
        let start_data: serde_json::Value = serde_json::from_str(&start_text)
            .expect("Failed to parse start response JSON");
        
        // Extract devices from the start response
        let devices_json = &start_data["data"]["devices"];
        let devices_resp: DevicesResponse = serde_json::from_value(serde_json::json!({
            "data": devices_json
        })).expect("Failed to parse devices response JSON");

        Ok(devices_resp)
    }

    /// Set the lock state of a device (flap)
    pub async fn set_lock_state(&self, token: &str, device_id: u32, lock_state: u32) -> Result<(), reqwest::Error> {
        let url = format!("{}/device/{}/control", self.cfg.api.surehub_url, device_id);
        debug!("Setting device {} lock state to {} at: {}", device_id, lock_state, url);

        let request_body = serde_json::json!({
            "locking": lock_state
        });

        let resp = self
            .client
            .put(&url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .header("User-Agent", "RustyPet")
            .json(&request_body)
            .send()
            .await?;

        debug!("Set lock state response status: {:?}", resp.status());

        if resp.status().is_success() {
            return Ok(());
        }

        Err(resp.error_for_status().err()
            .expect("Failed to get error status from response"))
    }

    /// Lock the flap (prevent all access)
    pub async fn lock(&self, token: &str, device_id: u32) -> Result<(), reqwest::Error> {
        self.set_lock_state(token, device_id, LOCK_STATE_LOCKED).await
    }

    /// Lock pets in (allow exit but prevent entry)
    pub async fn lock_in(&self, token: &str, device_id: u32) -> Result<(), reqwest::Error> {
        self.set_lock_state(token, device_id, LOCK_STATE_KEEP_IN).await
    }

    /// Lock pets out (allow entry but prevent exit)
    pub async fn lock_out(&self, token: &str, device_id: u32) -> Result<(), reqwest::Error> {
        self.set_lock_state(token, device_id, LOCK_STATE_KEEP_OUT).await
    }

    /// Unlock the flap (allow free access)
    pub async fn unlock(&self, token: &str, device_id: u32) -> Result<(), reqwest::Error> {
        self.set_lock_state(token, device_id, LOCK_STATE_UNLOCKED).await
    }

    /// Set curfew times for a device
    pub async fn set_curfew(&self, token: &str, device_id: u32, curfew_times: Vec<CurfewTime>) -> Result<(), reqwest::Error> {
        let url = format!("{}/device/{}/control", self.cfg.api.surehub_url, device_id);
        debug!("Setting device {} curfew at: {}", device_id, url);

        let request_body = serde_json::json!({
            "curfew": curfew_times
        });

        let resp = self
            .client
            .put(&url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .header("User-Agent", "RustyPet")
            .json(&request_body)
            .send()
            .await?;

        debug!("Set curfew response status: {:?}", resp.status());

        if resp.status().is_success() {
            return Ok(());
        }

        Err(resp.error_for_status().err()
            .expect("Failed to get error status from response"))
    }

    /// Get feeding history for a pet within a date range using the dashboard pet endpoint
    pub async fn get_feeding_history(&self, token: &str, pet_id: u32, date_range: DateRange) -> Result<FeedingHistory, reqwest::Error> {
        // Calculate days history from date range
        let days_history = (date_range.to - date_range.from).num_days().max(1);
        
        // Use the dashboard pet endpoint with the correct format
        let url = "https://app-api.production.surehub.io/api/dashboard/pet";
        debug!("Getting feeding history for pet {} from: {} (days: {})", pet_id, url, days_history);

        let resp = self
            .client
            .get(url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .header("User-Agent", "RustyPet")
            .query(&[
                ("Pet_Id", pet_id.to_string()),
                ("From", date_range.from.to_rfc3339()),
                ("dayshistory", days_history.to_string()),
            ])
            .send()
            .await?;

        debug!("Dashboard pet response status: {:?}", resp.status());

        if !resp.status().is_success() {
            return Err(resp.error_for_status().err()
                .expect("Failed to get error status from response"));
        }

        let response_text = resp.text().await?;
        debug!("Dashboard pet response length: {} chars", response_text.len());

        // Parse the API response and extract feeding events
        let _api_response: serde_json::Value = serde_json::from_str(&response_text)
            .expect("Failed to parse dashboard pet response JSON");
        
        // Parse the API response and extract feeding events
        let api_response: serde_json::Value = serde_json::from_str(&response_text)
            .expect("Failed to parse dashboard pet response JSON");
        
        let mut all_events = Vec::new();
        
        // Parse the dashboard endpoint response structure
        if let Some(data_array) = api_response.get("data").and_then(|d| d.as_array()) {
            for pet_data in data_array {
                // Check if this is data for our pet
                if let Some(returned_pet_id) = pet_data.get("pet_id").and_then(|id| id.as_u64()) {
                    if returned_pet_id as u32 != pet_id {
                        continue; // Skip data for other pets
                    }
                }
                
                // Extract feeding data
                if let Some(feeding) = pet_data.get("feeding") {
                    // Get device IDs for this pet's feeding data
                    let device_ids: Vec<u32> = feeding.get("device_ids")
                        .and_then(|ids| ids.as_array())
                        .map(|arr| arr.iter().filter_map(|id| id.as_u64().map(|id| id as u32)).collect())
                        .unwrap_or_default();
                    
                    let primary_device_id = device_ids.first().copied().unwrap_or(0);
                    
                    // Process daily feeding activity data
                    if let Some(activity_array) = feeding.get("activity").and_then(|a| a.as_array()) {
                        for daily_activity in activity_array {
                            if let (Some(date_str), Some(total_consumption)) = (
                                daily_activity.get("date").and_then(|d| d.as_str()),
                                daily_activity.get("total_consumption").and_then(|c| c.as_f64())
                            ) {
                                if let Ok(date) = date_str.parse::<DateTime<Utc>>() {
                                    // Only create feeding events if there was actual consumption
                                    if total_consumption > 0.0 {
                                        // Create a feeding event for this day
                                        // We'll place it at a reasonable feeding time (e.g., 12:00 PM)
                                        let feeding_time = date.with_hour(12).unwrap_or(date);
                                        
                                        all_events.push(FeedingEvent {
                                            timestamp: feeding_time,
                                            device_id: primary_device_id,
                                            amount: total_consumption as f32,
                                            duration: None, // Duration not provided in daily summary
                                        });
                                    }
                                }
                            }
                        }
                    }
                    
                    // Also extract summary data for additional context
                    if let Some(total_consumption) = feeding.get("total_consumption").and_then(|c| c.as_f64()) {
                        if let Some(number_of_visits) = feeding.get("number_of_visits").and_then(|v| v.as_u64()) {
                            debug!("Pet {} had {} total consumption and {} feeding visits", pet_id, total_consumption, number_of_visits);
                        }
                    }
                }
            }
        }

        debug!("Total feeding events found: {}", all_events.len());

        // Sort events by timestamp (most recent first)
        all_events.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        // Calculate summary
        let total_amount: f32 = all_events.iter().map(|e| e.amount).sum();
        let event_count = all_events.len() as u32;
        let days = (date_range.to - date_range.from).num_days().max(1) as f32;
        let daily_average = total_amount / days;

        let summary = if event_count > 0 {
            Some(FeedingSummary {
                total_amount,
                event_count,
                daily_average,
            })
        } else {
            None
        };

        Ok(FeedingHistory {
            pet_id,
            events: all_events,
            summary,
        })
    }

    /// Get drinking history for a pet within a date range using the dashboard pet endpoint
    pub async fn get_drinking_history(&self, token: &str, pet_id: u32, date_range: DateRange) -> Result<DrinkingHistory, reqwest::Error> {
        // Calculate days history from date range
        let days_history = (date_range.to - date_range.from).num_days().max(1);
        
        // Use the dashboard pet endpoint with the correct format
        let url = "https://app-api.production.surehub.io/api/dashboard/pet";
        debug!("Getting drinking history for pet {} from: {} (days: {})", pet_id, url, days_history);

        let resp = self
            .client
            .get(url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .header("User-Agent", "RustyPet")
            .query(&[
                ("Pet_Id", pet_id.to_string()),
                ("From", date_range.from.to_rfc3339()),
                ("dayshistory", days_history.to_string()),
            ])
            .send()
            .await?;

        debug!("Dashboard pet response status: {:?}", resp.status());

        if !resp.status().is_success() {
            return Err(resp.error_for_status().err()
                .expect("Failed to get error status from response"));
        }

        let response_text = resp.text().await?;
        debug!("Dashboard pet response length: {} chars", response_text.len());

        // Parse the API response and extract drinking events
        let _api_response: serde_json::Value = serde_json::from_str(&response_text)
            .expect("Failed to parse dashboard pet response JSON");
        
        // Parse the API response and extract drinking events
        let api_response: serde_json::Value = serde_json::from_str(&response_text)
            .expect("Failed to parse dashboard pet response JSON");
        
        let mut all_events = Vec::new();
        
        // Parse the dashboard endpoint response structure
        if let Some(data_array) = api_response.get("data").and_then(|d| d.as_array()) {
            for pet_data in data_array {
                // Check if this is data for our pet
                if let Some(returned_pet_id) = pet_data.get("pet_id").and_then(|id| id.as_u64()) {
                    if returned_pet_id as u32 != pet_id {
                        continue; // Skip data for other pets
                    }
                }
                
                // Extract drinking data
                if let Some(drinking) = pet_data.get("drinking") {
                    // Get device IDs for this pet's drinking data
                    let device_ids: Vec<u32> = drinking.get("device_ids")
                        .and_then(|ids| ids.as_array())
                        .map(|arr| arr.iter().filter_map(|id| id.as_u64().map(|id| id as u32)).collect())
                        .unwrap_or_default();
                    
                    let primary_device_id = device_ids.first().copied().unwrap_or(0);
                    
                    // Process daily drinking activity data
                    if let Some(activity_array) = drinking.get("activity").and_then(|a| a.as_array()) {
                        for daily_activity in activity_array {
                            if let (Some(date_str), Some(total_consumption)) = (
                                daily_activity.get("date").and_then(|d| d.as_str()),
                                daily_activity.get("total_consumption").and_then(|c| c.as_f64())
                            ) {
                                if let Ok(date) = date_str.parse::<DateTime<Utc>>() {
                                    // Only create drinking events if there was actual consumption
                                    if total_consumption > 0.0 {
                                        // Create a drinking event for this day
                                        // We'll place it at a reasonable drinking time (e.g., 10:00 AM)
                                        let drinking_time = date.with_hour(10).unwrap_or(date);
                                        
                                        all_events.push(DrinkingEvent {
                                            timestamp: drinking_time,
                                            device_id: primary_device_id,
                                            volume: total_consumption as f32,
                                            duration: None, // Duration not provided in daily summary
                                        });
                                    }
                                }
                            }
                        }
                    }
                    
                    // Also extract summary data for additional context
                    if let Some(total_consumption) = drinking.get("total_consumption").and_then(|c| c.as_f64()) {
                        if let Some(number_of_visits) = drinking.get("number_of_visits").and_then(|v| v.as_u64()) {
                            debug!("Pet {} had {} total consumption and {} drinking visits", pet_id, total_consumption, number_of_visits);
                        }
                    }
                }
            }
        }

        debug!("Total drinking events found: {}", all_events.len());

        // Sort events by timestamp (most recent first)
        all_events.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        // Calculate summary
        let total_volume: f32 = all_events.iter().map(|e| e.volume).sum();
        let event_count = all_events.len() as u32;
        let days = (date_range.to - date_range.from).num_days().max(1) as f32;
        let daily_average = total_volume / days;

        let summary = if event_count > 0 {
            Some(DrinkingSummary {
                total_volume,
                event_count,
                daily_average,
            })
        } else {
            None
        };

        Ok(DrinkingHistory {
            pet_id,
            events: all_events,
            summary,
        })
    }

    /// Get activity history for a pet within a date range using the dashboard pet endpoint
    pub async fn get_activity_history(&self, token: &str, pet_id: u32, date_range: DateRange) -> Result<ActivityHistory, reqwest::Error> {
        // Calculate days history from date range
        let days_history = (date_range.to - date_range.from).num_days().max(1);
        
        // Use the dashboard pet endpoint with the correct format
        let url = "https://app-api.production.surehub.io/api/dashboard/pet";
        debug!("Getting activity history for pet {} from: {} (days: {})", pet_id, url, days_history);

        let resp = self
            .client
            .get(url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .header("User-Agent", "RustyPet")
            .query(&[
                ("Pet_Id", pet_id.to_string()),
                ("From", date_range.from.to_rfc3339()),
                ("dayshistory", days_history.to_string()),
            ])
            .send()
            .await?;

        debug!("Dashboard pet response status: {:?}", resp.status());

        if !resp.status().is_success() {
            return Err(resp.error_for_status().err()
                .expect("Failed to get error status from response"));
        }

        let response_text = resp.text().await?;
        debug!("Dashboard pet response length: {} chars", response_text.len());

        // Parse the API response and extract activity events
        let api_response: serde_json::Value = serde_json::from_str(&response_text)
            .expect("Failed to parse dashboard pet response JSON");
        
        debug!("Dashboard pet response: {}", serde_json::to_string_pretty(&api_response)
            .unwrap_or_else(|_| "Failed to serialize response".to_string()));

        let mut all_events = Vec::new();
        
        // Parse the dashboard endpoint response structure
        if let Some(data_array) = api_response.get("data").and_then(|d| d.as_array()) {
            for pet_data in data_array {
                // Check if this is data for our pet
                if let Some(returned_pet_id) = pet_data.get("pet_id").and_then(|id| id.as_u64()) {
                    if returned_pet_id as u32 != pet_id {
                        continue; // Skip data for other pets
                    }
                }
                
                // Extract movement data
                if let Some(movement) = pet_data.get("movement") {
                    // Get device IDs for this pet's movement data
                    let device_ids: Vec<u32> = movement.get("device_ids")
                        .and_then(|ids| ids.as_array())
                        .map(|arr| arr.iter().filter_map(|id| id.as_u64().map(|id| id as u32)).collect())
                        .unwrap_or_default();
                    
                    let primary_device_id = device_ids.first().copied();
                    
                    // Process daily activity data
                    if let Some(activity_array) = movement.get("activity").and_then(|a| a.as_array()) {
                        for daily_activity in activity_array {
                            if let Some(date_str) = daily_activity.get("date").and_then(|d| d.as_str()) {
                                if let Ok(date) = date_str.parse::<DateTime<Utc>>() {
                                    // Get time outside for this day
                                    if let Some(time_outside_str) = daily_activity.get("time_outside").and_then(|t| t.as_str()) {
                                        // Parse time_outside format "HH:MM:SS"
                                        if time_outside_str != "00:00:00" {
                                            // Create synthetic activity events based on time outside
                                            // We'll create entry/exit pairs to represent the time spent outside
                                            
                                            // Parse the time outside duration
                                            let time_parts: Vec<&str> = time_outside_str.split(':').collect();
                                            if time_parts.len() == 3 {
                                                if let (Ok(hours), Ok(minutes), Ok(seconds)) = (
                                                    time_parts[0].parse::<u32>(),
                                                    time_parts[1].parse::<u32>(),
                                                    time_parts[2].parse::<u32>()
                                                ) {
                                                    let total_seconds = hours * 3600 + minutes * 60 + seconds;
                                                    
                                                    if total_seconds > 0 {
                                                        // Create an exit event (going outside) at the start of the day
                                                        let exit_time = date.with_hour(8).unwrap_or(date); // Assume 8 AM start
                                                        all_events.push(ActivityEvent {
                                                            timestamp: exit_time,
                                                            event_type: ActivityType::Exit,
                                                            location: 2, // Outside
                                                            device_id: primary_device_id,
                                                        });
                                                        
                                                        // Create an entry event (coming back inside) after the time outside
                                                        let entry_time = exit_time + chrono::Duration::seconds(total_seconds as i64);
                                                        all_events.push(ActivityEvent {
                                                            timestamp: entry_time,
                                                            event_type: ActivityType::Entry,
                                                            location: 1, // Inside
                                                            device_id: primary_device_id,
                                                        });
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    
                    // Also extract summary data for additional context
                    if let Some(entries) = movement.get("entries").and_then(|e| e.as_u64()) {
                        if let Some(trips_outside) = movement.get("trips_outside").and_then(|t| t.as_u64()) {
                            debug!("Pet {} had {} entries and {} trips outside", pet_id, entries, trips_outside);
                        }
                    }
                }
            }
        }

        debug!("Total events found: {}", all_events.len());

        // Sort events by timestamp (most recent first)
        all_events.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        // Calculate summary
        let total_events = all_events.len() as u32;
        let entries = all_events.iter().filter(|e| matches!(e.event_type, ActivityType::Entry)).count() as u32;
        let exits = all_events.iter().filter(|e| matches!(e.event_type, ActivityType::Exit)).count() as u32;
        let feeding_sessions = all_events.iter().filter(|e| matches!(e.event_type, ActivityType::FeedingStart)).count() as u32;
        let drinking_sessions = all_events.iter().filter(|e| matches!(e.event_type, ActivityType::DrinkingStart)).count() as u32;

        let summary = if total_events > 0 {
            Some(ActivitySummary {
                total_events,
                entries,
                exits,
                feeding_sessions,
                drinking_sessions,
            })
        } else {
            None
        };

        Ok(ActivityHistory {
            pet_id,
            events: all_events,
            summary,
        })
    }

    /// Set pet to indoor mode (location inside)
    pub async fn set_pet_indoor_mode(&self, token: &str, pet_id: u32) -> Result<(), reqwest::Error> {
        self.set_pet_location(token, pet_id, LOCATION_INSIDE).await
    }

    /// Set pet to outdoor mode (location outside)
    pub async fn set_pet_outdoor_mode(&self, token: &str, pet_id: u32) -> Result<(), reqwest::Error> {
        self.set_pet_location(token, pet_id, LOCATION_OUTSIDE).await
    }

    /// Batch set pet locations for multiple pets
    pub async fn batch_set_pet_locations(&self, token: &str, updates: Vec<PetLocationUpdate>) -> Result<BatchResult, reqwest::Error> {
        let mut successful = Vec::new();
        let mut failed = Vec::new();
        let total_processed = updates.len();

        for update in updates {
            match self.set_pet_location(token, update.pet_id, update.location).await {
                Ok(_) => {
                    successful.push(update.pet_id);
                }
                Err(e) => {
                    failed.push(BatchError {
                        id: update.pet_id,
                        error: format!("Failed to set location: {}", e),
                    });
                }
            }
        }

        Ok(BatchResult {
            successful,
            failed,
            total_processed,
        })
    }

    /// Batch device control operations for multiple devices
    pub async fn batch_device_control(&self, token: &str, commands: Vec<DeviceCommand>) -> Result<BatchResult, reqwest::Error> {
        let mut successful = Vec::new();
        let mut failed = Vec::new();
        let total_processed = commands.len();

        for command in commands {
            let result = match command.command_type {
                DeviceCommandType::SetLockState { lock_state } => {
                    self.set_lock_state(token, command.device_id, lock_state).await
                }
                DeviceCommandType::SetCurfew { curfew_times } => {
                    self.set_curfew(token, command.device_id, curfew_times).await
                }
            };

            match result {
                Ok(_) => {
                    successful.push(command.device_id);
                }
                Err(e) => {
                    failed.push(BatchError {
                        id: command.device_id,
                        error: format!("Failed to execute command: {}", e),
                    });
                }
            }
        }

        Ok(BatchResult {
            successful,
            failed,
            total_processed,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_result_creation() {
        let successful = vec![1, 2, 3];
        let failed = vec![
            BatchError {
                id: 4,
                error: "Network error".to_string(),
            }
        ];
        let total_processed = 4;

        let result = BatchResult {
            successful: successful.clone(),
            failed: failed.clone(),
            total_processed,
        };

        assert_eq!(result.successful, successful);
        assert_eq!(result.failed.len(), 1);
        assert_eq!(result.failed[0].id, 4);
        assert_eq!(result.total_processed, 4);
    }

    #[test]
    fn test_pet_location_update_creation() {
        let update = PetLocationUpdate {
            pet_id: 123,
            location: LOCATION_INSIDE,
        };

        assert_eq!(update.pet_id, 123);
        assert_eq!(update.location, LOCATION_INSIDE);
    }

    #[test]
    fn test_device_command_creation() {
        let command = DeviceCommand {
            device_id: 456,
            command_type: DeviceCommandType::SetLockState { 
                lock_state: LOCK_STATE_LOCKED 
            },
        };

        assert_eq!(command.device_id, 456);
        match command.command_type {
            DeviceCommandType::SetLockState { lock_state } => {
                assert_eq!(lock_state, LOCK_STATE_LOCKED);
            }
            _ => panic!("Expected SetLockState command type"),
        }
    }

    #[test]
    fn test_device_command_curfew_creation() {
        let curfew_times = vec![CurfewTime {
            enabled: true,
            lock_time: "22:00".to_string(),
            unlock_time: "06:00".to_string(),
        }];

        let command = DeviceCommand {
            device_id: 789,
            command_type: DeviceCommandType::SetCurfew { 
                curfew_times: curfew_times.clone() 
            },
        };

        assert_eq!(command.device_id, 789);
        match command.command_type {
            DeviceCommandType::SetCurfew { curfew_times: times } => {
                assert_eq!(times.len(), 1);
                assert_eq!(times[0].enabled, true);
                assert_eq!(times[0].lock_time, "22:00");
                assert_eq!(times[0].unlock_time, "06:00");
            }
            _ => panic!("Expected SetCurfew command type"),
        }
    }
}
