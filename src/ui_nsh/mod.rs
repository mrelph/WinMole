//! The Native Shell (nsh) desktop UI for WinMole — `winmole ui`.
//!
//! A single window with a sidebar of the seven modules (Dashboard, Cleanup,
//! Disk, Startup, Packages, Registry, Optimize), a top bar carrying a
//! language toggle (Simplified Chinese / English, default follows the OS), and
//! the module's screen. Scanning and applying run on worker threads that send
//! events back through an `mpsc` channel drained by an effect, so the engine
//! thread never blocks on file/registry/winget work.

pub mod i18n;
pub mod screens;
pub use screens::WinMoleApp;
pub mod run;

use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant};

use nsh_core::prelude::*;
use sysinfo::{Disks, System};

use crate::commands;
use crate::commands::optimize::common::{Tweak, TweakRisk, TweakState};
use crate::commands::optimize::{TweakExecutor, TweakRegistry};
use crate::commands::processes::ResidualProcess;
use crate::commands::registry::AuditFinding;
use crate::commands::startup::StartupItemInfo;
use crate::system::cleanup::CleanupTarget;
use crate::ui_nsh::i18n::{Lang, T};

/// The seven top-level modules, mirroring the ratatui dashboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleId {
    Dashboard,
    Cleanup,
    Disk,
    Startup,
    Packages,
    Registry,
    Optimize,
}

impl ModuleId {
    pub const ALL: [Self; 7] = [
        Self::Dashboard,
        Self::Cleanup,
        Self::Disk,
        Self::Startup,
        Self::Packages,
        Self::Registry,
        Self::Optimize,
    ];
}

/// A row's per-item outcome after an apply step.
#[derive(Debug, Clone, PartialEq)]
pub enum ItemResult {
    Pending,
    Ok(String),
    Err(String),
}

/// A cleanup target with a staged checkbox and an apply outcome.
#[derive(Debug, Clone, PartialEq)]
pub struct CleanupRow {
    pub target: CleanupTarget,
    pub checked: bool,
    pub result: ItemResult,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResidualRow {
    pub process: ResidualProcess,
    pub checked: bool,
    pub result: ItemResult,
}

#[derive(Clone, PartialEq)]
pub enum PendingAction {
    Optimize(Vec<(usize, Tweak, bool)>),
    Processes(Vec<ResidualProcess>),
}

/// A startup item with a staged enabled/disabled toggle.
#[derive(Debug, Clone, PartialEq)]
pub struct StartupRow {
    pub item: StartupItemInfo,
    pub staged: bool,
    pub result: ItemResult,
}

/// An upgradable package with a staged checkbox.
#[derive(Debug, Clone, PartialEq)]
pub struct PackageRow {
    pub name: String,
    pub id: String,
    pub current: String,
    pub available: String,
    pub checked: bool,
    pub result: ItemResult,
}

/// A registry finding with a staged checkbox.
#[derive(Debug, Clone, PartialEq)]
pub struct RegistryRow {
    pub finding: AuditFinding,
    pub checked: bool,
    pub result: ItemResult,
}

/// An optimization tweak with a staged desired state.
#[derive(Debug, Clone, PartialEq)]
pub struct OptimizeRow {
    pub tweak: Tweak,
    pub state: TweakState,
    pub desired: bool,
    pub result: ItemResult,
}

/// The state shared by every screen, owned by the root component. Shared by
/// all the components via an `Rc` (the worker receiver is not `Clone`, and
/// the reactive signals are `Copy` handles).
pub struct Ctx {
    pub active: Signal<ModuleId>,
    pub lang: Signal<Lang>,
    pub confirm_open: Signal<bool>,
    pub confirm_input: Signal<String>,
    /// The optimize or process changes awaiting typed confirmation.
    pub pending_action: Signal<Option<PendingAction>>,
    pub notice: Signal<Option<(String, bool)>>,
    notice_deadline: Signal<Option<Instant>>,

    pub stats: Signal<LiveStats>,
    pub health: Signal<Option<crate::system::HealthScore>>,

    pub cleanup_phase: Signal<Phase>,
    pub cleanup_rows: Signal<Vec<CleanupRow>>,
    pub cleanup_summary: Signal<String>,
    pub cleanup_error: Signal<String>,

    pub residual_phase: Signal<Phase>,
    pub residual_rows: Signal<Vec<ResidualRow>>,
    pub residual_summary: Signal<String>,
    pub residual_error: Signal<String>,

