#![allow(dead_code)] // Module contains future functionality not yet integrated

use crate::api::client::{Pet, Device, FeedingHistory, DrinkingHistory, ActivityHistory};
use chrono::{DateTime, Utc, Duration};
use log::{debug, warn, error};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tokio::fs as async_fs;

/// Cache error types
#[derive(Debug, thiserror::Error)]
pub enum CacheError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("Cache directory creation failed: {message}")]
    DirectoryCreation { message: String },
    
    #[error("Cache file corruption: {message}")]
    Corruption { message: String },
}

/// Cached data wrapper with metadata
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CachedData<T> {
    pub data: T,
    pub cached_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

impl<T> CachedData<T> {
    pub fn new(data: T, ttl: Duration) -> Self {
        let now = Utc::now();
        Self {
            data,
            cached_at: now,
            expires_at: now + ttl,
        }
    }
    
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }
    
    pub fn age(&self) -> Duration {
        Utc::now() - self.cached_at
    }
}

/// Cache manager for local data storage with TTL-based expiration
pub struct CacheManager {
    cache_dir: PathBuf,
    ttl: Duration,
}

impl CacheManager {
    /// Create a new cache manager with specified cache directory and TTL
    pub fn new(cache_dir: PathBuf, ttl_hours: u64) -> Result<Self, CacheError> {
        let ttl = Duration::hours(ttl_hours as i64);
        
        // Create cache directory if it doesn't exist
        if !cache_dir.exists() {
            fs::create_dir_all(&cache_dir).map_err(|e| CacheError::DirectoryCreation {
                message: format!("Failed to create cache directory {}: {}", cache_dir.display(), e),
            })?;
            debug!("Created cache directory: {}", cache_dir.display());
        }
        
        Ok(Self { cache_dir, ttl })
    }
    
    /// Create a default cache manager using system cache directory
    pub fn default() -> Result<Self, CacheError> {
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")))
            .join("rusty_pet");
            
        Self::new(cache_dir, 24) // Default 24 hour TTL
    }
    
    /// Get cached pets data if available and not expired
    pub async fn get_pets(&self) -> Option<CachedData<Vec<Pet>>> {
        self.get_cached_data("pets.json").await
    }
    
    /// Cache pets data with TTL
    pub async fn cache_pets(&self, pets: Vec<Pet>) -> Result<(), CacheError> {
        let cached_data = CachedData::new(pets, self.ttl);
        self.store_cached_data("pets.json", &cached_data).await
    }
    
    /// Get cached devices data if available and not expired
    pub async fn get_devices(&self) -> Option<CachedData<Vec<Device>>> {
        self.get_cached_data("devices.json").await
    }
    
    /// Cache devices data with TTL
    pub async fn cache_devices(&self, devices: Vec<Device>) -> Result<(), CacheError> {
        let cached_data = CachedData::new(devices, self.ttl);
        self.store_cached_data("devices.json", &cached_data).await
    }
    
    /// Get cached feeding history for a specific pet and date range
    pub async fn get_feeding_history(&self, pet_id: u32, date_range: &crate::api::client::DateRange) -> Option<CachedData<FeedingHistory>> {
        let from_str = date_range.from.format("%Y%m%d").to_string();
        let to_str = date_range.to.format("%Y%m%d").to_string();
        let filename = format!("feeding_history_{}_{}_to_{}.json", pet_id, from_str, to_str);
        self.get_cached_data(&filename).await
    }
    
    /// Cache feeding history for a specific pet with date range in filename
    pub async fn cache_feeding_history(&self, history: FeedingHistory, date_range: &crate::api::client::DateRange) -> Result<(), CacheError> {
        let from_str = date_range.from.format("%Y%m%d").to_string();
        let to_str = date_range.to.format("%Y%m%d").to_string();
        let filename = format!("feeding_history_{}_{}_to_{}.json", history.pet_id, from_str, to_str);
        let cached_data = CachedData::new(history, self.ttl);
        self.store_cached_data(&filename, &cached_data).await
    }
    
    /// Get cached drinking history for a specific pet and date range
    pub async fn get_drinking_history(&self, pet_id: u32, date_range: &crate::api::client::DateRange) -> Option<CachedData<DrinkingHistory>> {
        let from_str = date_range.from.format("%Y%m%d").to_string();
        let to_str = date_range.to.format("%Y%m%d").to_string();
        let filename = format!("drinking_history_{}_{}_to_{}.json", pet_id, from_str, to_str);
        self.get_cached_data(&filename).await
    }
    
    /// Cache drinking history for a specific pet with date range in filename
    pub async fn cache_drinking_history(&self, history: DrinkingHistory, date_range: &crate::api::client::DateRange) -> Result<(), CacheError> {
        let from_str = date_range.from.format("%Y%m%d").to_string();
        let to_str = date_range.to.format("%Y%m%d").to_string();
        let filename = format!("drinking_history_{}_{}_to_{}.json", history.pet_id, from_str, to_str);
        let cached_data = CachedData::new(history, self.ttl);
        self.store_cached_data(&filename, &cached_data).await
    }
    
