//! Structured Windows configuration audit with narrowly scoped remediation.

use anyhow::{anyhow, bail, Result};
use serde::Serialize;
#[cfg(windows)]
use sha2::{Digest, Sha256};

#[cfg(windows)]
use crate::commands::optimize::common::RegistryHive;
use crate::commands::optimize::common::{Tweak, TweakAction, TweakCategory, TweakRisk};
use crate::commands::optimize::{record_tweak_applied, TweakExecutor};
use crate::operations::OperationKind;
use crate::ui::theme;

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditSeverity {
    #[cfg(windows)]
    Warning,
}

#[derive(Debug, Clone, Serialize)]
pub struct AuditFinding {
    pub id: String,
    pub category: String,
    pub severity: AuditSeverity,
    pub name: String,
    pub location: String,
    pub evidence: String,
    pub remediable: bool,
    #[serde(skip)]
    remediation: Option<TweakAction>,
}

pub fn run(
    mode: &str,
    categories: &[String],
    finding_id: Option<&str>,
    dry_run: bool,
    json: bool,
) -> Result<()> {
    if !matches!(mode, "scan" | "audit" | "remediate") {
        bail!("Registry mode must be one of: audit, scan, remediate");
    }

    let findings = scan_findings(categories)?;

    if mode == "remediate" {
        let finding_id =
            finding_id.ok_or_else(|| anyhow!("--finding is required for remediation"))?;
        return remediate(&findings, finding_id, dry_run, json);
    }

    print_findings(&findings, json)
}

/// Return structured configuration findings without printing.
pub fn scan_findings(categories: &[String]) -> Result<Vec<AuditFinding>> {
    #[cfg(not(windows))]
    {
        let _ = categories;
        Ok(Vec::new())
    }
    #[cfg(windows)]
    {
        scan_windows(categories)
    }
}

fn print_findings(findings: &[AuditFinding], json: bool) -> Result<()> {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "audit": "windows_configuration",
                "findings": findings,
                "summary": {
                    "total": findings.len(),
                    "remediable": findings.iter().filter(|finding| finding.remediable).count(),
                }
            }))?
        );
        return Ok(());
    }

    theme::print_section_header("Windows Configuration Audit");
    if findings.is_empty() {
        theme::print_success("No configuration findings were detected.");
        return Ok(());
    }

    for finding in findings {
        let remediation = if finding.remediable {
            "remediation available"
        } else {
            "review only"
        };
        println!(
            "  [{}] {} ({}, {})",
            finding.id, finding.name, finding.category, remediation
        );
        println!("    Evidence: {}", finding.evidence);
        println!("    Location: {}", finding.location);
    }
    println!();
    println!(
        "  {} finding(s), {} with exact-value remediation",
        findings.len(),
        findings.iter().filter(|finding| finding.remediable).count()
    );
    Ok(())
}

fn remediate(findings: &[AuditFinding], finding_id: &str, dry_run: bool, json: bool) -> Result<()> {
    let finding = findings
        .iter()
        .find(|finding| finding.id == finding_id)
        .ok_or_else(|| anyhow!("Audit finding '{}' was not found", finding_id))?;
    let result = remediate_finding(finding, dry_run)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else if result.success {
        let operation = result.operation_id.as_deref().unwrap_or("unavailable");
        println!(
            "  {} {} (operation {})",
            if dry_run {
                "Would remediate"
            } else {
                "Remediated"
            },
            finding.name,
            operation
        );
    } else {
        bail!(
            "Remediation failed: {}",
            result.error.as_deref().unwrap_or("unknown error")
        );
    }
    Ok(())
}

/// Apply one exact-value audit remediation without printing.
pub fn remediate_finding(
    finding: &AuditFinding,
    dry_run: bool,
) -> Result<crate::commands::optimize::common::TweakResult> {
    let action = finding.remediation.clone().ok_or_else(|| {
        anyhow!(
            "Finding '{}' is review-only and has no safe automatic remediation",
            finding.id
        )
    })?;
    let tweak = Tweak {
        id: format!("audit_{}", finding.id),
        name: format!("Remediate {}", finding.name),
        description: finding.evidence.clone(),
        category: TweakCategory::Hardware,
        risk: TweakRisk::Moderate,
        requires_admin: action.requires_admin(),
        requires_restart: false,
        apply_actions: vec![action],
        revert_actions: Vec::new(),
        tags: vec!["audit".to_string()],
    };
    let result = TweakExecutor::new(dry_run).apply_as(&tweak, OperationKind::AuditRemediation)?;
    if result.success && !dry_run {
        record_tweak_applied(&tweak.id, &result);
    }
    Ok(result)
}

#[cfg(windows)]
fn scan_windows(categories: &[String]) -> Result<Vec<AuditFinding>> {
    let mut findings = Vec::new();
    for category in categories {
        match category.as_str() {
            "invalid_paths" | "invalidpaths" => findings.extend(scan_invalid_paths()?),
            "missing_dlls" | "missingdlls" => findings.extend(scan_missing_dlls()?),
            "orphaned_software" | "orphanedsoftware" => findings.extend(scan_orphaned_software()?),
            _ => bail!("Unknown audit category: {}", category),
        }
    }
    Ok(findings)
}