    pub disk_loading: Signal<bool>,
    pub disk_path: Signal<PathBuf>,
    pub disk_rows: Signal<Vec<(PathBuf, u64)>>,

    pub startup_phase: Signal<Phase>,
    pub startup_rows: Signal<Vec<StartupRow>>,
    pub startup_summary: Signal<String>,

    pub packages_phase: Signal<Phase>,
    pub packages_rows: Signal<Vec<PackageRow>>,
    pub packages_summary: Signal<String>,
    pub packages_error: Signal<String>,

    pub registry_phase: Signal<Phase>,
    pub registry_rows: Signal<Vec<RegistryRow>>,
    pub registry_summary: Signal<String>,
    pub registry_error: Signal<String>,

    pub optimize_phase: Signal<Phase>,
    pub optimize_rows: Signal<Vec<OptimizeRow>>,
    pub optimize_summary: Signal<String>,

    // Startup lazily-scanned modules and the durable error text.
    pub disk_started: Signal<bool>,
    pub packages_started: Signal<bool>,
    pub registry_started: Signal<bool>,
    pub optimize_started: Signal<bool>,

    pub worker_tx: Sender<WorkerEvent>,
    pub worker_rx: Receiver<WorkerEvent>,
}

/// Scan/apply progress of a module.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Scanning,
    Select,
    Applying,
    Done,
    Error,
}

/// A snapshot of the live system gauges on the dashboard.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LiveStats {
    pub cpu: f32,
    pub memory_used: u64,
    pub memory_total: u64,
    pub disk_free: u64,
    pub disk_total: u64,
    pub disk_name: String,
    pub uptime: u64,
}

/// Messages a worker thread sends back to the engine thread.
#[derive(Debug)]
pub enum WorkerEvent {
    CleanupScanned(Result<Vec<CleanupTarget>, String>),
    ResidualScanned(Result<Vec<ResidualProcess>, String>),
    ResidualProgress { index: usize, detail: Result<String, String> },
    ResidualComplete { summary: String },
    DiskScanned { path: PathBuf, rows: Vec<(PathBuf, u64)> },
    StartupScanned(Vec<StartupItemInfo>),
    PackagesScanned(Result<Vec<(String, String, String, String)>, String>),
    RegistryScanned(Result<Vec<AuditFinding>, String>),
    OptimizeScanned(Vec<OptimizeRow>),
    ActionProgress { module: ModuleId, index: usize, detail: Result<String, String> },
    ActionComplete { module: ModuleId, summary: String },
}

impl Ctx {
    /// Make a new context and start the initial scans (cleanup + startup).
    pub fn new() -> Self {
        let (worker_tx, worker_rx) = mpsc::channel();
        let lang = i18n::lang();
        lang.set(i18n::initial_lang());
        let ctx = Self {
            active: signal(ModuleId::Dashboard),
            lang,
            confirm_open: signal(false),
            confirm_input: signal(String::new()),
            pending_action: signal(None),
            notice: signal(None),
            notice_deadline: signal(None),
            stats: signal(LiveStats::default()),
            health: signal(None),
            cleanup_phase: signal(Phase::Scanning),
            cleanup_rows: signal(Vec::new()),
            cleanup_summary: signal(String::new()),
            cleanup_error: signal(String::new()),
            residual_phase: signal(Phase::Scanning),
            residual_rows: signal(Vec::new()),
            residual_summary: signal(String::new()),
            residual_error: signal(String::new()),
            disk_loading: signal(true),
            disk_path: signal(dirs::home_dir().unwrap_or_else(|| PathBuf::from("C:\\"))),
            disk_rows: signal(Vec::new()),
            startup_phase: signal(Phase::Scanning),
            startup_rows: signal(Vec::new()),
            startup_summary: signal(String::new()),
            packages_phase: signal(Phase::Scanning),
            packages_rows: signal(Vec::new()),
            packages_summary: signal(String::new()),
            packages_error: signal(String::new()),
            registry_phase: signal(Phase::Scanning),
            registry_rows: signal(Vec::new()),
            registry_summary: signal(String::new()),
            registry_error: signal(String::new()),
            optimize_phase: signal(Phase::Scanning),
            optimize_rows: signal(Vec::new()),
            optimize_summary: signal(String::new()),
            disk_started: signal(false),
            packages_started: signal(false),
            registry_started: signal(false),
            optimize_started: signal(false),
            worker_tx,
            worker_rx,
        };
        ctx.refresh_stats();
        ctx.scan_cleanup();
        ctx.scan_residual_processes();
        ctx.scan_startup();
        ctx
    }

