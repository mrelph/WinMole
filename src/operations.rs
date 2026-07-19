//! Durable operation journal and typed system-state snapshots.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;

use crate::commands::optimize::common::{
    ActionResult, RegistryValue, RegistryValueType, ServiceStartupType, TweakAction,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationKind {
    ApplyTweak,
    RevertTweak,
    ApplyProfile,
    RevertProfile,
    AuditRemediation,
    RestoreOperation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationStatus {
    Running,
    Preview,
    Succeeded,
    SucceededUnverified,
    Failed,
    RolledBack,
    RollbackFailed,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EffectRequirement {
    Immediate,
    ServiceRestart { service: String },
    ExplorerRestart,
    SignOut,
    Reboot,
}

impl std::fmt::Display for EffectRequirement {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Immediate => write!(formatter, "Immediate"),
            Self::ServiceRestart { service } => write!(formatter, "Restart service: {service}"),
            Self::ExplorerRestart => write!(formatter, "Restart Windows Explorer"),
            Self::SignOut => write!(formatter, "Sign out and back in"),
            Self::Reboot => write!(formatter, "Restart Windows"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ActionState {
    Registry {
        value_type: Option<RegistryValueType>,
        value: Option<RegistryValue>,
    },
    Service {
        startup_type: ServiceStartupType,
    },
    ScheduledTask {
        enabled: bool,
    },
    ActivePowerPlan {
        guid: String,
    },
    AppxPackages {
        package_pattern: String,
        provisioned: bool,
        package_names: Vec<String>,
    },
    Unavailable {
        reason: String,
    },
}

impl ActionState {
    pub fn is_recoverable(&self) -> bool {
        !matches!(self, Self::Unavailable { .. } | Self::AppxPackages { .. })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationAction {
    pub action: TweakAction,
    pub description: String,
    pub before: ActionState,
    pub after: Option<ActionState>,
    pub result: Option<ActionResult>,
    pub rollback_result: Option<ActionResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildIdentity {
    pub version: String,
    pub revision: String,
    pub dirty: bool,
    pub build_timestamp: String,
    pub build_profile: String,
    pub executable: Option<PathBuf>,
}

impl BuildIdentity {
    pub fn current() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            revision: env!("WINMOLE_GIT_REVISION").to_string(),
            dirty: env!("WINMOLE_GIT_DIRTY") == "true",
            build_timestamp: env!("WINMOLE_BUILD_TIMESTAMP").to_string(),
            build_profile: if cfg!(debug_assertions) {
                "debug"
            } else {
                "release"
            }
            .to_string(),
            executable: std::env::current_exe().ok(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationRecord {
    pub id: String,
    pub kind: OperationKind,
    pub target_id: String,
    pub target_name: String,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub status: OperationStatus,
    pub dry_run: bool,
    pub elevated: bool,
    pub build: BuildIdentity,
    pub effects: Vec<EffectRequirement>,
    pub actions: Vec<OperationAction>,
    #[serde(default)]
    pub child_operation_ids: Vec<String>,
    pub error: Option<String>,
}

impl OperationRecord {
    pub fn new(
        kind: OperationKind,
        target_id: impl Into<String>,
        target_name: impl Into<String>,
        dry_run: bool,
        effects: Vec<EffectRequirement>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            kind,
            target_id: target_id.into(),
            target_name: target_name.into(),
            started_at: chrono::Utc::now(),
            completed_at: None,
            status: OperationStatus::Running,
            dry_run,
            elevated: crate::commands::optimize::is_elevated(),
            build: BuildIdentity::current(),
            effects,
            actions: Vec::new(),
            child_operation_ids: Vec::new(),
            error: None,
        }
    }
}

pub struct OperationJournal {
    directory: PathBuf,
}

impl OperationJournal {
    pub fn open() -> Result<Self> {
        let directory = if let Some(directory) = std::env::var_os("WINMOLE_OPERATION_DIR") {
            PathBuf::from(directory)
        } else {
            dirs::data_local_dir()
                .ok_or_else(|| anyhow!("Could not determine local data directory"))?
                .join("WinMole")
                .join("operations")
        };
        std::fs::create_dir_all(&directory)?;
        Ok(Self { directory })
    }

    pub fn save(&self, operation: &OperationRecord) -> Result<()> {
        let path = self.path_for(&operation.id);
        let temporary = path.with_extension("json.tmp");
        std::fs::write(&temporary, serde_json::to_vec_pretty(operation)?)?;
        std::fs::rename(temporary, path)?;
        Ok(())
    }

    pub fn load(&self, id: &str) -> Result<OperationRecord> {
        let path = self.path_for(id);
        if !path.exists() {
            return Err(anyhow!("Operation '{}' was not found", id));
        }
        Ok(serde_json::from_slice(&std::fs::read(path)?)?)
    }

    pub fn list(&self) -> Result<Vec<OperationRecord>> {
        let mut operations = Vec::new();
        for entry in std::fs::read_dir(&self.directory)? {
            let entry = match entry {
                Ok(entry) => entry,
                Err(_) => continue,
            };
            if entry.path().extension().and_then(|value| value.to_str()) != Some("json") {
                continue;
            }
            if let Ok(operation) =
                serde_json::from_slice::<OperationRecord>(&std::fs::read(entry.path())?)
            {
                operations.push(operation);
            }
        }
        operations.sort_by(|left, right| right.started_at.cmp(&left.started_at));
        Ok(operations)
    }

    pub fn latest_recoverable(&self) -> Result<Option<OperationRecord>> {
        Ok(self
            .list()?
            .into_iter()
            .find(|operation| self.can_restore(operation)))
    }

    pub fn can_restore(&self, operation: &OperationRecord) -> bool {
        self.can_restore_inner(operation, &mut HashSet::new())
    }

    pub fn directory(&self) -> &PathBuf {
        &self.directory
    }

    fn path_for(&self, id: &str) -> PathBuf {
        self.directory.join(format!("{id}.json"))
    }

    fn can_restore_inner(
        &self,
        operation: &OperationRecord,
        visited: &mut HashSet<String>,
    ) -> bool {
        if !visited.insert(operation.id.clone())
            || !matches!(
                operation.status,
                OperationStatus::Succeeded | OperationStatus::SucceededUnverified
            )
        {
            return false;
        }
        let has_local = !operation.actions.is_empty();
        let local = operation
            .actions
            .iter()
            .all(|action| action.before.is_recoverable());
        let has_children = !operation.child_operation_ids.is_empty();
        let children = operation.child_operation_ids.iter().all(|id| {
            self.load(id)
                .is_ok_and(|child| self.can_restore_inner(&child, visited))
        });
        (has_local || has_children) && (!has_local || local) && (!has_children || children)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unavailable_and_appx_snapshots_are_not_recoverable() {
        assert!(!ActionState::Unavailable {
            reason: "test".to_string()
        }
        .is_recoverable());
        assert!(!ActionState::AppxPackages {
            package_pattern: "test".to_string(),
            provisioned: false,
            package_names: Vec::new(),
        }
        .is_recoverable());
    }

    #[test]
    fn registry_snapshot_is_recoverable_even_when_value_was_absent() {
        assert!(ActionState::Registry {
            value_type: None,
            value: None,
        }
        .is_recoverable());
    }

    #[test]
    fn parent_is_not_recoverable_when_child_snapshot_is_irreversible() {
        let directory =
            std::env::temp_dir().join(format!("winmole-journal-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&directory).unwrap();
        let journal = OperationJournal {
            directory: directory.clone(),
        };
        let mut child = OperationRecord::new(
            OperationKind::ApplyTweak,
            "child",
            "child",
            false,
            Vec::new(),
        );
        child.status = OperationStatus::SucceededUnverified;
        child.actions.push(OperationAction {
            action: TweakAction::AppxRemove {
                package_pattern: "test".to_string(),
                provisioned: false,
            },
            description: "test".to_string(),
            before: ActionState::AppxPackages {
                package_pattern: "test".to_string(),
                provisioned: false,
                package_names: vec!["test".to_string()],
            },
            after: None,
            result: None,
            rollback_result: None,
        });
        journal.save(&child).unwrap();

        let mut parent = OperationRecord::new(
            OperationKind::ApplyProfile,
            "parent",
            "parent",
            false,
            Vec::new(),
        );
        parent.status = OperationStatus::SucceededUnverified;
        parent.child_operation_ids.push(child.id);
        assert!(!journal.can_restore(&parent));
        std::fs::remove_dir_all(directory).unwrap();
    }
}
