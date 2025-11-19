use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;
use uuid::Uuid;

pub type DeviceId = String;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum DeviceType {
    CiscoIos,
    JuniperJunos,
    GenericSsh,
    AristaEos,
    CiscoNxosApi,
    MerakiCloud,
}

impl FromStr for DeviceType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "ciscoios" | "cisco_ios" => Ok(DeviceType::CiscoIos),
            "juniperjunos" | "juniper_junos" => Ok(DeviceType::JuniperJunos),
            "genericssh" | "generic_ssh" => Ok(DeviceType::GenericSsh),
            "aristaeos" | "arista_eos" => Ok(DeviceType::AristaEos),
            "cisconxosapi" | "cisco_nxos_api" | "nxos" => Ok(DeviceType::CiscoNxosApi),
            "merakicloud" | "meraki_cloud" => Ok(DeviceType::MerakiCloud),
            other => Err(format!("unknown device type '{}'", other)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialRef {
    pub name: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum Credential {
    UserPassword {
        username: String,
        password: String,
    },
    SshKey {
        username: String,
        key_path: String,
        passphrase: Option<String>,
    },
    Token {
        token: String,
    },
}

impl fmt::Debug for Credential {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Credential::UserPassword { username, .. } => f
                .debug_struct("UserPassword")
                .field("username", username)
                .field("password", &"******")
                .finish(),
            Credential::SshKey {
                username,
                key_path,
                ..
            } => f
                .debug_struct("SshKey")
                .field("username", username)
                .field("key_path", key_path)
                .field("passphrase", &"******")
                .finish(),
            Credential::Token { .. } => f.debug_struct("Token").field("token", &"******").finish(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    pub id: DeviceId,
    pub name: String,
    pub device_type: DeviceType,
    pub mgmt_address: String,
    pub credential: CredentialRef,
    pub tags: Vec<String>,
    pub capabilities: CapabilitySet,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct CapabilitySet {
    pub supports_commit: bool,
    pub supports_rollback: bool,
    pub supports_diff: bool,
    pub supports_dry_run: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: Uuid,
    pub name: String,
    pub kind: JobKind,
    pub targets: TargetSelector,
    pub parameters: HashMap<String, serde_json::Value>,
    pub max_parallel: Option<usize>,
    pub dry_run: bool,
    pub approval_id: Option<Uuid>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum JobKind {
    CommandBatch { commands: Vec<String> },
    ConfigPush { snippet: String },
    ComplianceCheck { rules: Vec<ComplianceRule> },
}

impl fmt::Debug for JobKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JobKind::CommandBatch { commands } => f
                .debug_struct("CommandBatch")
                .field("commands", commands)
                .finish(),
            JobKind::ConfigPush { snippet: _ } => f
                .debug_struct("ConfigPush")
                .field("snippet", &"***redacted***")
                .finish(),
            JobKind::ComplianceCheck { rules } => f
                .debug_struct("ComplianceCheck")
                .field("rules", rules)
                .finish(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceRule {
    pub name: String,
    pub description: String,
    pub expression: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum TargetSelector {
    All,
    ByIds { ids: Vec<DeviceId> },
    ByTags { all_of: Vec<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSummary {
    pub device_id: DeviceId,
    pub status: TaskStatus,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub logs: Vec<String>,
    pub diff: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskStatus {
    Pending,
    Running,
    Success,
    Failed,
    Skipped,
    RolledBack,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobResult {
    pub job_id: Uuid,
    pub started_at: DateTime<Utc>,
    pub finished_at: DateTime<Utc>,
    pub device_results: Vec<TaskSummary>,
}

impl JobResult {
    pub fn success_count(&self) -> usize {
        self.device_results
            .iter()
            .filter(|r| r.status == TaskStatus::Success)
            .count()
    }
}
