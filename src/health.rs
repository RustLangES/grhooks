use axum::{extract::State, http::StatusCode, response::Json};
use serde::Serialize;
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::GlobalConfig;

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: u64,
    pub version: String,
    pub uptime_seconds: u64,
    pub checks: HashMap<String, HealthCheck>,
}

#[derive(Serialize)]
pub struct HealthCheck {
    pub status: String,
    pub message: String,
    pub timestamp: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl HealthCheck {
    pub fn ok(message: &str) -> Self {
        Self {
            status: "ok".to_string(),
            message: message.to_string(),
            timestamp: current_timestamp(),
            details: None,
        }
    }

    pub fn ok_with_details(message: &str, details: serde_json::Value) -> Self {
        Self {
            status: "ok".to_string(),
            message: message.to_string(),
            timestamp: current_timestamp(),
            details: Some(details),
        }
    }

    pub fn warning(message: &str) -> Self {
        Self {
            status: "warning".to_string(),
            message: message.to_string(),
            timestamp: current_timestamp(),
            details: None,
        }
    }

    pub fn error(message: &str) -> Self {
        Self {
            status: "error".to_string(),
            message: message.to_string(),
            timestamp: current_timestamp(),
            details: None,
        }
    }
}

static START_TIME: std::sync::OnceLock<SystemTime> = std::sync::OnceLock::new();

pub fn init_health_system() {
    START_TIME.set(SystemTime::now()).ok();
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs()
}

fn get_uptime() -> u64 {
    START_TIME
        .get()
        .and_then(|start| SystemTime::now().duration_since(*start).ok())
        .unwrap_or(Duration::ZERO)
        .as_secs()
}

async fn check_configuration(config: &GlobalConfig) -> HealthCheck {
    let config_guard = config.read().await;
    
    if config_guard.webhooks.is_empty() {
        return HealthCheck::warning("No webhooks configured");
    }

    let webhook_count = config_guard.webhooks.len();
    let paths: Vec<String> = config_guard.webhooks.iter().map(|w| w.path.clone()).collect();
    
    // Check for duplicate paths
    let mut unique_paths = std::collections::HashSet::new();
    let mut duplicates = Vec::new();
    
    for path in &paths {
        if !unique_paths.insert(path) {
            duplicates.push(path.clone());
        }
    }
    
    let details = serde_json::json!({
        "webhook_count": webhook_count,
        "webhook_paths": paths,
        "port": config_guard.port,
        "duplicates": duplicates
    });

    if !duplicates.is_empty() {
        HealthCheck {
            status: "error".to_string(),
            message: format!("Duplicate webhook paths found: {:?}", duplicates),
            timestamp: current_timestamp(),
            details: Some(details),
        }
    } else {
        HealthCheck::ok_with_details(
            &format!("{} webhooks configured", webhook_count),
            details
        )
    }
}

fn check_memory() -> HealthCheck {
    // Basic memory check - in a real scenario you might want to use a memory profiling crate
    match std::alloc::System.by_ref() {
        _ => HealthCheck::ok("Memory allocator available"),
    }
}

fn check_filesystem() -> HealthCheck {
    use std::fs;
    
    // Check if we can write to temp directory
    let temp_file = std::env::temp_dir().join("grhooks_health_check");
    
    match fs::write(&temp_file, "health check") {
        Ok(_) => {
            // Clean up
            let _ = fs::remove_file(&temp_file);
            HealthCheck::ok("Filesystem write access available")
        }
        Err(e) => HealthCheck::error(&format!("Filesystem write failed: {}", e)),
    }
}

async fn check_webhooks_health(config: &GlobalConfig) -> HealthCheck {
    let config_guard = config.read().await;
    
    let mut webhook_details = Vec::new();
    let mut warnings = Vec::new();
    
    for webhook in &config_guard.webhooks {
        let mut webhook_info = serde_json::json!({
            "path": webhook.path,
            "events": webhook.events.iter().collect::<Vec<_>>(),
            "has_secret": webhook.secret.is_some(),
            "has_command": webhook.command.is_some(),
            "has_script": webhook.script.is_some(),
        });
        
        // Validate webhook configuration
        if webhook.command.is_none() && webhook.script.is_none() {
            warnings.push(format!("Webhook '{}' has no command or script", webhook.path));
            webhook_info["warning"] = serde_json::json!("No command or script configured");
        }
        
        if webhook.events.is_empty() {
            warnings.push(format!("Webhook '{}' has no events configured", webhook.path));
            webhook_info["warning"] = serde_json::json!("No events configured");
        }
        
        // Check if script file exists
        if let Some(script_path) = &webhook.script {
            if !script_path.exists() {
                warnings.push(format!("Script file not found for webhook '{}': {:?}", webhook.path, script_path));
                webhook_info["error"] = serde_json::json!("Script file not found");
            }
        }
        
        webhook_details.push(webhook_info);
    }
    
    let details = serde_json::json!({
        "webhooks": webhook_details,
        "warnings": warnings
    });
    
    if warnings.is_empty() {
        HealthCheck::ok_with_details("All webhooks configured correctly", details)
    } else {
        HealthCheck {
            status: "warning".to_string(),
            message: format!("Found {} webhook configuration issues", warnings.len()),
            timestamp: current_timestamp(),
            details: Some(details),
        }
    }
}

pub async fn health_handler(State(config): State<GlobalConfig>) -> (StatusCode, Json<HealthResponse>) {
    let mut checks = HashMap::new();
    
    // Configuration check
    checks.insert("configuration".to_string(), check_configuration(&config).await);
    
    // Memory check
    checks.insert("memory".to_string(), check_memory());
    
    // Filesystem check
    checks.insert("filesystem".to_string(), check_filesystem());
    
    // Webhooks health check
    checks.insert("webhooks".to_string(), check_webhooks_health(&config).await);
    
    // System info check
    let system_info = serde_json::json!({
        "rust_version": env!("RUSTC_VERSION"),
        "target": env!("TARGET"),
        "profile": if cfg!(debug_assertions) { "debug" } else { "release" },
    });
    checks.insert("system".to_string(), HealthCheck::ok_with_details("System info", system_info));
    
    // Determine overall status
    let has_errors = checks.values().any(|check| check.status == "error");
    let has_warnings = checks.values().any(|check| check.status == "warning");
    
    let overall_status = if has_errors {
        "unhealthy"
    } else if has_warnings {
        "degraded"
    } else {
        "healthy"
    };
    
    let status_code = if has_errors {
        StatusCode::SERVICE_UNAVAILABLE
    } else if has_warnings {
        StatusCode::OK // Still return 200 for warnings, but mark as degraded
    } else {
        StatusCode::OK
    };
    
    let response = HealthResponse {
        status: overall_status.to_string(),
        timestamp: current_timestamp(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: get_uptime(),
        checks,
    };
    
    (status_code, Json(response))
}

pub async fn readiness_handler(State(config): State<GlobalConfig>) -> StatusCode {
    let config_guard = config.read().await;
    
    // Simple readiness check - service is ready if:
    // 1. Has at least one webhook configured
    // 2. Can access configuration
    if !config_guard.webhooks.is_empty() {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    }
}

pub async fn liveness_handler() -> StatusCode {
    // Simple liveness check - service is alive if it can respond
    StatusCode::OK
}