#[cfg(windows)]
fn scan_invalid_paths() -> Result<Vec<AuditFinding>> {
    use winreg::enums::HKEY_LOCAL_MACHINE;
    use winreg::RegKey;

    let base = "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\App Paths";
    let app_paths = match RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(base) {
        Ok(key) => key,
        Err(_) => return Ok(Vec::new()),
    };
    let mut findings = Vec::new();
    for key_name in app_paths.enum_keys().filter_map(Result::ok) {
        let Ok(subkey) = app_paths.open_subkey(&key_name) else {
            continue;
        };
        let Ok(raw_path) = subkey.get_value::<String, _>("") else {
            continue;
        };
        let expanded = expand_env_vars(&raw_path);
        if expanded.is_empty() || std::path::Path::new(&expanded).exists() {
            continue;
        }
        let path = format!("{base}\\{key_name}");
        findings.push(finding(
            "invalid_path",
            "Invalid application path",
            &key_name,
            RegistryHive::Hklm,
            &path,
            "",
            format!("Referenced executable does not exist: {expanded}"),
            true,
        ));
    }
    Ok(findings)
}

#[cfg(windows)]
fn scan_missing_dlls() -> Result<Vec<AuditFinding>> {
    use winreg::enums::HKEY_LOCAL_MACHINE;
    use winreg::RegKey;

    let path = "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\SharedDLLs";
    let key = match RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(path) {
        Ok(key) => key,
        Err(_) => return Ok(Vec::new()),
    };
    let mut findings = Vec::new();
    for (name, _) in key.enum_values().filter_map(Result::ok).take(500) {
        let expanded = expand_env_vars(&name);
        if std::path::Path::new(&expanded).exists() {
            continue;
        }
        let display = std::path::Path::new(&name)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        findings.push(finding(
            "missing_dll",
            "Missing shared DLL",
            &display,
            RegistryHive::Hklm,
            path,
            &name,
            format!("Referenced DLL does not exist: {expanded}"),
            true,
        ));
    }
    Ok(findings)
}

#[cfg(windows)]
fn scan_orphaned_software() -> Result<Vec<AuditFinding>> {
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
    use winreg::RegKey;

    let locations = [
        (
            HKEY_LOCAL_MACHINE,
            RegistryHive::Hklm,
            "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall",
        ),
        (
            HKEY_LOCAL_MACHINE,
            RegistryHive::Hklm,
            "SOFTWARE\\WOW6432Node\\Microsoft\\Windows\\CurrentVersion\\Uninstall",
        ),
        (
            HKEY_CURRENT_USER,
            RegistryHive::Hkcu,
            "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall",
        ),
    ];
    let mut findings = Vec::new();
    for (root, hive, path) in locations {
        let Ok(uninstall) = RegKey::predef(root).open_subkey(path) else {
            continue;
        };
        for key_name in uninstall.enum_keys().filter_map(Result::ok).take(200) {
            let Ok(subkey) = uninstall.open_subkey(&key_name) else {
                continue;
            };
            let Ok(raw_location) = subkey.get_value::<String, _>("InstallLocation") else {
                continue;
            };
            let expanded = expand_env_vars(&raw_location);
            if expanded.is_empty() || std::path::Path::new(&expanded).exists() {
                continue;
            }
            let name = subkey
                .get_value::<String, _>("DisplayName")
                .unwrap_or_else(|_| key_name.clone());
            let key_path = format!("{path}\\{key_name}");
            findings.push(finding(
                "orphaned_software",
                "Orphaned software registration",
                &name,
                hive.clone(),
                &key_path,
                "InstallLocation",
                format!("Recorded install location does not exist: {expanded}"),
                false,
            ));
        }
    }
    Ok(findings)
}

#[cfg(windows)]
#[allow(clippy::too_many_arguments)]
fn finding(
    category: &str,
    label: &str,
    name: &str,
    hive: RegistryHive,
    path: &str,
    value_name: &str,
    evidence: String,
    remediable: bool,
) -> AuditFinding {
    let location = format!("{hive}\\{path}\\{value_name}");
    let digest = Sha256::digest(location.as_bytes());
    let short_hash = digest[..6]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let id = format!("{category}-{short_hash}");
    AuditFinding {
        id,
        category: label.to_string(),
        severity: AuditSeverity::Warning,
        name: name.to_string(),
        location,
        evidence,
        remediable,
        remediation: remediable.then(|| TweakAction::RegistryDelete {
            hive,
            path: path.to_string(),
            name: value_name.to_string(),
        }),
    }
}

#[cfg(windows)]
fn expand_env_vars(value: &str) -> String {
    let mut expanded = value.to_string();
    for variable in [
        "SystemRoot",
        "ProgramFiles",
        "ProgramFiles(x86)",
        "USERPROFILE",
    ] {
        if let Ok(replacement) = std::env::var(variable) {
            expanded = expanded.replace(&format!("%{variable}%"), &replacement);
            expanded = expanded.replace(
                &format!("%{}%", variable.to_ascii_lowercase()),
                &replacement,
            );
        }
    }
    expanded.trim_matches('"').to_string()
}
