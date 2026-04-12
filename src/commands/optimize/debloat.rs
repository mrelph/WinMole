//! Bloatware Removal (Debloat)
//!
//! Management of Windows AppX packages and bloatware:
//! - Safe apps (no dependencies)
//! - Moderate apps (may have light dependencies)
//! - Protected apps (should not be removed)

use anyhow::Result;
use console::style;
use std::process::Command;

use super::common::{Tweak, TweakAction, TweakCategory, TweakRisk};
use super::TweakRegistry;
use crate::ui::theme::{self, icons};

/// Categories of bloatware apps
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppCategory {
    /// Safe to remove, no system dependencies
    Safe,
    /// May have minor dependencies, use caution
    Moderate,
    /// Protected system apps, do not remove
    Protected,
}

/// Information about an AppX package
#[derive(Debug, Clone)]
pub struct AppxPackage {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub category: AppCategory,
    pub provisioned: bool,
}

/// Get the list of known bloatware apps with their categories
pub fn get_bloatware_list() -> Vec<AppxPackage> {
    vec![
        // =====================================================================
        // SAFE TO REMOVE
        // =====================================================================
        AppxPackage {
            name: "Microsoft.BingWeather".to_string(),
            display_name: "Weather".to_string(),
            description: "Weather app".to_string(),
            category: AppCategory::Safe,
            provisioned: true,
        },
        AppxPackage {
            name: "Microsoft.BingNews".to_string(),
            display_name: "News".to_string(),
            description: "News app".to_string(),
            category: AppCategory::Safe,
            provisioned: true,
        },
        AppxPackage {
            name: "Microsoft.BingFinance".to_string(),
            display_name: "Money".to_string(),
            description: "Finance/Money app".to_string(),
            category: AppCategory::Safe,
            provisioned: true,
        },
        AppxPackage {
            name: "Microsoft.BingSports".to_string(),
            display_name: "Sports".to_string(),
            description: "Sports app".to_string(),
            category: AppCategory::Safe,
            provisioned: true,
        },
        AppxPackage {
            name: "Microsoft.GetHelp".to_string(),
            display_name: "Get Help".to_string(),
            description: "Get Help support app".to_string(),
            category: AppCategory::Safe,
            provisioned: true,
        },
        AppxPackage {
            name: "Microsoft.Getstarted".to_string(),
            display_name: "Tips".to_string(),
            description: "Windows Tips app".to_string(),
            category: AppCategory::Safe,
            provisioned: true,
        },
        AppxPackage {
            name: "Microsoft.MicrosoftOfficeHub".to_string(),
            display_name: "Office".to_string(),
            description: "Office Hub (promotional)".to_string(),
            category: AppCategory::Safe,
            provisioned: true,
        },
        AppxPackage {
            name: "Microsoft.MicrosoftSolitaireCollection".to_string(),
            display_name: "Solitaire".to_string(),
            description: "Solitaire Collection".to_string(),
            category: AppCategory::Safe,
            provisioned: true,
        },
        AppxPackage {
            name: "Microsoft.MixedReality.Portal".to_string(),
            display_name: "Mixed Reality".to_string(),
            description: "Mixed Reality Portal".to_string(),
            category: AppCategory::Safe,
            provisioned: true,
        },
        AppxPackage {
            name: "Microsoft.People".to_string(),
            display_name: "People".to_string(),
            description: "People app".to_string(),
            category: AppCategory::Safe,
            provisioned: true,
        },
        AppxPackage {
            name: "Microsoft.SkypeApp".to_string(),
            display_name: "Skype".to_string(),
            description: "Skype app".to_string(),
            category: AppCategory::Safe,
            provisioned: true,
        },
        AppxPackage {
            name: "Microsoft.WindowsFeedbackHub".to_string(),
            display_name: "Feedback Hub".to_string(),
            description: "Windows Feedback Hub".to_string(),
            category: AppCategory::Safe,
            provisioned: true,
        },
        AppxPackage {
            name: "Microsoft.WindowsMaps".to_string(),
            display_name: "Maps".to_string(),
            description: "Windows Maps".to_string(),
            category: AppCategory::Safe,
            provisioned: true,
        },
        AppxPackage {
            name: "Microsoft.YourPhone".to_string(),
            display_name: "Your Phone".to_string(),
            description: "Phone Link app".to_string(),
            category: AppCategory::Safe,
            provisioned: true,
        },
        AppxPackage {
            name: "Microsoft.ZuneMusic".to_string(),
            display_name: "Groove Music".to_string(),
            description: "Groove Music".to_string(),
            category: AppCategory::Safe,
            provisioned: true,
        },
        AppxPackage {
            name: "Microsoft.ZuneVideo".to_string(),
            display_name: "Movies & TV".to_string(),
            description: "Movies & TV app".to_string(),
            category: AppCategory::Safe,
            provisioned: true,
        },
        AppxPackage {
            name: "Clipchamp.Clipchamp".to_string(),
            display_name: "Clipchamp".to_string(),
            description: "Video editor".to_string(),
            category: AppCategory::Safe,
            provisioned: true,
        },
        AppxPackage {
            name: "Microsoft.Todos".to_string(),
            display_name: "Microsoft To Do".to_string(),
            description: "To-do list app".to_string(),
            category: AppCategory::Safe,
            provisioned: true,
        },
        AppxPackage {
            name: "Microsoft.PowerAutomateDesktop".to_string(),
            display_name: "Power Automate".to_string(),
            description: "Automation app".to_string(),
            category: AppCategory::Safe,
            provisioned: true,
        },
        // Third-party bloatware commonly pre-installed
        AppxPackage {
            name: "SpotifyAB.SpotifyMusic".to_string(),
            display_name: "Spotify".to_string(),
            description: "Spotify (pre-installed)".to_string(),
            category: AppCategory::Safe,
            provisioned: true,
        },
        AppxPackage {
            name: "Disney.37853FC22B2CE".to_string(),
            display_name: "Disney+".to_string(),
            description: "Disney+ (pre-installed)".to_string(),
            category: AppCategory::Safe,
            provisioned: true,
        },
        AppxPackage {
            name: "king.com.CandyCrush*".to_string(),
            display_name: "Candy Crush".to_string(),
            description: "Candy Crush games".to_string(),
            category: AppCategory::Safe,
            provisioned: true,
        },

        // =====================================================================
        // MODERATE - Use Caution
        // =====================================================================
        AppxPackage {
            name: "Microsoft.Xbox.TCUI".to_string(),
            display_name: "Xbox TCUI".to_string(),
            description: "Xbox UI components (may affect gaming)".to_string(),
            category: AppCategory::Moderate,
            provisioned: true,
        },
        AppxPackage {
            name: "Microsoft.XboxApp".to_string(),
            display_name: "Xbox".to_string(),
            description: "Xbox app (Game Bar integration)".to_string(),
            category: AppCategory::Moderate,
            provisioned: true,
        },
        AppxPackage {
            name: "Microsoft.XboxGameOverlay".to_string(),
            display_name: "Xbox Game Bar".to_string(),
            description: "Xbox Game Bar overlay".to_string(),
            category: AppCategory::Moderate,
            provisioned: true,
        },
        AppxPackage {
            name: "Microsoft.XboxGamingOverlay".to_string(),
            display_name: "Xbox Gaming Overlay".to_string(),
            description: "Xbox Gaming Overlay".to_string(),
            category: AppCategory::Moderate,
            provisioned: true,
        },
        AppxPackage {
            name: "Microsoft.XboxIdentityProvider".to_string(),
            display_name: "Xbox Identity".to_string(),
            description: "Xbox sign-in services".to_string(),
            category: AppCategory::Moderate,
            provisioned: true,
        },
        AppxPackage {
            name: "Microsoft.XboxSpeechToTextOverlay".to_string(),
            display_name: "Xbox Speech".to_string(),
            description: "Xbox speech services".to_string(),
            category: AppCategory::Moderate,
            provisioned: true,
        },
        AppxPackage {
            name: "Microsoft.Windows.Cortana".to_string(),
            display_name: "Cortana".to_string(),
            description: "Cortana assistant".to_string(),
            category: AppCategory::Moderate,
            provisioned: true,
        },
        AppxPackage {
            name: "Microsoft.OneDrive".to_string(),
            display_name: "OneDrive".to_string(),
            description: "OneDrive (system integration)".to_string(),
            category: AppCategory::Moderate,
            provisioned: false,
        },

        // =====================================================================
        // PROTECTED - Do Not Remove
        // =====================================================================
        AppxPackage {
            name: "Microsoft.WindowsStore".to_string(),
            display_name: "Microsoft Store".to_string(),
            description: "Required for app installation".to_string(),
            category: AppCategory::Protected,
            provisioned: true,
        },
        AppxPackage {
            name: "Microsoft.WindowsCalculator".to_string(),
            display_name: "Calculator".to_string(),
            description: "System calculator".to_string(),
            category: AppCategory::Protected,
            provisioned: true,
        },
        AppxPackage {
            name: "Microsoft.Windows.Photos".to_string(),
            display_name: "Photos".to_string(),
            description: "Default photo viewer".to_string(),
            category: AppCategory::Protected,
            provisioned: true,
        },
        AppxPackage {
            name: "Microsoft.WindowsTerminal".to_string(),
            display_name: "Windows Terminal".to_string(),
            description: "Modern terminal".to_string(),
            category: AppCategory::Protected,
            provisioned: true,
        },
        AppxPackage {
            name: "Microsoft.WindowsNotepad".to_string(),
            display_name: "Notepad".to_string(),
            description: "System notepad".to_string(),
            category: AppCategory::Protected,
            provisioned: true,
        },
        AppxPackage {
            name: "Microsoft.Paint".to_string(),
            display_name: "Paint".to_string(),
            description: "MS Paint".to_string(),
            category: AppCategory::Protected,
            provisioned: true,
        },
        AppxPackage {
            name: "Microsoft.ScreenSketch".to_string(),
            display_name: "Snipping Tool".to_string(),
            description: "Screenshot tool".to_string(),
            category: AppCategory::Protected,
            provisioned: true,
        },
    ]
}