    /// Get cached activity history for a specific pet and date range
    pub async fn get_activity_history(&self, pet_id: u32, date_range: &crate::api::client::DateRange) -> Option<CachedData<ActivityHistory>> {
        let from_str = date_range.from.format("%Y%m%d").to_string();
        let to_str = date_range.to.format("%Y%m%d").to_string();
        let filename = format!("activity_history_{}_{}_to_{}.json", pet_id, from_str, to_str);
        self.get_cached_data(&filename).await
    }
    
    /// Cache activity history for a specific pet with date range in filename
    pub async fn cache_activity_history(&self, history: ActivityHistory, date_range: &crate::api::client::DateRange) -> Result<(), CacheError> {
        let from_str = date_range.from.format("%Y%m%d").to_string();
        let to_str = date_range.to.format("%Y%m%d").to_string();
        let filename = format!("activity_history_{}_{}_to_{}.json", history.pet_id, from_str, to_str);
        let cached_data = CachedData::new(history, self.ttl);
        self.store_cached_data(&filename, &cached_data).await
    }
    
    /// Check if a cache entry is expired by key
    pub async fn is_expired(&self, key: &str) -> bool {
        let file_path = self.cache_dir.join(key);
        
        if !file_path.exists() {
            return true; // Non-existent files are considered expired
        }
        
        // Try to read the cached data directly as CachedData<serde_json::Value>
        match self.read_cache_file::<serde_json::Value>(&file_path).await {
            Ok(cached_data) => {
                cached_data.is_expired()
            }
            Err(_) => true, // If we can't read the file, consider it expired
        }
    }
    
    /// Clear all expired cache entries
    pub async fn clear_expired(&self) -> Result<(), CacheError> {
        let mut cleared_count = 0;
        
        if !self.cache_dir.exists() {
            debug!("Cache directory doesn't exist, nothing to clear");
            return Ok(());
        }
        
        let mut entries = async_fs::read_dir(&self.cache_dir).await?;
        
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            
            if path.is_file() && path.extension().map_or(false, |ext| ext == "json") {
                if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                    if self.is_expired(filename).await {
                        match async_fs::remove_file(&path).await {
                            Ok(_) => {
                                debug!("Removed expired cache file: {}", path.display());
                                cleared_count += 1;
                            }
                            Err(e) => {
                                warn!("Failed to remove expired cache file {}: {}", path.display(), e);
                            }
                        }
                    }
                }
            }
        }
        
        debug!("Cleared {} expired cache entries", cleared_count);
        Ok(())
    }
    
    /// Clear all cache entries
    pub async fn clear_all(&self) -> Result<(), CacheError> {
        if !self.cache_dir.exists() {
            debug!("Cache directory doesn't exist, nothing to clear");
            return Ok(());
        }
        
        let mut entries = async_fs::read_dir(&self.cache_dir).await?;
        let mut cleared_count = 0;
        
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            
            if path.is_file() && path.extension().map_or(false, |ext| ext == "json") {
                match async_fs::remove_file(&path).await {
                    Ok(_) => {
                        debug!("Removed cache file: {}", path.display());
                        cleared_count += 1;
                    }
                    Err(e) => {
                        warn!("Failed to remove cache file {}: {}", path.display(), e);
                    }
                }
            }
        }
        
        debug!("Cleared {} cache entries", cleared_count);
        Ok(())
    }
    
    /// Get cache statistics
    pub async fn get_stats(&self) -> Result<CacheStats, CacheError> {
        let mut stats = CacheStats::default();
        
        if !self.cache_dir.exists() {
            return Ok(stats);
        }
        
        let mut entries = async_fs::read_dir(&self.cache_dir).await?;
        
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            
            if path.is_file() && path.extension().map_or(false, |ext| ext == "json") {
                stats.total_files += 1;
                
                if let Ok(metadata) = entry.metadata().await {
                    stats.total_size += metadata.len();
                }
                
                if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                    if self.is_expired(filename).await {
                        stats.expired_files += 1;
                    }
                }
            }
        }
        
        Ok(stats)
    }
    
    /// Generic method to get cached data
    async fn get_cached_data<T>(&self, filename: &str) -> Option<CachedData<T>>
    where
        T: for<'de> Deserialize<'de>,
    {
        let file_path = self.cache_dir.join(filename);
        
        if !file_path.exists() {
            debug!("Cache file does not exist: {}", file_path.display());
            return None;
        }
        
        match self.read_cache_file(&file_path).await {
            Ok(cached_data) => {
                if cached_data.is_expired() {
                    debug!("Cache file expired: {}", file_path.display());
                    // Optionally remove expired file
                    if let Err(e) = async_fs::remove_file(&file_path).await {
                        warn!("Failed to remove expired cache file {}: {}", file_path.display(), e);
                    }
                    None
                } else {
                    debug!("Cache hit for: {}", filename);
                    Some(cached_data)
                }
            }
            Err(e) => {
                error!("Failed to read cache file {}: {}", file_path.display(), e);
                None
            }
        }
    }
    
    /// Generic method to store cached data
    async fn store_cached_data<T>(&self, filename: &str, data: &CachedData<T>) -> Result<(), CacheError>
    where
        T: Serialize,
    {
        let file_path = self.cache_dir.join(filename);
        let json_data = serde_json::to_string_pretty(data)?;
        
        async_fs::write(&file_path, json_data).await?;
        debug!("Cached data to: {}", file_path.display());
        
        Ok(())
    }
    
    /// Read and deserialize cache file
    async fn read_cache_file<T>(&self, file_path: &Path) -> Result<CachedData<T>, CacheError>
    where
        T: for<'de> Deserialize<'de>,
    {
        let content = async_fs::read_to_string(file_path).await?;
        let cached_data: CachedData<T> = serde_json::from_str(&content)?;
        Ok(cached_data)
    }
}