    fn worker(&self) -> Sender<WorkerEvent> {
        self.worker_tx.clone()
    }

    /// Switch the active module and lazily start its first scan.
    pub fn set_active(&self, module: ModuleId) {
        self.active.set(module);
        self.notice.set(None);
        self.notice_deadline.set(None);
        self.ensure_scanned();
    }

    /// Flip the UI language between Chinese and English.
    pub fn toggle_lang(&self) {
        let next = match self.lang.get() {
            Lang::Zh => Lang::En,
            Lang::En => Lang::Zh,
        };
        self.lang.set(next);
    }

    /// Close the risky-confirm dialog without applying anything.
    pub fn confirm_cancel(&self) {
        self.pending_action.set(None);
        self.confirm_input.set(String::new());
        self.confirm_open.set(false);
    }

    pub fn set_notice(&self, message: impl Into<SharedString>, is_error: bool) {
        self.notice.set(Some((message.into().to_string(), is_error)));
        self.notice_deadline
            .set(Some(Instant::now() + Duration::from_secs(5)));
    }

    pub fn expire_notice(&self) {
        if self
            .notice_deadline
            .get()
            .is_some_and(|deadline| Instant::now() >= deadline)
        {
            self.notice.set(None);
            self.notice_deadline.set(None);
        }
    }

    pub fn set_confirm_input(&self, value: String) {
        self.confirm_input.set(value);
    }

    /// Apply the stored risky changes if the typed confirmation matches.
    pub fn confirm_try_apply_typed(&self, typed: &str) {
        if typed.trim().eq_ignore_ascii_case("APPLY") {
            self.confirm_apply();
        } else {
            self.set_notice(i18n::t(T::RISKY_CONFIRM), true);
        }
    }

    // ---- Live dashboard stats -------------------------------------------------

    pub fn refresh_stats(&self) {
        let mut system = System::new_all();
        system.refresh_all();
        let disks = Disks::new_with_refreshed_list();
        let mut stats = LiveStats {
            cpu: system.global_cpu_usage(),
            memory_used: system.used_memory(),
            memory_total: system.total_memory(),
            uptime: System::uptime(),
            ..LiveStats::default()
        };
        if let Some(disk) = disks
            .iter()
            .find(|d| d.mount_point().to_string_lossy().starts_with("C:"))
            .or_else(|| disks.iter().next())
        {
            stats.disk_total = disk.total_space();
            stats.disk_free = disk.available_space();
            stats.disk_name = disk.mount_point().to_string_lossy().to_string();
        }
        self.stats.set(stats);
        if let Ok(health) = crate::system::get_health_score() {
            self.health.set(Some(health));
        }
    }

    // ---- Scans ---------------------------------------------------------------

    pub fn scan_cleanup(&self) {
        self.cleanup_phase.set(Phase::Scanning);
        let tx = self.worker();
        std::thread::spawn(move || {
            let categories = vec!["all".to_string()];
            let result = commands::clean::scan_targets(&categories).map_err(|e| e.to_string());
            let _ = tx.send(WorkerEvent::CleanupScanned(result));
        });
    }

    pub fn scan_residual_processes(&self) {
        self.residual_phase.set(Phase::Scanning);
        let tx = self.worker();
        std::thread::spawn(move || {
            let result = commands::processes::scan_residual_processes().map_err(|e| e.to_string());
            let _ = tx.send(WorkerEvent::ResidualScanned(result));
        });
    }

    pub fn scan_disk(&self) {
        self.disk_loading.set(true);
        let tx = self.worker();
        std::thread::spawn(move || {
            let path = dirs::home_dir().unwrap_or_else(|| PathBuf::from("C:\\"));
            let rows = commands::disk::scan_largest_folders(&path, 16);
            let _ = tx.send(WorkerEvent::DiskScanned { path, rows });
        });
    }

    pub fn scan_startup(&self) {
        self.startup_phase.set(Phase::Scanning);
        let tx = self.worker();
        std::thread::spawn(move || {
            let _ = tx.send(WorkerEvent::StartupScanned(commands::startup::get_startup_items()));
        });
    }

    pub fn scan_packages(&self) {
        self.packages_phase.set(Phase::Scanning);
        let tx = self.worker();
        std::thread::spawn(move || {
            let result =
                commands::winget::scan_upgradable_packages().map_err(|e| e.to_string());
            let _ = tx.send(WorkerEvent::PackagesScanned(result));
        });
    }