/// Register debloat tweaks for individual app removal
pub fn register_tweaks(registry: &mut TweakRegistry) {
    // Register individual app removal tweaks
    for app in get_bloatware_list() {
        if app.category == AppCategory::Protected {
            continue; // Don't register protected apps
        }

        let risk = match app.category {
            AppCategory::Safe => TweakRisk::Safe,
            AppCategory::Moderate => TweakRisk::Moderate,
            AppCategory::Protected => continue,
        };

        registry.register(Tweak {
            id: format!("debloat_{}", app.name.to_lowercase().replace('.', "_").replace('*', "")),
            name: format!("Remove {}", app.display_name),
            description: app.description.clone(),
            category: TweakCategory::Debloat,
            risk,
            requires_admin: app.provisioned,
            requires_restart: false,
            apply_actions: vec![TweakAction::AppxRemove {
                package_pattern: app.name.clone(),
                provisioned: app.provisioned,
            }],
            revert_actions: vec![], // Can't reinstall automatically
            tags: vec!["debloat".to_string(), format!("{:?}", app.category).to_lowercase()],
        });
    }

    // Register bulk removal tweaks
    registry.register(Tweak {
        id: "debloat_all_safe".to_string(),
        name: "Remove All Safe Bloatware".to_string(),
        description: "Remove all apps marked as safe to remove".to_string(),
        category: TweakCategory::Debloat,
        risk: TweakRisk::Safe,
        requires_admin: true,
        requires_restart: false,
        apply_actions: get_bloatware_list()
            .into_iter()
            .filter(|a| a.category == AppCategory::Safe)
            .map(|a| TweakAction::AppxRemove {
                package_pattern: a.name,
                provisioned: a.provisioned,
            })
            .collect(),
        revert_actions: vec![],
        tags: vec!["debloat".to_string(), "bulk".to_string()],
    });
}

