#![allow(dead_code)] // Module contains future functionality not yet integrated

use crate::api::client::{Client, Pet, Device, PetLocationUpdate, DeviceCommand};
use crate::cache::CacheManager;
use crate::offline_queue::{OperationQueue, QueuedOperation, OperationResult, SyncResult, check_connectivity};
use crate::errors::CliError;
use log::{debug, info, warn};
use std::sync::Arc;

/// Offline manager that coordinates caching and operation queuing
pub struct OfflineManager {
    #[allow(dead_code)] // Future functionality
    cache: CacheManager,
    #[allow(dead_code)] // Future functionality
    queue: OperationQueue,
    #[allow(dead_code)] // Future functionality
    api_client: Arc<Client>,
}

impl OfflineManager {
    /// Create a new offline manager with default cache and queue
    pub fn new(api_client: Arc<Client>) -> Result<Self, CliError> {
        let cache = CacheManager::default().map_err(|e| {
            CliError::system_error_with_source(
                "Failed to initialize cache manager",
                Some("Check file system permissions".to_string()),
                Box::new(e),
            )
        })?;
        
        let queue = OperationQueue::default()?;
        
        Ok(Self {
            cache,
            queue,
            api_client,
        })
    }
    
    /// Create offline manager with custom cache and queue paths
    pub fn with_paths(
        api_client: Arc<Client>,
        cache_dir: std::path::PathBuf,
        queue_file: std::path::PathBuf,
        cache_ttl_hours: u64,
    ) -> Result<Self, CliError> {
        let cache = CacheManager::new(cache_dir, cache_ttl_hours).map_err(|e| {
            CliError::system_error_with_source(
                "Failed to initialize cache manager",
                Some("Check file system permissions".to_string()),
                Box::new(e),
            )
        })?;
        
        let queue = OperationQueue::new(queue_file);
        
        Ok(Self {
            cache,
            queue,
            api_client,
        })
    }
    
    /// Get pets with offline support - returns cached data if API is unavailable
    pub async fn get_pets(&self, token: &str, force_refresh: bool) -> Result<(Vec<Pet>, bool), CliError> {
        // Check if we should use cache
        if !force_refresh {
            if let Some(cached_pets) = self.cache.get_pets().await {
                debug!("Using cached pets data (age: {:?})", cached_pets.age());
                return Ok((cached_pets.data, true)); // true indicates cached data
            }
        }
        
        // Try to fetch from API
        match self.api_client.get_pets(token).await {
            Ok(pets_response) => {
                let pets = pets_response.data;
                
                // Cache the fresh data
                if let Err(e) = self.cache.cache_pets(pets.clone()).await {
                    warn!("Failed to cache pets data: {}", e);
                }
                
                Ok((pets, false)) // false indicates fresh data
            }
            Err(e) => {
                // API failed, try to use cached data even if expired
                if let Some(cached_pets) = self.cache.get_pets().await {
                    warn!("API failed, using cached pets data: {}", e);
                    Ok((cached_pets.data, true))
                } else {
                    Err(CliError::network_error(
                        &format!("Failed to get pets and no cached data available: {}", e),
                        true,
                    ))
                }
            }
        }
    }
    
    /// Get devices with offline support - returns cached data if API is unavailable
    pub async fn get_devices(&self, token: &str, force_refresh: bool) -> Result<(Vec<Device>, bool), CliError> {
        // Check if we should use cache
        if !force_refresh {
            if let Some(cached_devices) = self.cache.get_devices().await {
                debug!("Using cached devices data (age: {:?})", cached_devices.age());
                return Ok((cached_devices.data, true)); // true indicates cached data
            }
        }
        
        // Try to fetch from API
        match self.api_client.get_devices(token).await {
            Ok(devices_response) => {
                let devices = devices_response.data;
                
                // Cache the fresh data
                if let Err(e) = self.cache.cache_devices(devices.clone()).await {
                    warn!("Failed to cache devices data: {}", e);
                }
                
                Ok((devices, false)) // false indicates fresh data
            }
            Err(e) => {
                // API failed, try to use cached data even if expired
                if let Some(cached_devices) = self.cache.get_devices().await {
                    warn!("API failed, using cached devices data: {}", e);
                    Ok((cached_devices.data, true))
                } else {
                    Err(CliError::network_error(
                        &format!("Failed to get devices and no cached data available: {}", e),
                        true,
                    ))
                }
            }
        }
    }
    