    pub fn scan_registry(&self) {
        self.registry_phase.set(Phase::Scanning);
        let tx = self.worker();
        std::thread::spawn(move || {
            let categories = vec![
                "invalid_paths".to_string(),
                "missing_dlls".to_string(),
                "orphaned_software".to_string(),
            ];
            let result =
                commands::registry::scan_findings(&categories).map_err(|e| e.to_string());
            let _ = tx.send(WorkerEvent::RegistryScanned(result));
        });
    }

    pub fn scan_optimize(&self) {
        self.optimize_phase.set(Phase::Scanning);
        let tx = self.worker();
        std::thread::spawn(move || {
            let registry = TweakRegistry::new();
            let executor = TweakExecutor::new(false);
            let rows = registry
                .all()
                .cloned()
                .map(|tweak| {
                    let state = executor.detect_state(&tweak).unwrap_or(TweakState::Unknown);
                    OptimizeRow {
                        desired: state == TweakState::Applied,
                        tweak,
                        state,
                        result: ItemResult::Pending,
                    }
                })
                .collect();
            let _ = tx.send(WorkerEvent::OptimizeScanned(rows));
        });
    }

    /// Scan the module when it is first opened, or always on an explicit
    /// rescan.
    pub fn scan_active(&self, force: bool) {
        match self.active.get() {
            ModuleId::Dashboard => {
                self.refresh_stats();
                self.scan_cleanup();
                self.scan_startup();
            }
            ModuleId::Cleanup => {
                self.scan_cleanup();
                self.scan_residual_processes();
            }
            ModuleId::Disk => {
                self.disk_started.set(true);
                self.scan_disk();
            }
            ModuleId::Startup => self.scan_startup(),
            ModuleId::Packages => {
                self.packages_started.set(true);
                self.scan_packages();
            }
            ModuleId::Registry => {
                self.registry_started.set(true);
                self.scan_registry();
            }
            ModuleId::Optimize => {
                self.optimize_started.set(true);
                self.scan_optimize();
            }
        }
        let _ = force;
    }

    /// Lazily kick off scans for modules not yet opened.
    pub fn ensure_scanned(&self) {
        if self.active.get() == ModuleId::Disk && !self.disk_started.get() {
            self.disk_started.set(true);
            self.scan_disk();
        }
        if self.active.get() == ModuleId::Packages && !self.packages_started.get() {
            self.packages_started.set(true);
            self.scan_packages();
        }
        if self.active.get() == ModuleId::Registry && !self.registry_started.get() {
            self.registry_started.set(true);
            self.scan_registry();
        }
        if self.active.get() == ModuleId::Optimize && !self.optimize_started.get() {
            self.optimize_started.set(true);
            self.scan_optimize();
        }
    }

    // ---- Selection -----------------------------------------------------------

    /// Toggle the selected row's staged state. `index` is the row the list
    /// reports; the dashboard-style cursor is not retained here, so screens
    /// pass the tapped row index.
    pub fn toggle_row(&self, index: usize) {
        match self.active.get() {
            ModuleId::Cleanup => {
                self.cleanup_rows.update(|rows| {
                    if let Some(row) = rows.get_mut(index) {
                        row.checked = !row.checked;
                    }
                });
            }
            ModuleId::Startup => {
                self.startup_rows.update(|rows| {
                    if let Some(row) = rows.get_mut(index) {
                        if self.startup_phase.get() == Phase::Select {
                            row.staged = !row.staged;
                            row.result = ItemResult::Pending;
                        }
                    }
                });
            }
            ModuleId::Packages => {
                self.packages_rows.update(|rows| {
                    if let Some(row) = rows.get_mut(index) {
                        row.checked = !row.checked;
                    }
                });
            }
            ModuleId::Registry => {
                self.registry_rows.update(|rows| {
                    if let Some(row) = rows.get_mut(index) {
                        if row.finding.remediable {
                            row.checked = !row.checked;
                        }
                    }
                });
            }
            ModuleId::Optimize => {
                self.optimize_rows.update(|rows| {
                    if let Some(row) = rows.get_mut(index) {
                        if self.optimize_phase.get() == Phase::Select {
                            row.desired = !row.desired;
                        }
                    }
                });
            }
            _ => {}
        }
    }

    pub fn toggle_residual_row(&self, index: usize) {
        if self.residual_phase.get() != Phase::Select {
            return;
        }
        self.residual_rows.update(|rows| {
            if let Some(row) = rows.get_mut(index) {
                row.checked = !row.checked;
            }
        });
    }