/// List installed AppX packages
#[cfg(windows)]
pub fn list_installed_apps() -> Result<Vec<(String, String)>> {
    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            "Get-AppxPackage | Select-Object Name, PackageFullName | ConvertTo-Json",
        ])
        .output()?;

    if !output.status.success() {
        return Ok(vec![]);
    }

    let json_str = String::from_utf8_lossy(&output.stdout);

    // Parse JSON response
    if let Ok(packages) = serde_json::from_str::<Vec<serde_json::Value>>(&json_str) {
        Ok(packages
            .into_iter()
            .filter_map(|p| {
                let name = p.get("Name")?.as_str()?.to_string();
                let full_name = p.get("PackageFullName")?.as_str()?.to_string();
                Some((name, full_name))
            })
            .collect())
    } else {
        // Try single object (when only one package)
        if let Ok(p) = serde_json::from_str::<serde_json::Value>(&json_str) {
            if let (Some(name), Some(full_name)) = (
                p.get("Name").and_then(|v| v.as_str()),
                p.get("PackageFullName").and_then(|v| v.as_str()),
            ) {
                return Ok(vec![(name.to_string(), full_name.to_string())]);
            }
        }
        Ok(vec![])
    }
}

#[cfg(not(windows))]
pub fn list_installed_apps() -> Result<Vec<(String, String)>> {
    Ok(vec![])
}