    /// Set pet location with offline queuing support
    pub async fn set_pet_location(&self, token: &str, pet_id: u32, location: u32) -> Result<(), CliError> {
        // Try to execute immediately
        match self.api_client.set_pet_location(token, pet_id, location).await {
            Ok(_) => {
                debug!("Pet location set successfully");
                Ok(())
            }
            Err(e) => {
                // Check if this is a network error that should be queued
                if self.should_queue_operation(&e).await {
                    let operation = QueuedOperation::SetPetLocation { pet_id, location };
                    let operation_id = self.queue.enqueue(operation).await?;
                    
                    info!("Queued pet location operation {} for offline execution", operation_id);
                    Ok(())
                } else {
                    Err(CliError::network_error(
                        &format!("Failed to set pet location: {}", e),
                        false,
                    ))
                }
            }
        }
    }
    
    /// Set device lock state with offline queuing support
    pub async fn set_device_lock_state(&self, token: &str, device_id: u32, lock_state: u32) -> Result<(), CliError> {
        // Try to execute immediately
        match self.api_client.set_lock_state(token, device_id, lock_state).await {
            Ok(_) => {
                debug!("Device lock state set successfully");
                Ok(())
            }
            Err(e) => {
                // Check if this is a network error that should be queued
                if self.should_queue_operation(&e).await {
                    let operation = QueuedOperation::SetDeviceLockState { device_id, lock_state };
                    let operation_id = self.queue.enqueue(operation).await?;
                    
                    info!("Queued device lock state operation {} for offline execution", operation_id);
                    Ok(())
                } else {
                    Err(CliError::network_error(
                        &format!("Failed to set device lock state: {}", e),
                        false,
                    ))
                }
            }
        }
    }
    
    /// Batch set pet locations with offline queuing support
    pub async fn batch_set_pet_locations(&self, token: &str, updates: Vec<PetLocationUpdate>) -> Result<(), CliError> {
        // Try to execute immediately
        match self.api_client.batch_set_pet_locations(token, updates.clone()).await {
            Ok(result) => {
                debug!("Batch pet location update completed: {} successful, {} failed", 
                       result.successful.len(), result.failed.len());
                
                // If some operations failed due to network issues, queue them
                if !result.failed.is_empty() {
                    let failed_updates: Vec<PetLocationUpdate> = result.failed.iter()
                        .filter_map(|error| {
                            updates.iter().find(|update| update.pet_id == error.id).cloned()
                        })
                        .collect();
                    
                    if !failed_updates.is_empty() {
                        let operation = QueuedOperation::BatchSetPetLocations { updates: failed_updates };
                        let operation_id = self.queue.enqueue(operation).await?;
                        info!("Queued {} failed pet location operations for retry: {}", 
                              result.failed.len(), operation_id);
                    }
                }
                
                Ok(())
            }
            Err(e) => {
                // Check if this is a network error that should be queued
                if self.should_queue_operation(&e).await {
                    let operation = QueuedOperation::BatchSetPetLocations { updates };
                    let operation_id = self.queue.enqueue(operation).await?;
                    
                    info!("Queued batch pet location operation {} for offline execution", operation_id);
                    Ok(())
                } else {
                    Err(CliError::network_error(
                        &format!("Failed to batch set pet locations: {}", e),
                        false,
                    ))
                }
            }
        }
    }
    