    pub fn select_all_safe(&self) {
        let module = self.active.get();
        match module {
            ModuleId::Cleanup => {
                self.cleanup_rows
                    .update(|rows| for r in rows { r.checked = !r.target.requires_admin; });
            }
            ModuleId::Packages => {
                self.packages_rows.update(|rows| for r in rows { r.checked = true; });
            }
            ModuleId::Registry => {
                self.registry_rows
                    .update(|rows| for r in rows { r.checked = r.finding.remediable; });
            }
            ModuleId::Optimize => {
                self.optimize_rows.update(|rows| {
                    for r in rows {
                        if r.tweak.risk == TweakRisk::Safe {
                            r.desired = true;
                        }
                    }
                });
            }
            _ => {}
        }
    }

    // ---- Apply actions (run on worker threads) ------------------------------

    pub fn commit_active(&self) {
        match self.active.get() {
            ModuleId::Cleanup => self.apply_cleanup(),
            ModuleId::Startup => self.apply_startup(),
            ModuleId::Packages => self.apply_packages(),
            ModuleId::Registry => self.apply_registry(),
            ModuleId::Optimize => self.apply_optimize(),
            _ => {}
        }
    }

    fn apply_cleanup(&self) {
        if self.cleanup_phase.get() != Phase::Select {
            return;
        }
        let selected: Vec<(usize, CleanupTarget)> = self
            .cleanup_rows
            .get()
            .into_iter()
            .enumerate()
            .filter(|(_, r)| r.checked)
            .map(|(i, r)| (i, r.target.clone()))
            .collect();
        if selected.is_empty() {
            self.set_notice(i18n::t(T::SELECT_AT_LEAST_ONE), true);
            return;
        }
        self.cleanup_phase.set(Phase::Applying);
        let lang = i18n::lang().get();
        let tx = self.worker();
        std::thread::spawn(move || {
            let mut freed = 0u64;
            let mut files = 0u64;
            let mut errors = 0u64;
            for (index, target) in selected {
                let result = commands::clean::clean_target(&target).map_err(|e| e.to_string());
                match &result {
                    Ok((size, count)) => {
                        freed += size;
                        files += *count;
                    }
                    Err(_) => errors += 1,
                }
                let detail = result.map(|(size, _)| commands::format_size(size));
                let _ = tx.send(WorkerEvent::ActionProgress {
                    module: ModuleId::Cleanup,
                    index,
                    detail,
                });
            }
            let _ = tx.send(WorkerEvent::ActionComplete {
                module: ModuleId::Cleanup,
                summary: format!(
                    "{} · {} {} · {} {}",
                    i18n::pick(lang, T::RECLAIMS),
                    commands::format_size(freed),
                    i18n::pick(lang, T::ITEMS),
                    files,
                    i18n::pick(lang, T::ERROR)
                ),
            });
            let _ = errors;
        });
    }

    pub fn stage_residual_termination(&self) {
        if self.residual_phase.get() != Phase::Select {
            return;
        }
        let selected: Vec<ResidualProcess> = self
            .residual_rows
            .get()
            .into_iter()
            .filter(|row| row.checked)
            .map(|row| row.process)
            .collect();
        if selected.is_empty() {
            self.set_notice(i18n::t(T::SELECT_RESIDUAL_PROCESS), true);
            return;
        }
        self.pending_action.set(Some(PendingAction::Processes(selected)));
        self.confirm_input.set(String::new());
        self.confirm_open.set(true);
    }

    fn run_residual_processes(&self, selected: Vec<ResidualProcess>) {
        self.residual_phase.set(Phase::Applying);
        let total = selected.len();
        let lang = i18n::lang().get();
        let tx = self.worker();
        std::thread::spawn(move || {
            let mut terminated = 0usize;
            for (index, process) in selected.into_iter().enumerate() {
                let detail = match commands::processes::terminate_residual_process(&process) {
                    Ok(true) => {
                        terminated += 1;
                        Ok(i18n::pick(lang, T::OK).to_string())
                    }
                    Ok(false) => Ok(i18n::pick(lang, T::PROCESS_SKIPPED).to_string()),
                    Err(error) => Err(error.to_string()),
                };
                let _ = tx.send(WorkerEvent::ResidualProgress { index, detail });
            }
            let summary = format!(
                "{}: {terminated}/{total}",
                i18n::pick(lang, T::RESIDUAL_COMPLETE)
            );
            let _ = tx.send(WorkerEvent::ResidualComplete { summary });
        });
    }