/// Remove an AppX package by name pattern
#[cfg(windows)]
pub fn remove_app(name_pattern: &str, dry_run: bool) -> Result<bool> {
    if dry_run {
        println!(
            "  {} Would remove: {}",
            style(icons::INFO).cyan(),
            name_pattern
        );
        return Ok(true);
    }

    let safe_pattern = name_pattern.replace('\'', "''");
    let script = format!(
        "Get-AppxPackage -AllUsers | Where-Object {{ $_.Name -like '*{}*' }} | Remove-AppxPackage -AllUsers -ErrorAction SilentlyContinue",
        safe_pattern
    );

    let output = Command::new("powershell")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &script])
        .output()?;

    Ok(output.status.success())
}

#[cfg(not(windows))]
pub fn remove_app(_name_pattern: &str, _dry_run: bool) -> Result<bool> {
    Ok(false)
}

/// Remove a provisioned AppX package
#[cfg(windows)]
pub fn remove_provisioned_app(name_pattern: &str, dry_run: bool) -> Result<bool> {
    if dry_run {
        println!(
            "  {} Would remove provisioned: {}",
            style(icons::INFO).cyan(),
            name_pattern
        );
        return Ok(true);
    }

    let safe_pattern = name_pattern.replace('\'', "''");
    let script = format!(
        "Get-AppxProvisionedPackage -Online | Where-Object {{ $_.PackageName -like '*{}*' }} | Remove-AppxProvisionedPackage -Online -ErrorAction SilentlyContinue",
        safe_pattern
    );

    let output = Command::new("powershell")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &script])
        .output()?;

    Ok(output.status.success())
}

#[cfg(not(windows))]
pub fn remove_provisioned_app(_name_pattern: &str, _dry_run: bool) -> Result<bool> {
    Ok(false)
}

/// Run the debloat command
pub fn run(action: &str, category: Option<&str>, dry_run: bool) -> Result<()> {
    match action {
        "list" => list_apps(category),
        "scan" => scan_bloatware(),
        "remove" => {
            if let Some(app_name) = category {
                remove_single_app(app_name, dry_run)
            } else {
                Err(anyhow::anyhow!("Please specify an app to remove"))
            }
        }
        "remove-safe" => remove_safe_apps(dry_run),
        _ => Err(anyhow::anyhow!("Unknown action: {}", action)),
    }
}

/// List all installed apps
fn list_apps(category_filter: Option<&str>) -> Result<()> {
    theme::print_section_header("Installed AppX Packages");

    let installed = list_installed_apps()?;
    let bloatware = get_bloatware_list();

    for (name, _full_name) in &installed {
        let app_info = bloatware.iter().find(|a| name.contains(&a.name.replace('*', "")));

        let (category, display_name) = if let Some(info) = app_info {
            (Some(info.category), info.display_name.as_str())
        } else {
            (None, name.as_str())
        };

        // Filter by category if specified
        if let Some(filter) = category_filter {
            match filter {
                "safe" if category != Some(AppCategory::Safe) => continue,
                "moderate" if category != Some(AppCategory::Moderate) => continue,
                "protected" if category != Some(AppCategory::Protected) => continue,
                _ => {}
            }
        }

        let category_style = match category {
            Some(AppCategory::Safe) => style("[Safe]").green(),
            Some(AppCategory::Moderate) => style("[Moderate]").yellow(),
            Some(AppCategory::Protected) => style("[Protected]").red(),
            None => style("[Unknown]").dim(),
        };

        println!("  {} {} {}", category_style, display_name, style(name).dim());
    }

    println!();
    println!("  {} Total: {} packages", style(icons::INFO).cyan(), installed.len());
    println!();

    Ok(())
}