    /// Batch device control with offline queuing support
    pub async fn batch_device_control(&self, token: &str, commands: Vec<DeviceCommand>) -> Result<(), CliError> {
        // Try to execute immediately
        match self.api_client.batch_device_control(token, commands.clone()).await {
            Ok(result) => {
                debug!("Batch device control completed: {} successful, {} failed", 
                       result.successful.len(), result.failed.len());
                
                // If some operations failed due to network issues, queue them
                if !result.failed.is_empty() {
                    let failed_commands: Vec<DeviceCommand> = result.failed.iter()
                        .filter_map(|error| {
                            commands.iter().find(|cmd| cmd.device_id == error.id).cloned()
                        })
                        .collect();
                    
                    if !failed_commands.is_empty() {
                        let operation = QueuedOperation::BatchDeviceControl { commands: failed_commands };
                        let operation_id = self.queue.enqueue(operation).await?;
                        info!("Queued {} failed device control operations for retry: {}", 
                              result.failed.len(), operation_id);
                    }
                }
                
                Ok(())
            }
            Err(e) => {
                // Check if this is a network error that should be queued
                if self.should_queue_operation(&e).await {
                    let operation = QueuedOperation::BatchDeviceControl { commands };
                    let operation_id = self.queue.enqueue(operation).await?;
                    
                    info!("Queued batch device control operation {} for offline execution", operation_id);
                    Ok(())
                } else {
                    Err(CliError::network_error(
                        &format!("Failed to batch control devices: {}", e),
                        false,
                    ))
                }
            }
        }
    }
    
    /// Check connectivity and synchronize queued operations
    pub async fn synchronize_when_online(&self, token: &str) -> Result<Option<SyncResult>, CliError> {
        // Check if we have any queued operations
        if self.queue.is_empty().await? {
            debug!("No operations to synchronize");
            return Ok(None);
        }
        
        // Check connectivity
        let api_url = &self.api_client.cfg.api.surehub_url;
        if !check_connectivity(api_url).await {
            debug!("No connectivity, skipping synchronization");
            return Ok(None);
        }
        
        info!("Connectivity restored, synchronizing queued operations");
        
        // Create executor closure that uses our API client
        let api_client = self.api_client.clone();
        let token = token.to_string();
        
        let executor = move |operation: QueuedOperation| {
            let client = api_client.clone();
            let token = token.clone();
            
            async move {
                match operation {
                    QueuedOperation::SetPetLocation { pet_id, location } => {
                        match client.set_pet_location(&token, pet_id, location).await {
                            Ok(_) => OperationResult::Success,
                            Err(e) => {
                                if is_retryable_error(&e) {
                                    OperationResult::Retry(format!("Network error: {}", e))
                                } else {
                                    OperationResult::Fail(format!("Permanent error: {}", e))
                                }
                            }
                        }
                    }
                    QueuedOperation::SetDeviceLockState { device_id, lock_state } => {
                        match client.set_lock_state(&token, device_id, lock_state).await {
                            Ok(_) => OperationResult::Success,
                            Err(e) => {
                                if is_retryable_error(&e) {
                                    OperationResult::Retry(format!("Network error: {}", e))
                                } else {
                                    OperationResult::Fail(format!("Permanent error: {}", e))
                                }
                            }
                        }
                    }
                    QueuedOperation::SetDeviceCurfew { device_id, curfew_times } => {
                        match client.set_curfew(&token, device_id, curfew_times).await {
                            Ok(_) => OperationResult::Success,
                            Err(e) => {
                                if is_retryable_error(&e) {
                                    OperationResult::Retry(format!("Network error: {}", e))
                                } else {
                                    OperationResult::Fail(format!("Permanent error: {}", e))
                                }
                            }
                        }
                    }
                    QueuedOperation::BatchSetPetLocations { updates } => {
                        match client.batch_set_pet_locations(&token, updates).await {
                            Ok(_) => OperationResult::Success,
                            Err(e) => {
                                if is_retryable_error(&e) {
                                    OperationResult::Retry(format!("Network error: {}", e))
                                } else {
                                    OperationResult::Fail(format!("Permanent error: {}", e))
                                }
                            }
                        }
                    }
                    QueuedOperation::BatchDeviceControl { commands } => {
                        match client.batch_device_control(&token, commands).await {
                            Ok(_) => OperationResult::Success,
                            Err(e) => {
                                if is_retryable_error(&e) {
                                    OperationResult::Retry(format!("Network error: {}", e))
                                } else {
                                    OperationResult::Fail(format!("Permanent error: {}", e))
                                }
                            }
                        }
                    }
                }
            }
        };
        
        let result = self.queue.synchronize(executor).await?;
        Ok(Some(result))
    }
    