    fn apply_startup(&self) {
        if self.startup_phase.get() != Phase::Select {
            return;
        }
        let changes: Vec<(usize, String, bool)> = self
            .startup_rows
            .get()
            .into_iter()
            .enumerate()
            .filter(|(_, r)| r.item.enabled != r.staged)
            .map(|(i, r)| (i, r.item.name.clone(), r.staged))
            .collect();
        if changes.is_empty() {
            self.set_notice(i18n::t(T::NO_STARTUP_CHANGES), false);
            return;
        }
        self.startup_phase.set(Phase::Applying);
        let lang = i18n::lang().get();
        let tx = self.worker();
        std::thread::spawn(move || {
            let mut succeeded = 0usize;
            let mut failed = 0usize;
            for (index, name, enabled) in changes {
                let result = commands::startup::set_item_enabled(&name, enabled)
                    .map(|_| i18n::pick(lang, T::OK).to_string())
                    .map_err(|e| e.to_string());
                if result.is_ok() {
                    succeeded += 1;
                } else {
                    failed += 1;
                }
                let _ = tx.send(WorkerEvent::ActionProgress {
                    module: ModuleId::Startup,
                    index,
                    detail: result,
                });
            }
            let summary = i18n::pick(lang, T::STARTUP_RESULT)
                .replace("{ok}", &succeeded.to_string())
                .replace("{failed}", &failed.to_string());
            let _ = tx.send(WorkerEvent::ActionComplete {
                module: ModuleId::Startup,
                summary,
            });
        });
    }

    fn apply_packages(&self) {
        if self.packages_phase.get() != Phase::Select {
            return;
        }
        let selected: Vec<(usize, String)> = self
            .packages_rows
            .get()
            .into_iter()
            .enumerate()
            .filter(|(_, r)| r.checked)
            .map(|(i, r)| (i, r.id.clone()))
            .collect();
        if selected.is_empty() {
            self.set_notice(i18n::t(T::SELECT_AT_LEAST_ONE_PKG), true);
            return;
        }
        self.packages_phase.set(Phase::Applying);
        let lang = i18n::lang().get();
        let tx = self.worker();
        std::thread::spawn(move || {
            for (index, id) in selected {
                let result = commands::winget::upgrade_package_quiet(&id)
                    .map(|_| i18n::pick(lang, T::OK).to_string())
                    .map_err(|e| e.to_string());
                let _ = tx.send(WorkerEvent::ActionProgress {
                    module: ModuleId::Packages,
                    index,
                    detail: result,
                });
            }
            let _ = tx.send(WorkerEvent::ActionComplete {
                module: ModuleId::Packages,
                summary: i18n::pick(lang, T::PACKAGES_COMPLETE).to_string(),
            });
        });
    }

    fn apply_registry(&self) {
        if self.registry_phase.get() != Phase::Select {
            return;
        }
        let selected: Vec<(usize, AuditFinding)> = self
            .registry_rows
            .get()
            .into_iter()
            .enumerate()
            .filter(|(_, r)| r.checked && r.finding.remediable)
            .map(|(i, r)| (i, r.finding.clone()))
            .collect();
        if selected.is_empty() {
            self.set_notice(i18n::t(T::SELECT_REMEDIABLE), true);
            return;
        }
        self.registry_phase.set(Phase::Applying);
        let lang = i18n::lang().get();
        let tx = self.worker();
        std::thread::spawn(move || {
            for (index, finding) in selected {
                let result = commands::registry::remediate_finding(&finding, false)
                    .and_then(|r| {
                        if r.success {
                            Ok(i18n::pick(lang, T::OK).to_string())
                        } else {
                            anyhow::bail!(r.error.unwrap_or_else(|| "remediation failed".into()))
                        }
                    })
                    .map_err(|e| e.to_string());
                let _ = tx.send(WorkerEvent::ActionProgress {
                    module: ModuleId::Registry,
                    index,
                    detail: result,
                });
            }
            let _ = tx.send(WorkerEvent::ActionComplete {
                module: ModuleId::Registry,
                summary: i18n::pick(lang, T::REGISTRY_COMPLETE).to_string(),
            });
        });
    }

    fn apply_optimize(&self) {
        if self.optimize_phase.get() != Phase::Select {
            return;
        }
        let changes: Vec<(usize, Tweak, bool)> = self
            .optimize_rows
            .get()
            .into_iter()
            .enumerate()
            .filter(|(_, r)| r.desired != (r.state == TweakState::Applied))
            .map(|(i, r)| (i, r.tweak.clone(), r.desired))
            .collect();
        if changes.is_empty() {
            self.set_notice(i18n::t(T::NO_OPTIMIZE_CHANGES), false);
            return;
        }
        let risky = changes.iter().any(|(_, tweak, _)| {
            matches!(tweak.risk, TweakRisk::Risky | TweakRisk::Dangerous)
        });
        if risky {
            self.pending_action
                .set(Some(PendingAction::Optimize(changes)));
            self.confirm_open.set(true);
            self.confirm_input.set(String::new());
            return;
        }
        self.run_optimize(changes);
    }

