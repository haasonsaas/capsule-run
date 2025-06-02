use crate::api::schema::{IsolationConfig, ResourceLimits};
use crate::error::CapsuleResult;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub defaults: DefaultConfig,
    pub profiles: HashMap<String, ExecutionProfile>,
    pub security: SecurityConfig,
    pub monitoring: MonitoringConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DefaultConfig {
    pub timeout_ms: u64,
    pub resources: ResourceLimits,
    pub isolation: IsolationConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExecutionProfile {
    pub description: Option<String>,
    pub timeout_ms: Option<u64>,
    pub resources: Option<ResourceLimits>,
    pub isolation: Option<IsolationConfig>,
    pub environment: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SecurityConfig {
    pub allowed_commands: Option<Vec<String>>,
    pub blocked_commands: Option<Vec<String>>,
    pub max_concurrent_executions: Option<u32>,
    pub audit_log: Option<AuditConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AuditConfig {
    pub enabled: bool,
    pub log_file: Option<String>,
    pub log_level: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MonitoringConfig {
    pub enabled: bool,
    pub interval_ms: u64,
    pub metrics_export: Option<MetricsExportConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MetricsExportConfig {
    pub prometheus: Option<PrometheusConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PrometheusConfig {
    pub enabled: bool,
    pub port: u16,
    pub path: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            defaults: DefaultConfig {
                timeout_ms: 30_000,
                resources: ResourceLimits::default(),
                isolation: IsolationConfig::default(),
            },
            profiles: HashMap::new(),
            security: SecurityConfig {
                allowed_commands: None,
                blocked_commands: Some(vec![
                    "rm".to_string(),
                    "rmdir".to_string(),
                    "sudo".to_string(),
                    "su".to_string(),
                    "chmod".to_string(),
                    "chown".to_string(),
                ]),
                max_concurrent_executions: Some(10),
                audit_log: Some(AuditConfig {
                    enabled: false,
                    log_file: None,
                    log_level: "info".to_string(),
                }),
            },
            monitoring: MonitoringConfig {
                enabled: true,
                interval_ms: 100,
                metrics_export: None,
            },
        }
    }
}

impl Config {
    pub fn load_from_file(path: &Path) -> CapsuleResult<Self> {
        let content = std::fs::read_to_string(path)?;

        // Support multiple formats
        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            Ok(serde_json::from_str(&content)?)
        } else {
            // Default to TOML
            Ok(toml::from_str(&content).map_err(|e| {
                crate::error::CapsuleError::Config(format!("Failed to parse TOML config: {}", e))
            })?)
        }
    }

    pub fn save_to_file(&self, path: &Path) -> CapsuleResult<()> {
        let content = if path.extension().and_then(|s| s.to_str()) == Some("json") {
            serde_json::to_string_pretty(self)?
        } else {
            toml::to_string_pretty(self).map_err(|e| {
                crate::error::CapsuleError::Config(format!("Failed to serialize config: {}", e))
            })?
        };

        std::fs::write(path, content)?;
        Ok(())
    }

    pub fn get_profile(&self, name: &str) -> Option<&ExecutionProfile> {
        self.profiles.get(name)
    }

    pub fn merge_with_profile(&self, profile_name: Option<&str>) -> Self {
        let mut config = self.clone();

        if let Some(profile_name) = profile_name {
            if let Some(profile) = self.get_profile(profile_name) {
                // Merge profile settings with defaults
                if let Some(timeout) = profile.timeout_ms {
                    config.defaults.timeout_ms = timeout;
                }

                if let Some(resources) = &profile.resources {
                    config.defaults.resources = resources.clone();
                }

                if let Some(isolation) = &profile.isolation {
                    config.defaults.isolation = isolation.clone();
                }
            }
        }

        config
    }

    pub fn validate_command(&self, command: &[String]) -> bool {
        if command.is_empty() {
            return false;
        }

        let command_name = &command[0];

        // Check blocked commands first
        if let Some(blocked) = &self.security.blocked_commands {
            if blocked
                .iter()
                .any(|blocked_cmd| command_name.contains(blocked_cmd))
            {
                return false;
            }
        }

        // Check allowed commands if specified
        if let Some(allowed) = &self.security.allowed_commands {
            return allowed
                .iter()
                .any(|allowed_cmd| command_name.contains(allowed_cmd));
        }

        // If no allowed list is specified, allow by default (after blocked check)
        true
    }
}

pub fn load_config() -> CapsuleResult<Config> {
    // Try to load config from various locations
    let config_paths = [
        "capsule-run.toml",
        "capsule-run.json",
        "config/capsule-run.toml",
        "config/capsule-run.json",
        "~/.config/capsule-run/config.toml",
        "/etc/capsule-run/config.toml",
    ];

    for path_str in &config_paths {
        let path = Path::new(path_str);
        if path.exists() {
            println!("Loading config from: {}", path.display());
            return Config::load_from_file(path);
        }
    }

    // If no config file found, return default config
    println!("No config file found, using defaults");
    Ok(Config::default())
}

pub fn create_default_config_file(path: &Path) -> CapsuleResult<()> {
    let config = Config::default();
    config.save_to_file(path)?;
    println!("Created default config file at: {}", path.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.defaults.timeout_ms, 30_000);
        assert!(config.security.blocked_commands.is_some());
        assert!(config.monitoring.enabled);
    }

    #[test]
    fn test_config_serialization() {
        let config = Config::default();

        // Test TOML serialization
        let toml_file = NamedTempFile::new().unwrap();
        config.save_to_file(toml_file.path()).unwrap();
        let loaded_config = Config::load_from_file(toml_file.path()).unwrap();
        assert_eq!(
            config.defaults.timeout_ms,
            loaded_config.defaults.timeout_ms
        );
    }

    #[test]
    fn test_command_validation() {
        let config = Config::default();

        // Test blocked command
        assert!(!config.validate_command(&["rm".to_string(), "-rf".to_string()]));

        // Test allowed command
        assert!(config.validate_command(&["echo".to_string(), "hello".to_string()]));

        // Test empty command
        assert!(!config.validate_command(&[]));
    }

    #[test]
    fn test_profile_merging() {
        let mut config = Config::default();

        // Add a test profile
        let profile = ExecutionProfile {
            description: Some("Test profile".to_string()),
            timeout_ms: Some(60_000),
            resources: None,
            isolation: None,
            environment: None,
        };
        config.profiles.insert("test".to_string(), profile);

        // Test merging
        let merged = config.merge_with_profile(Some("test"));
        assert_eq!(merged.defaults.timeout_ms, 60_000);

        // Test with non-existent profile
        let merged = config.merge_with_profile(Some("nonexistent"));
        assert_eq!(merged.defaults.timeout_ms, 30_000);
    }
}
