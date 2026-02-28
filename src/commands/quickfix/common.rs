use std::fmt;
use std::time::Duration;

// ============================================================================
// ISSUE CATEGORIES
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IssueCategory {
    Storage,
    DiskHealth,
    Maintenance,
}

impl fmt::Display for IssueCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IssueCategory::Storage => write!(f, "Storage"),
            IssueCategory::DiskHealth => write!(f, "Disk Health"),
            IssueCategory::Maintenance => write!(f, "Maintenance"),
        }
    }
}

// ============================================================================
// ISSUE SEVERITY
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[allow(dead_code)]
pub enum IssueSeverity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

impl fmt::Display for IssueSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IssueSeverity::Info => write!(f, "Info"),
            IssueSeverity::Low => write!(f, "Low"),
            IssueSeverity::Medium => write!(f, "Medium"),
            IssueSeverity::High => write!(f, "High"),
            IssueSeverity::Critical => write!(f, "Critical"),
        }
    }
}

// ============================================================================
// FIX RISK LEVELS
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum FixRisk {
    Safe,
    Moderate,
    Risky,
}

impl fmt::Display for FixRisk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FixRisk::Safe => write!(f, "Safe"),
            FixRisk::Moderate => write!(f, "Moderate"),
            FixRisk::Risky => write!(f, "Risky"),
        }
    }
}

// ============================================================================
// FIX ACTIONS
// ============================================================================

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum FixAction {
    CleanDirectory {
        path: String,
        description: String,
        requires_admin: bool,
    },
    PowerShellCommand {
        script: String,
        description: String,
        requires_admin: bool,
    },
    SystemCommand {
        command: String,
        args: Vec<String>,
        description: String,
        requires_admin: bool,
    },
    DeleteFile {
        path: String,
        description: String,
        requires_admin: bool,
    },
}

impl FixAction {
    #[allow(dead_code)]
    pub fn requires_admin(&self) -> bool {
        match self {
            FixAction::CleanDirectory { requires_admin, .. } => *requires_admin,
            FixAction::PowerShellCommand { requires_admin, .. } => *requires_admin,
            FixAction::SystemCommand { requires_admin, .. } => *requires_admin,
            FixAction::DeleteFile { requires_admin, .. } => *requires_admin,
        }
    }

    pub fn description(&self) -> &str {
        match self {
            FixAction::CleanDirectory { description, .. } => description,
            FixAction::PowerShellCommand { description, .. } => description,
            FixAction::SystemCommand { description, .. } => description,
            FixAction::DeleteFile { description, .. } => description,
        }
    }
}

// ============================================================================
// ISSUE
// ============================================================================

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Issue {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: IssueCategory,
    pub severity: IssueSeverity,
    pub fix_risk: FixRisk,
    pub estimated_savings: Option<u64>,
    pub requires_admin: bool,
    pub fix_actions: Vec<FixAction>,
    pub auto_fixable: bool,
}

// ============================================================================
// RESULTS
// ============================================================================

pub struct ScanResult {
    pub issues: Vec<Issue>,
    pub scan_duration: Duration,
}

#[allow(dead_code)]
pub struct FixResult {
    pub issue_id: String,
    pub success: bool,
    pub space_freed: Option<u64>,
    pub error: Option<String>,
}