    fn run_optimize(&self, changes: Vec<(usize, Tweak, bool)>) {
        self.optimize_phase.set(Phase::Applying);
        let lang = i18n::lang().get();
        let tx = self.worker();
        std::thread::spawn(move || {
            let executor = TweakExecutor::new(false);
            for (index, tweak, desired) in changes {
                let result = if desired {
                    executor.apply(&tweak)
                } else {
                    executor.revert(&tweak)
                };
                let detail = match result {
                    Ok(result) if result.success => {
                        if desired {
                            commands::optimize::record_tweak_applied(&tweak.id, &result);
                        } else {
                            commands::optimize::record_tweak_reverted(&tweak.id);
                        }
                        Ok(i18n::pick(lang, T::OK).to_string())
                    }
                    Ok(result) => Err(result
                        .error
                        .unwrap_or_else(|| "change was not verified".to_string())),
                    Err(e) => Err(e.to_string()),
                };
                let _ = tx.send(WorkerEvent::ActionProgress {
                    module: ModuleId::Optimize,
                    index,
                    detail,
                });
            }
            let _ = tx.send(WorkerEvent::ActionComplete {
                module: ModuleId::Optimize,
                summary: i18n::pick(lang, T::OPTIMIZE_COMPLETE).to_string(),
            });
        });
    }

    /// The confirmation dialog was accepted: run the stored risky changes.
    pub fn confirm_apply(&self) {
        let action = self.pending_action.get();
        self.pending_action.set(None);
        self.confirm_open.set(false);
        match action {
            Some(PendingAction::Optimize(changes)) => self.run_optimize(changes),
            Some(PendingAction::Processes(processes)) => self.run_residual_processes(processes),
            None => {}
        }
    }
}


/// Drain pending worker events into the signals. Called from a repeating
/// effect in the root component.
pub fn drain(ctx: &Ctx) {
    ctx.expire_notice();
    while let Ok(event) = ctx.worker_rx.try_recv() {
        match event {
            WorkerEvent::CleanupScanned(Ok(targets)) => {
                ctx.cleanup_rows.set(
                    targets
                        .into_iter()
                        .map(|target| CleanupRow {
                            checked: !target.requires_admin,
                            target,
                            result: ItemResult::Pending,
                        })
                        .collect(),
                );
                ctx.cleanup_phase.set(Phase::Select);
            }
            WorkerEvent::CleanupScanned(Err(e)) => {
                ctx.cleanup_error.set(e);
                ctx.cleanup_phase.set(Phase::Error);
            }
            WorkerEvent::ResidualScanned(Ok(processes)) => {
                ctx.residual_rows.set(
                    processes
                        .into_iter()
                        .map(|process| ResidualRow {
                            process,
                            checked: false,
                            result: ItemResult::Pending,
                        })
                        .collect(),
                );
                ctx.residual_phase.set(Phase::Select);
            }
            WorkerEvent::ResidualScanned(Err(e)) => {
                ctx.residual_error.set(e);
                ctx.residual_phase.set(Phase::Error);
            }
            WorkerEvent::ResidualProgress { index, detail } => {
                ctx.residual_rows.update(|rows| {
                    if let Some(row) = rows.get_mut(index) {
                        row.result = match detail {
                            Ok(message) => ItemResult::Ok(message),
                            Err(message) => ItemResult::Err(message),
                        };
                    }
                });
            }
            WorkerEvent::ResidualComplete { summary } => {
                ctx.residual_phase.set(Phase::Done);
                ctx.residual_summary.set(summary.clone());
                ctx.set_notice(summary, false);
            }
            WorkerEvent::DiskScanned { path, rows } => {
                ctx.disk_path.set(path);
                ctx.disk_rows.set(rows);
                ctx.disk_loading.set(false);
            }
            WorkerEvent::StartupScanned(items) => {
                ctx.startup_rows.set(
                    items
                        .into_iter()
                        .map(|item| StartupRow {
                            staged: item.enabled,
                            item,
                            result: ItemResult::Pending,
                        })
                        .collect(),
                );
                ctx.startup_phase.set(Phase::Select);
            }
            WorkerEvent::PackagesScanned(Ok(packages)) => {
                ctx.packages_rows.set(
                    packages
                        .into_iter()
                        .map(|(name, id, current, available)| PackageRow {
                            name,
                            id,
                            current,
                            available,
                            checked: false,
                            result: ItemResult::Pending,
                        })
                        .collect(),
                );
                ctx.packages_phase.set(Phase::Select);
            }
            WorkerEvent::PackagesScanned(Err(e)) => {
                ctx.packages_error.set(e);
                ctx.packages_phase.set(Phase::Error);
            }
            WorkerEvent::RegistryScanned(Ok(findings)) => {
                ctx.registry_rows.set(
                    findings
                        .into_iter()
                        .map(|finding| RegistryRow {
                            finding,
                            checked: false,
                            result: ItemResult::Pending,
                        })
                        .collect(),
                );
                ctx.registry_phase.set(Phase::Select);
            }
            WorkerEvent::RegistryScanned(Err(e)) => {
                ctx.registry_error.set(e);
                ctx.registry_phase.set(Phase::Error);
            }
            WorkerEvent::OptimizeScanned(rows) => {
                ctx.optimize_rows.set(rows);
                ctx.optimize_phase.set(Phase::Select);
            }
            WorkerEvent::ActionProgress {
                module,
                index,
                detail,
            } => ctx.record_progress(module, index, detail),
            WorkerEvent::ActionComplete { module, summary } => ctx.finish_action(module, summary),
        }
    }
}