/// Cache statistics
#[derive(Debug, Default)]
pub struct CacheStats {
    pub total_files: u32,
    pub expired_files: u32,
    pub total_size: u64,
}

impl CacheStats {
    pub fn active_files(&self) -> u32 {
        self.total_files - self.expired_files
    }
    
    pub fn size_mb(&self) -> f64 {
        self.total_size as f64 / (1024.0 * 1024.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use crate::api::client::{Pet, Tag, GENDER_FEMALE};

    fn create_test_pet() -> Pet {
        Pet {
            id: 1,
            name: "Test Pet".to_string(),
            gender: Some(GENDER_FEMALE),
            date_of_birth: Some("2020-01-01".to_string()),
            weight: Some("5.0".to_string()),
            breed: Some("Test Breed".to_string()),
            comments: None,
            household_id: 1,
            breed_id: 1,
            colour_id: Some(1),
            species_id: 1,
            tag_id: 123456789,
            version: 1,
            created_at: "2023-01-01T00:00:00Z".to_string(),
            updated_at: "2023-01-01T00:00:00Z".to_string(),
            photo: None,
            status: None,
            position: None,
            tag: Some(Tag {
                id: 123456789,
                index: Some(1),
                profile: Some(1),
            }),
        }
    }

    #[tokio::test]
    async fn test_cache_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let cache_manager = CacheManager::new(temp_dir.path().to_path_buf(), 1).unwrap();
        
        assert!(temp_dir.path().exists());
        assert_eq!(cache_manager.ttl, Duration::hours(1));
    }

    #[tokio::test]
    async fn test_cache_pets() {
        let temp_dir = TempDir::new().unwrap();
        let cache_manager = CacheManager::new(temp_dir.path().to_path_buf(), 1).unwrap();
        
        let pets = vec![create_test_pet()];
        
        // Cache the pets
        cache_manager.cache_pets(pets.clone()).await.unwrap();
        
        // Retrieve the cached pets
        let cached_pets = cache_manager.get_pets().await.unwrap();
        assert_eq!(cached_pets.data.len(), 1);
        assert_eq!(cached_pets.data[0].id, 1);
        assert_eq!(cached_pets.data[0].name, "Test Pet");
    }

    #[tokio::test]
    async fn test_cache_expiration() {
        let temp_dir = TempDir::new().unwrap();
        // Create cache manager with very short TTL (1 second)
        let mut cache_manager = CacheManager::new(temp_dir.path().to_path_buf(), 0).unwrap();
        cache_manager.ttl = Duration::seconds(1);
        
        let pets = vec![create_test_pet()];
        
        // Cache the pets
        cache_manager.cache_pets(pets).await.unwrap();
        
        // Should be available immediately
        assert!(cache_manager.get_pets().await.is_some());
        
        // Wait for expiration
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        
        // Should be expired now
        assert!(cache_manager.get_pets().await.is_none());
    }

    #[tokio::test]
    async fn test_clear_expired() {
        let temp_dir = TempDir::new().unwrap();
        let mut cache_manager = CacheManager::new(temp_dir.path().to_path_buf(), 0).unwrap();
        cache_manager.ttl = Duration::seconds(1);
        
        let pets = vec![create_test_pet()];
        cache_manager.cache_pets(pets).await.unwrap();
        
        // Wait for expiration
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        
        // Clear expired entries
        cache_manager.clear_expired().await.unwrap();
        
        // Verify file was removed
        let stats = cache_manager.get_stats().await.unwrap();
        assert_eq!(stats.total_files, 0);
    }

    #[tokio::test]
    async fn test_cache_stats() {
        let temp_dir = TempDir::new().unwrap();
        let cache_manager = CacheManager::new(temp_dir.path().to_path_buf(), 24).unwrap();
        
        let pets = vec![create_test_pet()];
        cache_manager.cache_pets(pets).await.unwrap();
        
        let stats = cache_manager.get_stats().await.unwrap();
        assert_eq!(stats.total_files, 1);
        assert_eq!(stats.expired_files, 0);
        assert_eq!(stats.active_files(), 1);
        assert!(stats.total_size > 0);
    }
}