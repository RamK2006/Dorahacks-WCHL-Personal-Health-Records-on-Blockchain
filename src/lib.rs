use candid::{CandidType, Deserialize, Principal};
use ic_cdk::api::caller;
use ic_cdk_macros::{init, post_upgrade, pre_upgrade, query, update};
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{DefaultMemoryImpl, StableBTreeMap};
use serde::Serialize;
use std::cell::RefCell;

// Type aliases for memory management
type Memory = VirtualMemory<DefaultMemoryImpl>;
type UserRecordsMap = StableBTreeMap<Principal, Vec<HealthRecord>, Memory>;

// Health Record structure
#[derive(CandidType, Deserialize, Serialize, Clone, Debug)]
pub struct HealthRecord {
    pub id: String,
    pub title: String,
    pub record_type: String,
    pub date: u64, // Unix timestamp
    pub encrypted_url: String, // IPFS or IC storage URL
    pub file_size: Option<u64>,
    pub created_at: u64,
}

// Request structure for adding new records
#[derive(CandidType, Deserialize)]
pub struct AddRecordRequest {
    pub title: String,
    pub record_type: String,
    pub encrypted_url: String,
    pub file_size: Option<u64>,
}

// Response structures
#[derive(CandidType, Deserialize)]
pub struct ApiResponse {
    pub success: bool,
    pub message: String,
    pub data: Option<Vec<HealthRecord>>,
}

// Thread-local storage for the canister state
thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> = 
        RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));
    
    static USER_RECORDS: RefCell<UserRecordsMap> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(0)))
        )
    );
}

// Initialize canister
#[init]
fn init() {
    // Initialization logic if needed
}

// Pre-upgrade hook
#[pre_upgrade]
fn pre_upgrade() {
    // Any cleanup before upgrade
}

// Post-upgrade hook
#[post_upgrade]
fn post_upgrade() {
    // Any setup after upgrade
}

// Generate unique ID for records
fn generate_record_id(user: &Principal, timestamp: u64) -> String {
    format!("{}_{}", user.to_text(), timestamp)
}

// Get current timestamp (in nanoseconds, convert to seconds)
fn get_current_timestamp() -> u64 {
    ic_cdk::api::time() / 1_000_000_000
}

// Add a new health record for the caller
#[update]
fn add_record(request: AddRecordRequest) -> ApiResponse {
    let caller = caller();
    
    // Validate caller is not anonymous
    if caller == Principal::anonymous() {
        return ApiResponse {
            success: false,
            message: "Anonymous users cannot add records".to_string(),
            data: None,
        };
    }

    // Validate input
    if request.title.trim().is_empty() {
        return ApiResponse {
            success: false,
            message: "Title cannot be empty".to_string(),
            data: None,
        };
    }

    if request.record_type.trim().is_empty() {
        return ApiResponse {
            success: false,
            message: "Record type cannot be empty".to_string(),
            data: None,
        };
    }

    let current_time = get_current_timestamp();
    
    // Create new health record
    let new_record = HealthRecord {
        id: generate_record_id(&caller, current_time),
        title: request.title.trim().to_string(),
        record_type: request.record_type.trim().to_string(),
        date: current_time,
        encrypted_url: request.encrypted_url,
        file_size: request.file_size,
        created_at: current_time,
    };

    // Add record to user's records
    USER_RECORDS.with(|records| {
        let mut records = records.borrow_mut();
        let mut user_records = records.get(&caller).unwrap_or_default();
        user_records.push(new_record);
        records.insert(caller, user_records);
    });

    ApiResponse {
        success: true,
        message: "Record added successfully".to_string(),
        data: None,
    }
}

// Get all records for the caller
#[query]
fn get_my_records() -> ApiResponse {
    let caller = caller();
    
    // Check if caller is authenticated
    if caller == Principal::anonymous() {
        return ApiResponse {
            success: false,
            message: "Authentication required".to_string(),
            data: None,
        };
    }

    USER_RECORDS.with(|records| {
        let records = records.borrow();
        let user_records = records.get(&caller).unwrap_or_default();
        
        ApiResponse {
            success: true,
            message: format!("Found {} records", user_records.len()),
            data: Some(user_records),
        }
    })
}

// Get a specific record by ID (only if owned by caller)
#[query]
fn get_record_by_id(record_id: String) -> ApiResponse {
    let caller = caller();
    
    if caller == Principal::anonymous() {
        return ApiResponse {
            success: false,
            message: "Authentication required".to_string(),
            data: None,
        };
    }

    USER_RECORDS.with(|records| {
        let records = records.borrow();
        let user_records = records.get(&caller).unwrap_or_default();
        
        if let Some(record) = user_records.iter().find(|r| r.id == record_id) {
            ApiResponse {
                success: true,
                message: "Record found".to_string(),
                data: Some(vec![record.clone()]),
            }
        } else {
            ApiResponse {
                success: false,
                message: "Record not found or access denied".to_string(),
                data: None,
            }
        }
    })
}

// Delete a record by ID (only if owned by caller)
#[update]
fn delete_record(record_id: String) -> ApiResponse {
    let caller = caller();
    
    if caller == Principal::anonymous() {
        return ApiResponse {
            success: false,
            message: "Authentication required".to_string(),
            data: None,
        };
    }

    USER_RECORDS.with(|records| {
        let mut records = records.borrow_mut();
        let mut user_records = records.get(&caller).unwrap_or_default();
        
        let initial_len = user_records.len();
        user_records.retain(|r| r.id != record_id);
        
        if user_records.len() < initial_len {
            records.insert(caller, user_records);
            ApiResponse {
                success: true,
                message: "Record deleted successfully".to_string(),
                data: None,
            }
        } else {
            ApiResponse {
                success: false,
                message: "Record not found or access denied".to_string(),
                data: None,
            }
        }
    })
}

// Get total number of records for the caller
#[query]
fn get_record_count() -> u64 {
    let caller = caller();
    
    if caller == Principal::anonymous() {
        return 0;
    }

    USER_RECORDS.with(|records| {
        let records = records.borrow();
        records.get(&caller).unwrap_or_default().len() as u64
    })
}

// Health check endpoint
#[query]
fn health_check() -> String {
    "Health Records Backend is running".to_string()
}

// Get caller's principal (for debugging)
#[query]
fn whoami() -> Principal {
    caller()
}