    /// Get queue status for display
    pub async fn get_queue_status(&self) -> Result<(usize, Vec<String>), CliError> {
        let operations = self.queue.get_all().await?;
        let count = operations.len();
        
        let descriptions: Vec<String> = operations.iter().map(|entry| {
            match &entry.operation {
                QueuedOperation::SetPetLocation { pet_id, location } => {
                    format!("Set pet {} location to {}", pet_id, location)
                }
                QueuedOperation::SetDeviceLockState { device_id, lock_state } => {
                    format!("Set device {} lock state to {}", device_id, lock_state)
                }
                QueuedOperation::SetDeviceCurfew { device_id, .. } => {
                    format!("Set device {} curfew", device_id)
                }
                QueuedOperation::BatchSetPetLocations { updates } => {
                    format!("Batch set locations for {} pets", updates.len())
                }
                QueuedOperation::BatchDeviceControl { commands } => {
                    format!("Batch control {} devices", commands.len())
                }
            }
        }).collect();
        
        Ok((count, descriptions))
    }
    
    /// Clear cache and queue
    pub async fn clear_all(&self) -> Result<(), CliError> {
        self.cache.clear_all().await.map_err(|e| {
            CliError::system_error_with_source(
                "Failed to clear cache",
                None,
                Box::new(e),
            )
        })?;
        
        self.queue.clear().await?;
        
        info!("Cleared all cached data and queued operations");
        Ok(())
    }
    
    /// Check if an error should result in queuing the operation
    async fn should_queue_operation(&self, error: &reqwest::Error) -> bool {
        // Queue operations for network-related errors
        error.is_timeout() || error.is_connect() || error.is_request()
    }
}

/// Check if an error is retryable (network-related)
fn is_retryable_error(error: &reqwest::Error) -> bool {
    error.is_timeout() || error.is_connect() || error.is_request()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use tempfile::TempDir;

    fn create_test_client() -> Arc<Client> {
        let config = Config {
            api: crate::config::Api {
                surehub_url: "https://app.api.surehub.io".to_string(),
            },
        };
        Arc::new(Client::new(config))
    }

    #[tokio::test]
    async fn test_offline_manager_creation() {
        let client = create_test_client();
        let manager = OfflineManager::new(client);
        assert!(manager.is_ok());
    }

    #[tokio::test]
    async fn test_offline_manager_with_custom_paths() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path().join("cache");
        let queue_file = temp_dir.path().join("queue.json");
        
        let client = create_test_client();
        let manager = OfflineManager::with_paths(client, cache_dir, queue_file, 24);
        assert!(manager.is_ok());
    }

    #[tokio::test]
    async fn test_queue_status() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path().join("cache");
        let queue_file = temp_dir.path().join("queue.json");
        
        let client = create_test_client();
        let manager = OfflineManager::with_paths(client, cache_dir, queue_file, 24).unwrap();
        
        // Initially empty
        let (count, descriptions) = manager.get_queue_status().await.unwrap();
        assert_eq!(count, 0);
        assert!(descriptions.is_empty());
    }

    #[tokio::test]
    async fn test_clear_all() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path().join("cache");
        let queue_file = temp_dir.path().join("queue.json");
        
        let client = create_test_client();
        let manager = OfflineManager::with_paths(client, cache_dir, queue_file, 24).unwrap();
        
        // Should not fail even when empty
        let result = manager.clear_all().await;
        assert!(result.is_ok());
    }
}