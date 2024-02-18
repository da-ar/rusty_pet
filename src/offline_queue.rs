#![allow(dead_code)] // Module contains future functionality not yet integrated

use crate::api::client::{PetLocationUpdate, DeviceCommand, CurfewTime};
use crate::errors::CliError;
use chrono::{DateTime, Utc};
use log::{debug, warn, error, info};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs as async_fs;
use uuid::Uuid;

/// Types of operations that can be queued for offline execution
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum QueuedOperation {
    SetPetLocation {
        pet_id: u32,
        location: u32,
    },
    SetDeviceLockState {
        device_id: u32,
        lock_state: u32,
    },
    SetDeviceCurfew {
        device_id: u32,
        curfew_times: Vec<CurfewTime>,
    },
    BatchSetPetLocations {
        updates: Vec<PetLocationUpdate>,
    },
    BatchDeviceControl {
        commands: Vec<DeviceCommand>,
    },
}

/// Queued operation with metadata
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct QueuedOperationEntry {
    pub id: String,
    pub operation: QueuedOperation,
    pub queued_at: DateTime<Utc>,
    pub retry_count: u32,
    pub max_retries: u32,
}

impl QueuedOperationEntry {
    pub fn new(operation: QueuedOperation) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            operation,
            queued_at: Utc::now(),
            retry_count: 0,
            max_retries: 3,
        }
    }
    
    pub fn can_retry(&self) -> bool {
        self.retry_count < self.max_retries
    }
    
    pub fn increment_retry(&mut self) {
        self.retry_count += 1;
    }
}

/// Result of executing a queued operation
#[derive(Debug)]
pub enum OperationResult {
    Success,
    Retry(String), // Error message, should retry
    Fail(String),  // Error message, don't retry
}

/// Synchronization result summary
#[derive(Debug)]
pub struct SyncResult {
    pub total_operations: usize,
    pub successful: usize,
    pub failed: usize,
    pub retried: usize,
}

/// Operation queue manager for offline mode
pub struct OperationQueue {
    #[allow(dead_code)] // Future functionality
    queue_file: PathBuf,
}

impl OperationQueue {
    /// Create a new operation queue with specified queue file path
    pub fn new(queue_file: PathBuf) -> Self {
        Self { queue_file }
    }
    