impl Ctx {
    fn record_progress(&self, module: ModuleId, index: usize, detail: Result<String, String>) {
        let ok = detail.is_ok();
        let result = match detail {
            Ok(text) => ItemResult::Ok(text),
            Err(text) => ItemResult::Err(text),
        };
        match module {
            ModuleId::Cleanup => {
                self.cleanup_rows.update(|rows| {
                    if let Some(row) = rows.get_mut(index) {
                        row.result = result.clone();
                    }
                });
            }
            ModuleId::Startup => {
                self.startup_rows.update(|rows| {
                    if let Some(row) = rows.get_mut(index) {
                        if ok {
                            row.item.enabled = row.staged;
                        }
                        row.result = result.clone();
                    }
                });
            }
            ModuleId::Packages => {
                self.packages_rows.update(|rows| {
                    if let Some(row) = rows.get_mut(index) {
                        row.result = result.clone();
                    }
                });
            }
            ModuleId::Registry => {
                self.registry_rows.update(|rows| {
                    if let Some(row) = rows.get_mut(index) {
                        row.result = result.clone();
                    }
                });
            }
            ModuleId::Optimize => {
                self.optimize_rows.update(|rows| {
                    if let Some(row) = rows.get_mut(index) {
                        if ok {
                            row.state = if row.desired {
                                TweakState::Applied
                            } else {
                                TweakState::NotApplied
                            };
                        }
                        row.result = result.clone();
                    }
                });
            }
            _ => {}
        }
    }

    fn finish_action(&self, module: ModuleId, summary: String) {
        match module {
            ModuleId::Cleanup => {
                self.cleanup_phase.set(Phase::Done);
                self.cleanup_summary.set(summary.clone());
            }
            ModuleId::Startup => {
                self.startup_phase.set(Phase::Select);
                self.startup_summary.set(summary.clone());
            }
            ModuleId::Packages => {
                self.packages_phase.set(Phase::Done);
                self.packages_summary.set(summary.clone());
            }
            ModuleId::Registry => {
                self.registry_phase.set(Phase::Done);
                self.registry_summary.set(summary.clone());
            }
            ModuleId::Optimize => {
                self.optimize_phase.set(Phase::Select);
                self.optimize_summary.set(summary.clone());
            }
            _ => {}
        }
        self.set_notice(summary, false);
    }
}

/// Total reclaimable bytes across cleanup rows.
pub fn cleanable_total(ctx: &Ctx) -> u64 {
    ctx.cleanup_rows
        .get()
        .iter()
        .map(|r| r.target.size)
        .sum()
}

/// Format a duration of seconds as `Nd hh:mm`.
pub fn format_uptime(seconds: u64) -> String {
    let days = seconds / 86_400;
    let hours = (seconds % 86_400) / 3_600;
    let minutes = (seconds % 3_600) / 60;
    format!("{days}d {hours:02}:{minutes:02}")
}

/// A small helper to keep a worker thread from being cleaned while the app
/// runs (the threads are detached and owned by the process, so this is only
/// a marker). The engine thread never blocks on them.
#[allow(dead_code)]
fn _keep_alive(_: &Duration) {}