/// Scan for removable bloatware
fn scan_bloatware() -> Result<()> {
    theme::print_section_header("Scanning for Bloatware");

    let installed = list_installed_apps()?;
    let bloatware = get_bloatware_list();

    let mut safe_count = 0;
    let mut moderate_count = 0;

    println!("  {} Safe to Remove:", style(icons::SUCCESS).green());
    for (name, _) in &installed {
        for app in &bloatware {
            if app.category == AppCategory::Safe && name.contains(&app.name.replace('*', "")) {
                println!("    {} {} - {}", style(icons::BULLET).green(), app.display_name, app.description);
                safe_count += 1;
            }
        }
    }

    println!();
    println!("  {} Moderate (Use Caution):", style(icons::WARNING).yellow());
    for (name, _) in &installed {
        for app in &bloatware {
            if app.category == AppCategory::Moderate && name.contains(&app.name.replace('*', "")) {
                println!("    {} {} - {}", style(icons::BULLET).yellow(), app.display_name, app.description);
                moderate_count += 1;
            }
        }
    }

    println!();
    theme::print_result_summary(
        "BLOATWARE SCAN COMPLETE",
        &[
            ("Safe to Remove", safe_count.to_string()),
            ("Moderate Risk", moderate_count.to_string()),
        ],
        &[
            "Use 'winmole debloat remove-safe' to remove all safe apps",
            "Use 'winmole debloat remove <app>' to remove specific apps",
        ],
    );

    Ok(())
}

/// Remove a single app
fn remove_single_app(app_name: &str, dry_run: bool) -> Result<()> {
    let bloatware = get_bloatware_list();

    // Find the app
    let app = bloatware.iter().find(|a| {
        a.name.to_lowercase().contains(&app_name.to_lowercase())
            || a.display_name.to_lowercase().contains(&app_name.to_lowercase())
    });

    if let Some(app) = app {
        if app.category == AppCategory::Protected {
            theme::print_error(&format!("{} is a protected system app and cannot be removed", app.display_name));
            return Ok(());
        }

        if dry_run {
            println!("  {} Would remove: {} ({})", style(icons::INFO).cyan(), app.display_name, app.name);
        } else {
            print!("  {} Removing {}... ", style(icons::PROGRESS).cyan(), app.display_name);

            let success = if app.provisioned {
                remove_provisioned_app(&app.name, false)? && remove_app(&app.name, false)?
            } else {
                remove_app(&app.name, false)?
            };

            if success {
                println!("{}", style("OK").green());
            } else {
                println!("{}", style("FAILED").red());
            }
        }
    } else {
        // Try to remove by raw name
        if dry_run {
            println!("  {} Would remove: {}", style(icons::INFO).cyan(), app_name);
        } else {
            print!("  {} Removing {}... ", style(icons::PROGRESS).cyan(), app_name);
            let success = remove_app(app_name, false)?;
            if success {
                println!("{}", style("OK").green());
            } else {
                println!("{}", style("FAILED").red());
            }
        }
    }

    Ok(())
}

/// Remove all safe apps
fn remove_safe_apps(dry_run: bool) -> Result<()> {
    theme::print_section_header("Removing Safe Bloatware");

    if dry_run {
        println!("  {} Running in dry-run mode (no changes will be made)", style(icons::INFO).cyan());
        println!();
    }

    let installed = list_installed_apps()?;
    let bloatware = get_bloatware_list();

    let mut removed = 0;
    let mut failed = 0;

    for (name, _) in &installed {
        for app in &bloatware {
            if app.category == AppCategory::Safe && name.contains(&app.name.replace('*', "")) {
                if dry_run {
                    println!("  {} Would remove: {}", style(icons::BULLET).cyan(), app.display_name);
                    removed += 1;
                } else {
                    print!("  {} Removing {}... ", style(icons::PROGRESS).cyan(), app.display_name);

                    let success = if app.provisioned {
                        remove_provisioned_app(&app.name, false).unwrap_or(false)
                            && remove_app(&app.name, false).unwrap_or(false)
                    } else {
                        remove_app(&app.name, false).unwrap_or(false)
                    };

                    if success {
                        println!("{}", style("OK").green());
                        removed += 1;
                    } else {
                        println!("{}", style("FAILED").red());
                        failed += 1;
                    }
                }
            }
        }
    }

    println!();
    theme::print_result_summary(
        "DEBLOAT COMPLETE",
        &[
            ("Removed", removed.to_string()),
            ("Failed", failed.to_string()),
        ],
        &["Some apps may reappear after Windows updates"],
    );

    Ok(())
}