    /// Create a default operation queue using system cache directory
    pub fn default() -> Result<Self, CliError> {
        let queue_dir = dirs::cache_dir()
            .unwrap_or_else(|| dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")))
            .join("rusty_pet");
            
        // Ensure directory exists
        if !queue_dir.exists() {
            std::fs::create_dir_all(&queue_dir).map_err(|e| {
                CliError::system_error_with_source(
                    "Failed to create queue directory",
                    Some("Check file system permissions".to_string()),
                    Box::new(e),
                )
            })?;
        }
        
        let queue_file = queue_dir.join("operation_queue.json");
        Ok(Self::new(queue_file))
    }
    
    /// Add an operation to the queue
    pub async fn enqueue(&self, operation: QueuedOperation) -> Result<String, CliError> {
        let entry = QueuedOperationEntry::new(operation);
        let operation_id = entry.id.clone();
        
        let mut queue = self.load_queue().await?;
        queue.push(entry);
        
        self.save_queue(&queue).await?;
        
        debug!("Queued operation {} for offline execution", operation_id);
        Ok(operation_id)
    }
    
    /// Get the current queue size
    pub async fn size(&self) -> Result<usize, CliError> {
        let queue = self.load_queue().await?;
        Ok(queue.len())
    }
    
    /// Check if the queue is empty
    pub async fn is_empty(&self) -> Result<bool, CliError> {
        let queue = self.load_queue().await?;
        Ok(queue.is_empty())
    }
    
    /// Get all queued operations (for display purposes)
    pub async fn get_all(&self) -> Result<Vec<QueuedOperationEntry>, CliError> {
        self.load_queue().await
    }
    
    /// Clear all operations from the queue
    pub async fn clear(&self) -> Result<(), CliError> {
        self.save_queue(&Vec::new()).await?;
        debug!("Cleared operation queue");
        Ok(())
    }
    
    /// Remove a specific operation from the queue by ID
    pub async fn remove(&self, operation_id: &str) -> Result<bool, CliError> {
        let mut queue = self.load_queue().await?;
        let initial_len = queue.len();
        
        queue.retain(|entry| entry.id != operation_id);
        
        if queue.len() < initial_len {
            self.save_queue(&queue).await?;
            debug!("Removed operation {} from queue", operation_id);
            Ok(true)
        } else {
            Ok(false)
        }
    }
    
    /// Synchronize all queued operations when connectivity returns
    pub async fn synchronize<F, Fut>(&self, executor: F) -> Result<SyncResult, CliError>
    where
        F: Fn(QueuedOperation) -> Fut,
        Fut: std::future::Future<Output = OperationResult>,
    {
        let queue = self.load_queue().await?;
        let total_operations = queue.len();
        
        if total_operations == 0 {
            debug!("No operations to synchronize");
            return Ok(SyncResult {
                total_operations: 0,
                successful: 0,
                failed: 0,
                retried: 0,
            });
        }
        
        info!("Starting synchronization of {} queued operations", total_operations);
        
        let mut successful = 0;
        let mut failed = 0;
        let mut retried = 0;
        let mut operations_to_keep = Vec::new();
        
        for mut entry in queue {
            debug!("Executing queued operation: {}", entry.id);
            
            match executor(entry.operation.clone()).await {
                OperationResult::Success => {
                    debug!("Operation {} executed successfully", entry.id);
                    successful += 1;
                }
                OperationResult::Retry(error_msg) => {
                    if entry.can_retry() {
                        let operation_id = entry.id.clone();
                        entry.increment_retry();
                        operations_to_keep.push(entry);
                        warn!("Operation {} failed, will retry: {}", operation_id, error_msg);
                        retried += 1;
                    } else {
                        error!("Operation {} failed after {} retries: {}", entry.id, entry.retry_count, error_msg);
                        failed += 1;
                    }
                }
                OperationResult::Fail(error_msg) => {
                    error!("Operation {} failed permanently: {}", entry.id, error_msg);
                    failed += 1;
                }
            }
        }
        
        // Save operations that need to be retried
        self.save_queue(&operations_to_keep).await?;
        
        let result = SyncResult {
            total_operations,
            successful,
            failed,
            retried,
        };
        
        info!(
            "Synchronization complete: {} successful, {} failed, {} retried",
            result.successful, result.failed, result.retried
        );
        
        Ok(result)
    }
    
    /// Load the operation queue from disk
    async fn load_queue(&self) -> Result<Vec<QueuedOperationEntry>, CliError> {
        if !self.queue_file.exists() {
            debug!("Queue file does not exist, returning empty queue");
            return Ok(Vec::new());
        }
        
        let content = async_fs::read_to_string(&self.queue_file).await.map_err(|e| {
            CliError::system_error_with_source(
                "Failed to read operation queue file",
                Some("Check file permissions and disk space".to_string()),
                Box::new(e),
            )
        })?;
        
        if content.trim().is_empty() {
            debug!("Queue file is empty, returning empty queue");
            return Ok(Vec::new());
        }
        
        let queue: Vec<QueuedOperationEntry> = serde_json::from_str(&content).map_err(|e| {
            CliError::system_error_with_source(
                "Failed to parse operation queue file",
                Some("Queue file may be corrupted, consider clearing it".to_string()),
                Box::new(e),
            )
        })?;
        
        debug!("Loaded {} operations from queue", queue.len());
        Ok(queue)
    }
    
    /// Save the operation queue to disk
    async fn save_queue(&self, queue: &[QueuedOperationEntry]) -> Result<(), CliError> {
        let json_data = serde_json::to_string_pretty(queue).map_err(|e| {
            CliError::system_error_with_source(
                "Failed to serialize operation queue",
                None,
                Box::new(e),
            )
        })?;
        
        async_fs::write(&self.queue_file, json_data).await.map_err(|e| {
            CliError::system_error_with_source(
                "Failed to write operation queue file",
                Some("Check file permissions and disk space".to_string()),
                Box::new(e),
            )
        })?;
        
        debug!("Saved {} operations to queue", queue.len());
        Ok(())
    }
}

/// Helper function to check network connectivity
pub async fn check_connectivity(api_url: &str) -> bool {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap_or_default();
    
    // Try to make a simple HEAD request to the API
    match client.head(api_url).send().await {
        Ok(response) => {
            debug!("Connectivity check: HTTP {}", response.status());
            response.status().is_success() || response.status().is_client_error()
        }
        Err(e) => {
            debug!("Connectivity check failed: {}", e);
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use crate::api::client::{LOCATION_INSIDE, LOCK_STATE_LOCKED};

    #[tokio::test]
    async fn test_operation_queue_creation() {
        let temp_dir = TempDir::new().unwrap();
        let queue_file = temp_dir.path().join("test_queue.json");
        let queue = OperationQueue::new(queue_file);
        
        assert!(queue.is_empty().await.unwrap());
        assert_eq!(queue.size().await.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_enqueue_operation() {
        let temp_dir = TempDir::new().unwrap();
        let queue_file = temp_dir.path().join("test_queue.json");
        let queue = OperationQueue::new(queue_file);
        
        let operation = QueuedOperation::SetPetLocation {
            pet_id: 1,
            location: LOCATION_INSIDE,
        };
        
        let operation_id = queue.enqueue(operation).await.unwrap();
        assert!(!operation_id.is_empty());
        assert_eq!(queue.size().await.unwrap(), 1);
        assert!(!queue.is_empty().await.unwrap());
    }

    #[tokio::test]
    async fn test_queue_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let queue_file = temp_dir.path().join("test_queue.json");
        
        // Create queue and add operation
        {
            let queue = OperationQueue::new(queue_file.clone());
            let operation = QueuedOperation::SetDeviceLockState {
                device_id: 1,
                lock_state: LOCK_STATE_LOCKED,
            };
            queue.enqueue(operation).await.unwrap();
        }
        
        // Create new queue instance and verify persistence
        {
            let queue = OperationQueue::new(queue_file);
            assert_eq!(queue.size().await.unwrap(), 1);
            
            let operations = queue.get_all().await.unwrap();
            assert_eq!(operations.len(), 1);
            
            match &operations[0].operation {
                QueuedOperation::SetDeviceLockState { device_id, lock_state } => {
                    assert_eq!(*device_id, 1);
                    assert_eq!(*lock_state, LOCK_STATE_LOCKED);
                }
                _ => panic!("Expected SetDeviceLockState operation"),
            }
        }
    }

    #[tokio::test]
    async fn test_remove_operation() {
        let temp_dir = TempDir::new().unwrap();
        let queue_file = temp_dir.path().join("test_queue.json");
        let queue = OperationQueue::new(queue_file);
        
        let operation = QueuedOperation::SetPetLocation {
            pet_id: 1,
            location: LOCATION_INSIDE,
        };
        
        let operation_id = queue.enqueue(operation).await.unwrap();
        assert_eq!(queue.size().await.unwrap(), 1);
        
        let removed = queue.remove(&operation_id).await.unwrap();
        assert!(removed);
        assert_eq!(queue.size().await.unwrap(), 0);
        
        // Try to remove non-existent operation
        let removed = queue.remove("non-existent").await.unwrap();
        assert!(!removed);
    }

    #[tokio::test]
    async fn test_clear_queue() {
        let temp_dir = TempDir::new().unwrap();
        let queue_file = temp_dir.path().join("test_queue.json");
        let queue = OperationQueue::new(queue_file);
        
        // Add multiple operations
        for i in 1..=3 {
            let operation = QueuedOperation::SetPetLocation {
                pet_id: i,
                location: LOCATION_INSIDE,
            };
            queue.enqueue(operation).await.unwrap();
        }
        
        assert_eq!(queue.size().await.unwrap(), 3);
        
        queue.clear().await.unwrap();
        assert_eq!(queue.size().await.unwrap(), 0);
        assert!(queue.is_empty().await.unwrap());
    }

    #[tokio::test]
    async fn test_synchronize_operations() {
        let temp_dir = TempDir::new().unwrap();
        let queue_file = temp_dir.path().join("test_queue.json");
        let queue = OperationQueue::new(queue_file);
        
        // Add test operations
        let operation1 = QueuedOperation::SetPetLocation {
            pet_id: 1,
            location: LOCATION_INSIDE,
        };
        let operation2 = QueuedOperation::SetPetLocation {
            pet_id: 2,
            location: LOCATION_INSIDE,
        };
        
        queue.enqueue(operation1).await.unwrap();
        queue.enqueue(operation2).await.unwrap();
        
        // Mock executor that succeeds for all operations
        let executor = |_operation: QueuedOperation| async {
            OperationResult::Success
        };
        
        let result = queue.synchronize(executor).await.unwrap();
        
        assert_eq!(result.total_operations, 2);
        assert_eq!(result.successful, 2);
        assert_eq!(result.failed, 0);
        assert_eq!(result.retried, 0);
        
        // Queue should be empty after successful sync
        assert!(queue.is_empty().await.unwrap());
    }

    #[tokio::test]
    async fn test_retry_logic() {
        let temp_dir = TempDir::new().unwrap();
        let queue_file = temp_dir.path().join("test_queue.json");
        let queue = OperationQueue::new(queue_file);
        
        let operation = QueuedOperation::SetPetLocation {
            pet_id: 1,
            location: LOCATION_INSIDE,
        };
        
        queue.enqueue(operation).await.unwrap();
        
        // Mock executor that always returns retry
        let executor = |_operation: QueuedOperation| async {
            OperationResult::Retry("Network error".to_string())
        };
        
        let result = queue.synchronize(executor).await.unwrap();
        
        assert_eq!(result.total_operations, 1);
        assert_eq!(result.successful, 0);
        assert_eq!(result.failed, 0);
        assert_eq!(result.retried, 1);
        
        // Operation should still be in queue for retry
        assert_eq!(queue.size().await.unwrap(), 1);
        
        // Check that retry count was incremented
        let operations = queue.get_all().await.unwrap();
        assert_eq!(operations[0].retry_count, 1);
    }
}