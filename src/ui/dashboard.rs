use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Margin, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, BorderType, Borders, Clear, Gauge, List, ListItem, ListState, Paragraph, Wrap,
};
use ratatui::{DefaultTerminal, Frame};
use sysinfo::{Disks, System};

use crate::commands;
use crate::commands::optimize::common::{Tweak, TweakRisk, TweakState};
use crate::commands::optimize::{TweakExecutor, TweakRegistry};
use crate::commands::registry::AuditFinding;
use crate::commands::startup::StartupItemInfo;
use crate::system::cleanup::CleanupTarget;

const ACCENT: Color = Color::Cyan;
const SELECTED: Color = Color::LightCyan;
const TEXT: Color = Color::White;
const MUTED: Color = Color::DarkGray;
const SUCCESS: Color = Color::Green;
const WARNING: Color = Color::Yellow;
const DANGER: Color = Color::Red;
const SPINNERS: [&str; 4] = ["◐", "◓", "◑", "◒"];

pub fn run() -> Result<()> {
    let mut terminal = ratatui::init();
    let result = run_loop(&mut terminal);
    ratatui::restore();
    result
}

fn run_loop(terminal: &mut DefaultTerminal) -> Result<()> {
    let mut app = App::new();
    while !app.should_quit {
        app.drain_worker_events();
        app.refresh_live_stats();
        app.expire_notice();
        terminal.draw(|frame| render(frame, &app))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    app.handle_key(key);
                }
            }
        }
        app.tick = app.tick.wrapping_add(1);
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ModuleId {
    Dashboard,
    Cleanup,
    Disk,
    Startup,
    Packages,
    Registry,
    Optimize,
}

impl ModuleId {
    const ALL: [Self; 7] = [
        Self::Dashboard,
        Self::Cleanup,
        Self::Disk,
        Self::Startup,
        Self::Packages,
        Self::Registry,
        Self::Optimize,
    ];

    fn number(self) -> usize {
        Self::ALL
            .iter()
            .position(|module| *module == self)
            .unwrap_or_default()
            + 1
    }

    fn label(self) -> &'static str {
        match self {
            Self::Dashboard => "Dashboard",
            Self::Cleanup => "Cleanup",
            Self::Disk => "Disk",
            Self::Startup => "Startup",
            Self::Packages => "Packages",
            Self::Registry => "Registry",
            Self::Optimize => "Optimize",
        }
    }

    fn from_digit(digit: char) -> Option<Self> {
        digit
            .to_digit(10)
            .and_then(|value| Self::ALL.get(value.saturating_sub(1) as usize))
            .copied()
    }
}

struct App {
    active: ModuleId,
    should_quit: bool,
    system: System,
    disks: Disks,
    stats: LiveStats,
    cleanup: CleanupState,
    disk: DiskState,
    startup: StartupState,
    packages: PackageState,
    registry: RegistryState,
    optimize: OptimizeState,
    palette: Option<PaletteState>,
    confirm: Option<Confirmation>,
    notice: Option<Notice>,
    worker_tx: Sender<WorkerEvent>,
    worker_rx: Receiver<WorkerEvent>,
    last_stats_refresh: Instant,
    last_health_refresh: Instant,
    tick: usize,
}

impl App {
    fn new() -> Self {
        let (worker_tx, worker_rx) = mpsc::channel();
        let mut app = Self {
            active: ModuleId::Dashboard,
            should_quit: false,
            system: System::new_all(),
            disks: Disks::new_with_refreshed_list(),
            stats: LiveStats::default(),
            cleanup: CleanupState::default(),
            disk: DiskState::default(),
            startup: StartupState::default(),
            packages: PackageState::default(),
            registry: RegistryState::default(),
            optimize: OptimizeState::default(),
            palette: None,
            confirm: None,
            notice: None,
            worker_tx,
            worker_rx,
            last_stats_refresh: Instant::now() - Duration::from_secs(2),
            last_health_refresh: Instant::now() - Duration::from_secs(31),
            tick: 0,
        };
        app.refresh_live_stats();
        app.start_dashboard_scans();
        app
    }

    fn start_dashboard_scans(&self) {
        self.scan_cleanup();
        self.scan_startup();
    }

    fn scan_cleanup(&self) {
        let tx = self.worker_tx.clone();
        thread::spawn(move || {
            let categories = vec!["all".to_string()];
            let result = commands::clean::scan_targets(&categories).map_err(|e| e.to_string());
            let _ = tx.send(WorkerEvent::CleanupScanned(result));
        });
    }

    fn scan_disk(&self) {
        let tx = self.worker_tx.clone();
        thread::spawn(move || {
            let path = dirs::home_dir().unwrap_or_else(|| PathBuf::from("C:\\"));
            let rows = commands::disk::scan_largest_folders(&path, 14);
            let _ = tx.send(WorkerEvent::DiskScanned { path, rows });
        });
    }

    fn scan_startup(&self) {
        let tx = self.worker_tx.clone();
        thread::spawn(move || {
            let _ = tx.send(WorkerEvent::StartupScanned(
                commands::startup::get_startup_items(),
            ));
        });
    }

    fn scan_packages(&self) {
        let tx = self.worker_tx.clone();
        thread::spawn(move || {
            let result = commands::winget::scan_upgradable_packages().map_err(|e| e.to_string());
            let _ = tx.send(WorkerEvent::PackagesScanned(result));
        });
    }

    fn scan_registry(&self) {
        let tx = self.worker_tx.clone();
        thread::spawn(move || {
            let categories = vec![
                "invalid_paths".to_string(),
                "missing_dlls".to_string(),
                "orphaned_software".to_string(),
            ];
            let result = commands::registry::scan_findings(&categories).map_err(|e| e.to_string());
            let _ = tx.send(WorkerEvent::RegistryScanned(result));
        });
    }

    fn scan_optimize(&self) {
        let tx = self.worker_tx.clone();
        thread::spawn(move || {
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
                        result: None,
                    }
                })
                .collect();
            let _ = tx.send(WorkerEvent::OptimizeScanned(rows));
        });
    }

    fn refresh_live_stats(&mut self) {
        if self.last_stats_refresh.elapsed() < Duration::from_secs(1) {
            return;
        }
        self.system.refresh_all();
        self.disks = Disks::new_with_refreshed_list();

        self.stats.cpu = self.system.global_cpu_usage();
        self.stats.memory_total = self.system.total_memory();
        self.stats.memory_used = self.system.used_memory();
        if let Some(disk) = self
            .disks
            .iter()
            .find(|disk| disk.mount_point().to_string_lossy().starts_with("C:"))
            .or_else(|| self.disks.iter().next())
        {
            self.stats.disk_total = disk.total_space();
            self.stats.disk_free = disk.available_space();
            self.stats.disk_name = disk.mount_point().to_string_lossy().to_string();
        }
        self.stats.uptime = System::uptime();
        if self.last_health_refresh.elapsed() >= Duration::from_secs(30) {
            if let Ok(health) = crate::system::get_health_score() {
                self.stats.health = health.score;
                self.stats.health_status = health.status;
                self.stats.recommendations = health.recommendations;
            }
            self.last_health_refresh = Instant::now();
        }
        self.last_stats_refresh = Instant::now();
    }

    fn handle_key(&mut self, key: KeyEvent) {
        if self.confirm.is_some() {
            self.handle_confirmation_key(key);
            return;
        }
        if self.palette.is_some() {
            self.handle_palette_key(key);
            return;
        }

        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.should_quit = true;
            return;
        }

        match key.code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char(':') => self.palette = Some(PaletteState::default()),
            KeyCode::Char(digit @ '1'..='7') => {
                if let Some(module) = ModuleId::from_digit(digit) {
                    self.activate(module);
                }
            }
            KeyCode::Esc => self.active = ModuleId::Dashboard,
            KeyCode::Char('r') => self.rescan_active(),
            KeyCode::Up => self.move_cursor(-1),
            KeyCode::Down => self.move_cursor(1),
            KeyCode::Char(' ') => self.toggle_current(),
            KeyCode::Char('a') => self.select_all_safe(),
            KeyCode::Enter => self.commit_active(),
            _ => {}
        }
    }

    fn activate(&mut self, module: ModuleId) {
        self.active = module;
        match module {
            ModuleId::Disk if !self.disk.started => {
                self.disk.started = true;
                self.scan_disk();
            }
            ModuleId::Packages if !self.packages.started => {
                self.packages.started = true;
                self.scan_packages();
            }
            ModuleId::Registry if !self.registry.started => {
                self.registry.started = true;
                self.scan_registry();
            }
            ModuleId::Optimize if !self.optimize.started => {
                self.optimize.started = true;
                self.scan_optimize();
            }
            _ => {}
        }
    }

    fn move_cursor(&mut self, delta: isize) {
        match self.active {
            ModuleId::Cleanup => {
                move_index(&mut self.cleanup.cursor, self.cleanup.rows.len(), delta)
            }
            ModuleId::Disk => move_index(&mut self.disk.cursor, self.disk.rows.len(), delta),
            ModuleId::Startup => {
                move_index(&mut self.startup.cursor, self.startup.rows.len(), delta)
            }
            ModuleId::Packages => {
                move_index(&mut self.packages.cursor, self.packages.rows.len(), delta)
            }
            ModuleId::Registry => {
                move_index(&mut self.registry.cursor, self.registry.rows.len(), delta)
            }
            ModuleId::Optimize => {
                move_index(&mut self.optimize.cursor, self.optimize.rows.len(), delta)
            }
            ModuleId::Dashboard => {}
        }
    }

    fn toggle_current(&mut self) {
        match self.active {
            ModuleId::Cleanup if self.cleanup.phase == WorkPhase::Select => {
                if let Some(row) = self.cleanup.rows.get_mut(self.cleanup.cursor) {
                    row.checked = !row.checked;
                }
            }
            ModuleId::Startup if self.startup.phase == WorkPhase::Select => {
                if let Some(row) = self.startup.rows.get_mut(self.startup.cursor) {
                    row.staged = !row.staged;
                }
            }
            ModuleId::Packages if self.packages.phase == WorkPhase::Select => {
                if let Some(row) = self.packages.rows.get_mut(self.packages.cursor) {
                    row.checked = !row.checked;
                }
            }
            ModuleId::Registry if self.registry.phase == WorkPhase::Select => {
                if let Some(row) = self.registry.rows.get_mut(self.registry.cursor) {
                    if row.finding.remediable {
                        row.checked = !row.checked;
                    }
                }
            }
            ModuleId::Optimize if self.optimize.phase == WorkPhase::Select => {
                if let Some(row) = self.optimize.rows.get_mut(self.optimize.cursor) {
                    row.desired = !row.desired;
                }
            }
            _ => {}
        }
    }

    fn select_all_safe(&mut self) {
        match self.active {
            ModuleId::Cleanup => {
                for row in &mut self.cleanup.rows {
                    row.checked = !row.target.requires_admin;
                }
            }
            ModuleId::Packages => {
                for row in &mut self.packages.rows {
                    row.checked = true;
                }
            }
            ModuleId::Registry => {
                for row in &mut self.registry.rows {
                    row.checked = row.finding.remediable;
                }
            }
            ModuleId::Optimize => {
                for row in &mut self.optimize.rows {
                    if row.tweak.risk == TweakRisk::Safe {
                        row.desired = true;
                    }
                }
            }
            _ => {}
        }
    }

    fn commit_active(&mut self) {
        match self.active {
            ModuleId::Cleanup => self.start_cleanup_apply(),
            ModuleId::Startup => self.start_startup_apply(),
            ModuleId::Packages => self.start_package_apply(),
            ModuleId::Registry => self.start_registry_apply(),
            ModuleId::Optimize => {
                let risky = self.optimize.rows.iter().any(|row| {
                    row.desired != (row.state == TweakState::Applied)
                        && matches!(row.tweak.risk, TweakRisk::Risky | TweakRisk::Dangerous)
                });
                if risky {
                    self.confirm = Some(Confirmation {
                        title: "Confirm risky optimization changes".to_string(),
                        message:
                            "These changes can affect Windows behavior. Type APPLY to continue."
                                .to_string(),
                        required: Some("APPLY".to_string()),
                        input: String::new(),
                        action: PendingAction::Optimize,
                    });
                } else {
                    self.start_optimize_apply();
                }
            }
            _ => {}
        }
    }

    fn start_cleanup_apply(&mut self) {
        if self.cleanup.phase == WorkPhase::Done {
            self.cleanup = CleanupState::default();
            self.scan_cleanup();
            return;
        }
        if self.cleanup.phase != WorkPhase::Select {
            return;
        }
        let selected: Vec<_> = self
            .cleanup
            .rows
            .iter()
            .enumerate()
            .filter(|(_, row)| row.checked)
            .map(|(index, row)| (index, row.target.clone()))
            .collect();
        if selected.is_empty() {
            self.set_notice("Select at least one cleanup category", true);
            return;
        }
        self.cleanup.phase = WorkPhase::Applying;
        let tx = self.worker_tx.clone();
        thread::spawn(move || {
            let mut freed = 0;
            let mut files = 0;
            let mut errors = 0;
            for (index, target) in selected {
                let result = commands::clean::clean_target(&target).map_err(|e| e.to_string());
                match &result {
                    Ok((size, count)) => {
                        freed += size;
                        files += count;
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
                    "Reclaimed {} · {} items · {} errors",
                    commands::format_size(freed),
                    files,
                    errors
                ),
            });
        });
    }

    fn start_startup_apply(&mut self) {
        if self.startup.phase != WorkPhase::Select {
            return;
        }
        let changes: Vec<_> = self
            .startup
            .rows
            .iter()
            .enumerate()
            .filter(|(_, row)| row.item.enabled != row.staged)
            .map(|(index, row)| (index, row.item.name.clone(), row.staged))
            .collect();
        if changes.is_empty() {
            self.set_notice("No startup changes are staged", false);
            return;
        }
        self.startup.phase = WorkPhase::Applying;
        let tx = self.worker_tx.clone();
        thread::spawn(move || {
            let mut changed = 0;
            for (index, name, enabled) in changes {
                let result = commands::startup::set_item_enabled(&name, enabled)
                    .map(|_| {
                        changed += 1;
                        if enabled {
                            "enabled".to_string()
                        } else {
                            "disabled".to_string()
                        }
                    })
                    .map_err(|e| e.to_string());
                let _ = tx.send(WorkerEvent::ActionProgress {
                    module: ModuleId::Startup,
                    index,
                    detail: result,
                });
            }
            let _ = tx.send(WorkerEvent::ActionComplete {
                module: ModuleId::Startup,
                summary: format!("Applied {} startup change(s)", changed),
            });
        });
    }

    fn start_package_apply(&mut self) {
        if self.packages.phase != WorkPhase::Select {
            return;
        }
        let selected: Vec<_> = self
            .packages
            .rows
            .iter()
            .enumerate()
            .filter(|(_, row)| row.checked)
            .map(|(index, row)| (index, row.id.clone()))
            .collect();
        if selected.is_empty() {
            self.set_notice("Select at least one package", true);
            return;
        }
        self.packages.phase = WorkPhase::Applying;
        let tx = self.worker_tx.clone();
        thread::spawn(move || {
            let mut updated = 0;
            for (index, id) in selected {
                let result = commands::winget::upgrade_package_quiet(&id)
                    .map(|_| {
                        updated += 1;
                        "updated".to_string()
                    })
                    .map_err(|e| e.to_string());
                let _ = tx.send(WorkerEvent::ActionProgress {
                    module: ModuleId::Packages,
                    index,
                    detail: result,
                });
            }
            let _ = tx.send(WorkerEvent::ActionComplete {
                module: ModuleId::Packages,
                summary: format!("Updated {} package(s)", updated),
            });
        });
    }

    fn start_registry_apply(&mut self) {
        if self.registry.phase != WorkPhase::Select {
            return;
        }
        let selected: Vec<_> = self
            .registry
            .rows
            .iter()
            .enumerate()
            .filter(|(_, row)| row.checked && row.finding.remediable)
            .map(|(index, row)| (index, row.finding.clone()))
            .collect();
        if selected.is_empty() {
            self.set_notice("Select a remediable finding", true);
            return;
        }
        self.registry.phase = WorkPhase::Applying;
        let tx = self.worker_tx.clone();
        thread::spawn(move || {
            let mut remediated = 0;
            for (index, finding) in selected {
                let result = commands::registry::remediate_finding(&finding, false)
                    .and_then(|result| {
                        if result.success {
                            remediated += 1;
                            Ok("remediated".to_string())
                        } else {
                            anyhow::bail!(
                                "{}",
                                result.error.as_deref().unwrap_or("remediation failed")
                            )
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
                summary: format!("Remediated {} finding(s)", remediated),
            });
        });
    }

    fn start_optimize_apply(&mut self) {
        if self.optimize.phase != WorkPhase::Select {
            return;
        }
        let changes: Vec<_> = self
            .optimize
            .rows
            .iter()
            .enumerate()
            .filter(|(_, row)| row.desired != (row.state == TweakState::Applied))
            .map(|(index, row)| (index, row.tweak.clone(), row.desired))
            .collect();
        if changes.is_empty() {
            self.set_notice("No optimization changes are staged", false);
            return;
        }
        self.optimize.phase = WorkPhase::Applying;
        let tx = self.worker_tx.clone();
        thread::spawn(move || {
            let executor = TweakExecutor::new(false);
            let mut changed = 0;
            for (index, tweak, desired) in changes {
                let result = if desired {
                    executor.apply(&tweak)
                } else {
                    executor.revert(&tweak)
                };
                let detail = match result {
                    Ok(result) if result.success => {
                        changed += 1;
                        if desired {
                            commands::optimize::record_tweak_applied(&tweak.id, &result);
                            Ok("applied".to_string())
                        } else {
                            commands::optimize::record_tweak_reverted(&tweak.id);
                            Ok("reverted".to_string())
                        }
                    }
                    Ok(result) => Err(result
                        .error
                        .unwrap_or_else(|| "change was not verified".to_string())),
                    Err(error) => Err(error.to_string()),
                };
                let _ = tx.send(WorkerEvent::ActionProgress {
                    module: ModuleId::Optimize,
                    index,
                    detail,
                });
            }
            let _ = tx.send(WorkerEvent::ActionComplete {
                module: ModuleId::Optimize,
                summary: format!("Applied {} optimization change(s)", changed),
            });
        });
    }

    fn rescan_active(&mut self) {
        match self.active {
            ModuleId::Cleanup => {
                self.cleanup = CleanupState::default();
                self.scan_cleanup();
            }
            ModuleId::Disk => {
                self.disk = DiskState::default();
                self.disk.started = true;
                self.scan_disk();
            }
            ModuleId::Startup => {
                self.startup = StartupState::default();
                self.scan_startup();
            }
            ModuleId::Packages => {
                self.packages = PackageState::default();
                self.packages.started = true;
                self.scan_packages();
            }
            ModuleId::Registry => {
                self.registry = RegistryState::default();
                self.registry.started = true;
                self.scan_registry();
            }
            ModuleId::Optimize => {
                self.optimize = OptimizeState::default();
                self.optimize.started = true;
                self.scan_optimize();
            }
            ModuleId::Dashboard => self.start_dashboard_scans(),
        }
    }

    fn handle_palette_key(&mut self, key: KeyEvent) {
        let Some(palette) = self.palette.as_mut() else {
            return;
        };
        match key.code {
            KeyCode::Esc => self.palette = None,
            KeyCode::Up => {
                let count = palette.filtered().len();
                move_index(&mut palette.cursor, count, -1);
            }
            KeyCode::Down => {
                let count = palette.filtered().len();
                move_index(&mut palette.cursor, count, 1);
            }
            KeyCode::Backspace => {
                palette.query.pop();
                palette.cursor = 0;
            }
            KeyCode::Char(character) => {
                palette.query.push(character);
                palette.cursor = 0;
            }
            KeyCode::Enter => {
                let selected = palette.filtered().get(palette.cursor).copied();
                if let Some(command) = selected {
                    self.palette = None;
                    self.activate(command.module);
                }
            }
            _ => {}
        }
    }

    fn handle_confirmation_key(&mut self, key: KeyEvent) {
        let Some(confirm) = self.confirm.as_mut() else {
            return;
        };
        match key.code {
            KeyCode::Esc => self.confirm = None,
            KeyCode::Backspace => {
                confirm.input.pop();
            }
            KeyCode::Char(character) => confirm.input.push(character),
            KeyCode::Enter => {
                let valid = confirm
                    .required
                    .as_ref()
                    .is_none_or(|required| confirm.input == *required);
                if valid {
                    let action = confirm.action;
                    self.confirm = None;
                    match action {
                        PendingAction::Optimize => self.start_optimize_apply(),
                    }
                }
            }
            _ => {}
        }
    }

    fn drain_worker_events(&mut self) {
        while let Ok(event) = self.worker_rx.try_recv() {
            match event {
                WorkerEvent::CleanupScanned(result) => match result {
                    Ok(targets) => {
                        self.cleanup.rows = targets
                            .into_iter()
                            .map(|target| CleanupRow {
                                checked: !target.requires_admin,
                                target,
                                result: None,
                            })
                            .collect();
                        self.cleanup.phase = WorkPhase::Select;
                    }
                    Err(error) => self.cleanup.fail(error),
                },
                WorkerEvent::DiskScanned { path, rows } => {
                    self.disk.path = path;
                    self.disk.rows = rows;
                    self.disk.loading = false;
                }
                WorkerEvent::StartupScanned(items) => {
                    self.startup.rows = items
                        .into_iter()
                        .map(|item| StartupRow {
                            staged: item.enabled,
                            item,
                            result: None,
                        })
                        .collect();
                    self.startup.phase = WorkPhase::Select;
                }
                WorkerEvent::PackagesScanned(result) => match result {
                    Ok(packages) => {
                        self.packages.rows = packages
                            .into_iter()
                            .map(|(name, id, current, available)| PackageRow {
                                name,
                                id,
                                current,
                                available,
                                checked: false,
                                result: None,
                            })
                            .collect();
                        self.packages.phase = WorkPhase::Select;
                    }
                    Err(error) => self.packages.fail(error),
                },
                WorkerEvent::RegistryScanned(result) => match result {
                    Ok(findings) => {
                        self.registry.rows = findings
                            .into_iter()
                            .map(|finding| RegistryRow {
                                finding,
                                checked: false,
                                result: None,
                            })
                            .collect();
                        self.registry.phase = WorkPhase::Select;
                    }
                    Err(error) => self.registry.fail(error),
                },
                WorkerEvent::OptimizeScanned(rows) => {
                    self.optimize.rows = rows;
                    self.optimize.phase = WorkPhase::Select;
                }
                WorkerEvent::ActionProgress {
                    module,
                    index,
                    detail,
                } => self.record_action_progress(module, index, detail),
                WorkerEvent::ActionComplete { module, summary } => {
                    self.finish_action(module, summary)
                }
            }
        }
    }

    fn record_action_progress(
        &mut self,
        module: ModuleId,
        index: usize,
        detail: std::result::Result<String, String>,
    ) {
        match module {
            ModuleId::Cleanup => {
                if let Some(row) = self.cleanup.rows.get_mut(index) {
                    row.result = Some(detail);
                }
            }
            ModuleId::Startup => {
                if let Some(row) = self.startup.rows.get_mut(index) {
                    if detail.is_ok() {
                        row.item.enabled = row.staged;
                    }
                    row.result = Some(detail);
                }
            }
            ModuleId::Packages => {
                if let Some(row) = self.packages.rows.get_mut(index) {
                    row.result = Some(detail);
                }
            }
            ModuleId::Registry => {
                if let Some(row) = self.registry.rows.get_mut(index) {
                    row.result = Some(detail);
                }
            }
            ModuleId::Optimize => {
                if let Some(row) = self.optimize.rows.get_mut(index) {
                    if detail.is_ok() {
                        row.state = if row.desired {
                            TweakState::Applied
                        } else {
                            TweakState::NotApplied
                        };
                    }
                    row.result = Some(detail);
                }
            }
            _ => {}
        }
    }

    fn finish_action(&mut self, module: ModuleId, summary: String) {
        match module {
            ModuleId::Cleanup => {
                self.cleanup.phase = WorkPhase::Done;
                self.cleanup.summary = summary.clone();
            }
            ModuleId::Startup => {
                self.startup.phase = WorkPhase::Select;
                self.startup.summary = summary.clone();
            }
            ModuleId::Packages => {
                self.packages.phase = WorkPhase::Done;
                self.packages.summary = summary.clone();
            }
            ModuleId::Registry => {
                self.registry.phase = WorkPhase::Done;
                self.registry.summary = summary.clone();
            }
            ModuleId::Optimize => {
                self.optimize.phase = WorkPhase::Select;
                self.optimize.summary = summary.clone();
            }
            _ => {}
        }
        self.set_notice(&summary, false);
    }

    fn set_notice(&mut self, message: &str, is_error: bool) {
        self.notice = Some(Notice {
            message: message.to_string(),
            is_error,
            created: Instant::now(),
        });
    }

    fn expire_notice(&mut self) {
        if self
            .notice
            .as_ref()
            .is_some_and(|notice| notice.created.elapsed() > Duration::from_secs(5))
        {
            self.notice = None;
        }
    }
}

#[derive(Default)]
struct LiveStats {
    cpu: f32,
    memory_used: u64,
    memory_total: u64,
    disk_free: u64,
    disk_total: u64,
    disk_name: String,
    uptime: u64,
    health: u32,
    health_status: String,
    recommendations: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WorkPhase {
    Scanning,
    Select,
    Applying,
    Done,
    Error,
}

struct CleanupState {
    phase: WorkPhase,
    rows: Vec<CleanupRow>,
    cursor: usize,
    summary: String,
    error: String,
}

impl Default for CleanupState {
    fn default() -> Self {
        Self {
            phase: WorkPhase::Scanning,
            rows: Vec::new(),
            cursor: 0,
            summary: String::new(),
            error: String::new(),
        }
    }
}

impl CleanupState {
    fn fail(&mut self, error: String) {
        self.phase = WorkPhase::Error;
        self.error = error;
    }
}

struct CleanupRow {
    target: CleanupTarget,
    checked: bool,
    result: Option<std::result::Result<String, String>>,
}

struct DiskState {
    started: bool,
    loading: bool,
    path: PathBuf,
    rows: Vec<(PathBuf, u64)>,
    cursor: usize,
}

impl Default for DiskState {
    fn default() -> Self {
        Self {
            started: false,
            loading: true,
            path: dirs::home_dir().unwrap_or_else(|| PathBuf::from("C:\\")),
            rows: Vec::new(),
            cursor: 0,
        }
    }
}

struct StartupState {
    phase: WorkPhase,
    rows: Vec<StartupRow>,
    cursor: usize,
    summary: String,
}

impl Default for StartupState {
    fn default() -> Self {
        Self {
            phase: WorkPhase::Scanning,
            rows: Vec::new(),
            cursor: 0,
            summary: String::new(),
        }
    }
}

struct StartupRow {
    item: StartupItemInfo,
    staged: bool,
    result: Option<std::result::Result<String, String>>,
}

struct PackageState {
    started: bool,
    phase: WorkPhase,
    rows: Vec<PackageRow>,
    cursor: usize,
    summary: String,
    error: String,
}

impl Default for PackageState {
    fn default() -> Self {
        Self {
            started: false,
            phase: WorkPhase::Scanning,
            rows: Vec::new(),
            cursor: 0,
            summary: String::new(),
            error: String::new(),
        }
    }
}

impl PackageState {
    fn fail(&mut self, error: String) {
        self.phase = WorkPhase::Error;
        self.error = error;
    }
}

struct PackageRow {
    name: String,
    id: String,
    current: String,
    available: String,
    checked: bool,
    result: Option<std::result::Result<String, String>>,
}

struct RegistryState {
    started: bool,
    phase: WorkPhase,
    rows: Vec<RegistryRow>,
    cursor: usize,
    summary: String,
    error: String,
}

impl Default for RegistryState {
    fn default() -> Self {
        Self {
            started: false,
            phase: WorkPhase::Scanning,
            rows: Vec::new(),
            cursor: 0,
            summary: String::new(),
            error: String::new(),
        }
    }
}

impl RegistryState {
    fn fail(&mut self, error: String) {
        self.phase = WorkPhase::Error;
        self.error = error;
    }
}

struct RegistryRow {
    finding: AuditFinding,
    checked: bool,
    result: Option<std::result::Result<String, String>>,
}

struct OptimizeState {
    started: bool,
    phase: WorkPhase,
    rows: Vec<OptimizeRow>,
    cursor: usize,
    summary: String,
}

impl Default for OptimizeState {
    fn default() -> Self {
        Self {
            started: false,
            phase: WorkPhase::Scanning,
            rows: Vec::new(),
            cursor: 0,
            summary: String::new(),
        }
    }
}

struct OptimizeRow {
    tweak: Tweak,
    state: TweakState,
    desired: bool,
    result: Option<std::result::Result<String, String>>,
}

enum WorkerEvent {
    CleanupScanned(std::result::Result<Vec<CleanupTarget>, String>),
    DiskScanned {
        path: PathBuf,
        rows: Vec<(PathBuf, u64)>,
    },
    StartupScanned(Vec<StartupItemInfo>),
    PackagesScanned(std::result::Result<Vec<(String, String, String, String)>, String>),
    RegistryScanned(std::result::Result<Vec<AuditFinding>, String>),
    OptimizeScanned(Vec<OptimizeRow>),
    ActionProgress {
        module: ModuleId,
        index: usize,
        detail: std::result::Result<String, String>,
    },
    ActionComplete {
        module: ModuleId,
        summary: String,
    },
}

struct Notice {
    message: String,
    is_error: bool,
    created: Instant,
}

#[derive(Clone, Copy)]
enum PendingAction {
    Optimize,
}

struct Confirmation {
    title: String,
    message: String,
    required: Option<String>,
    input: String,
    action: PendingAction,
}

#[derive(Default)]
struct PaletteState {
    query: String,
    cursor: usize,
}

impl PaletteState {
    fn filtered(&self) -> Vec<&'static PaletteCommand> {
        let needle = self.query.to_lowercase();
        PALETTE_COMMANDS
            .iter()
            .filter(|command| {
                needle.is_empty()
                    || command.command.to_lowercase().contains(&needle)
                    || command.description.to_lowercase().contains(&needle)
            })
            .collect()
    }
}

struct PaletteCommand {
    command: &'static str,
    description: &'static str,
    module: ModuleId,
}

const PALETTE_COMMANDS: [PaletteCommand; 7] = [
    PaletteCommand {
        command: "status",
        description: "open system dashboard",
        module: ModuleId::Dashboard,
    },
    PaletteCommand {
        command: "clean --dry-run",
        description: "review and clean caches",
        module: ModuleId::Cleanup,
    },
    PaletteCommand {
        command: "disk --mode tree",
        description: "inspect largest folders",
        module: ModuleId::Disk,
    },
    PaletteCommand {
        command: "startup list",
        description: "stage startup changes",
        module: ModuleId::Startup,
    },
    PaletteCommand {
        command: "winget audit",
        description: "review package updates",
        module: ModuleId::Packages,
    },
    PaletteCommand {
        command: "registry scan",
        description: "audit Windows configuration",
        module: ModuleId::Registry,
    },
    PaletteCommand {
        command: "optimize --profile gaming",
        description: "review optimization tweaks",
        module: ModuleId::Optimize,
    },
];

fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();
    if area.width < 76 || area.height < 22 {
        let message = Paragraph::new(vec![
            Line::styled("WinMole needs a little more room", bold(TEXT)),
            Line::styled("Resize the terminal to at least 76 x 22 cells.", MUTED),
        ])
        .alignment(Alignment::Center)
        .block(pane(" TERMINAL SIZE "));
        frame.render_widget(message, area);
        return;
    }

    let shell = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(18),
        Constraint::Length(2),
    ])
    .split(area);
    render_header(frame, shell[0], app);

    let body = Layout::horizontal([Constraint::Length(23), Constraint::Min(50)]).split(shell[1]);
    render_rail(frame, body[0], app);
    let content = body[1].inner(Margin::new(2, 1));
    match app.active {
        ModuleId::Dashboard => render_dashboard(frame, content, app),
        ModuleId::Cleanup => render_cleanup(frame, content, app),
        ModuleId::Disk => render_disk(frame, content, app),
        ModuleId::Startup => render_startup(frame, content, app),
        ModuleId::Packages => render_packages(frame, content, app),
        ModuleId::Registry => render_registry(frame, content, app),
        ModuleId::Optimize => render_optimize(frame, content, app),
    }
    render_footer(frame, shell[2], app);

    if let Some(palette) = &app.palette {
        render_palette(frame, area, palette);
    }
    if let Some(confirm) = &app.confirm {
        render_confirmation(frame, area, confirm);
    }
}

fn render_header(frame: &mut Frame, area: Rect, app: &App) {
    let columns =
        Layout::horizontal([Constraint::Percentage(45), Constraint::Percentage(55)]).split(area);
    let border = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(MUTED));
    frame.render_widget(border.clone(), area);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("▲ WinMole", bold(ACCENT)),
            Span::styled(format!("  v{}", env!("CARGO_PKG_VERSION")), MUTED),
        ])),
        columns[0],
    );
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("health ", MUTED),
            Span::styled(
                app.stats.health.to_string(),
                bold(health_color(app.stats.health)),
            ),
            Span::styled("  ·  ", MUTED),
            Span::styled(
                format!("{} cleanable", commands::format_size(cleanable_total(app))),
                bold(WARNING),
            ),
            Span::styled("  ·  up ", MUTED),
            Span::styled(format_uptime(app.stats.uptime), bold(TEXT)),
        ]))
        .alignment(Alignment::Right),
        columns[1],
    );
}

fn render_rail(frame: &mut Frame, area: Rect, app: &App) {
    let rows = ModuleId::ALL.iter().map(|module| {
        let active = *module == app.active;
        let prefix = if active { "❯" } else { " " };
        let style = if active {
            Style::default()
                .fg(SELECTED)
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(TEXT)
        };
        ListItem::new(format!(
            "{}[{}] {}",
            prefix,
            module.number(),
            module.label()
        ))
        .style(style)
    });
    let list = List::new(rows).block(
        Block::default()
            .borders(Borders::RIGHT)
            .border_style(Style::default().fg(MUTED)),
    );
    frame.render_widget(list, area);
    if area.height > 3 {
        let brand = Rect::new(
            area.x + 2,
            area.bottom().saturating_sub(2),
            area.width.saturating_sub(4),
            1,
        );
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("╱\\", ACCENT),
                Span::styled(" dig deeper", MUTED),
            ])),
            brand,
        );
    }
}

fn render_footer(frame: &mut Frame, area: Rect, app: &App) {
    frame.render_widget(
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(MUTED)),
        area,
    );
    let text_area = Rect::new(
        area.x,
        area.y.saturating_add(1),
        area.width,
        area.height.saturating_sub(1),
    );
    let columns = Layout::horizontal([Constraint::Percentage(75), Constraint::Percentage(25)])
        .split(text_area);
    let hint = match app.active {
        ModuleId::Dashboard => "1-7 modules  ·  Esc dashboard  ·  q quit",
        ModuleId::Disk => "↑↓ select  ·  r rescan  ·  Esc dashboard  ·  q quit",
        _ => "↑↓ move  ·  Space stage  ·  Enter apply  ·  a safe/all  ·  r rescan",
    };
    let left = if let Some(notice) = &app.notice {
        Line::styled(
            &notice.message,
            if notice.is_error { DANGER } else { SUCCESS },
        )
    } else {
        Line::styled(hint, MUTED)
    };
    frame.render_widget(Paragraph::new(left), columns[0]);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(":", bold(SELECTED)),
            Span::styled(" palette", MUTED),
        ]))
        .alignment(Alignment::Right),
        columns[1],
    );
}

fn render_dashboard(frame: &mut Frame, area: Rect, app: &App) {
    let rows = Layout::vertical([
        Constraint::Length(7),
        Constraint::Length(11),
        Constraint::Min(5),
    ])
    .split(area);
    let gauges = Layout::horizontal([
        Constraint::Ratio(1, 3),
        Constraint::Ratio(1, 3),
        Constraint::Ratio(1, 3),
    ])
    .split(rows[0]);

    render_metric(
        frame,
        gauges[0],
        " CPU ",
        app.stats.cpu as f64,
        format!("{} logical cores", app.system.cpus().len()),
    );
    let memory_percent = percent(app.stats.memory_used, app.stats.memory_total);
    render_metric(
        frame,
        gauges[1],
        " MEMORY ",
        memory_percent,
        format!(
            "{}/{}",
            commands::format_size(app.stats.memory_used),
            commands::format_size(app.stats.memory_total)
        ),
    );
    let disk_used = app.stats.disk_total.saturating_sub(app.stats.disk_free);
    render_metric(
        frame,
        gauges[2],
        &format!(" DISK {} ", display_disk_name(&app.stats.disk_name)),
        percent(disk_used, app.stats.disk_total),
        format!("{} free", commands::format_size(app.stats.disk_free)),
    );

    let middle =
        Layout::horizontal([Constraint::Percentage(60), Constraint::Percentage(40)]).split(rows[1]);
    render_cleanable_summary(frame, middle[0], app);
    render_startup_summary(frame, middle[1], app);
    render_recommendations(frame, rows[2], app);
}

fn render_metric(frame: &mut Frame, area: Rect, title: &str, value: f64, detail: String) {
    let inner = pane(title).inner(area);
    frame.render_widget(pane(title), area);
    if inner.height == 0 {
        return;
    }
    let rows = Layout::vertical([
        Constraint::Length(2),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .split(inner);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(format!("{value:.1}%"), bold(TEXT)),
            Span::styled(format!("  {detail}"), MUTED),
        ])),
        rows[0],
    );
    let color = threshold_color(value);
    frame.render_widget(
        Gauge::default()
            .gauge_style(Style::default().fg(color).bg(Color::Black))
            .ratio((value / 100.0).clamp(0.0, 1.0))
            .label(""),
        rows[2],
    );
}

fn render_cleanable_summary(frame: &mut Frame, area: Rect, app: &App) {
    let total = cleanable_total(app);
    let title = format!(
        " CLEANABLE · {} ",
        if app.cleanup.phase == WorkPhase::Scanning {
            "scanning".to_string()
        } else {
            commands::format_size(total)
        }
    );
    let block = pane(&title);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let mut lines = Vec::new();
    if app.cleanup.phase == WorkPhase::Scanning {
        lines.push(Line::from(vec![
            Span::styled(spinner(app.tick), bold(SELECTED)),
            Span::styled(" Scanning cleanup targets…", TEXT),
        ]));
    } else {
        for row in app
            .cleanup
            .rows
            .iter()
            .take(inner.height.saturating_sub(2) as usize)
        {
            let width = if total > 0 {
                ((row.target.size as f64 / total as f64) * 10.0).ceil() as usize
            } else {
                0
            };
            lines.push(Line::from(vec![
                Span::styled(format!("{:<24}", row.target.name), TEXT),
                Span::styled(
                    format!("{:>9}", commands::format_size(row.target.size)),
                    WARNING,
                ),
                Span::styled(format!(" {}", "█".repeat(width.min(10))), SUCCESS),
            ]));
        }
    }
    lines.push(Line::styled("press 2 to review & clean", MUTED));
    frame.render_widget(Paragraph::new(lines), inner);
}

fn render_startup_summary(frame: &mut Frame, area: Rect, app: &App) {
    let block = pane(" STARTUP IMPACT ");
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let mut lines = Vec::new();
    if app.startup.phase == WorkPhase::Scanning {
        lines.push(Line::from(vec![
            Span::styled(spinner(app.tick), bold(SELECTED)),
            Span::styled(" Reading startup items…", TEXT),
        ]));
    } else {
        let mut items: Vec<_> = app.startup.rows.iter().collect();
        items.sort_by_key(|row| match row.item.impact.as_str() {
            "High" => 0,
            "Medium" => 1,
            _ => 2,
        });
        for row in items
            .into_iter()
            .take(inner.height.saturating_sub(2) as usize)
        {
            lines.push(Line::from(vec![
                Span::styled(format!("{:<24}", row.item.name), TEXT),
                Span::styled(
                    format!("[{}]", impact_short(&row.item.impact)),
                    impact_color(&row.item.impact),
                ),
            ]));
        }
    }
    lines.push(Line::styled("press 4 to manage", MUTED));
    frame.render_widget(Paragraph::new(lines), inner);
}

fn render_recommendations(frame: &mut Frame, area: Rect, app: &App) {
    let mut lines = Vec::new();
    if app.stats.disk_total > 0
        && percent(
            app.stats.disk_total.saturating_sub(app.stats.disk_free),
            app.stats.disk_total,
        ) >= 80.0
    {
        lines.push(Line::from(vec![
            Span::styled("⚠ ", WARNING),
            Span::styled("Disk space is low — cleaning can reclaim ", TEXT),
            Span::styled(commands::format_size(cleanable_total(app)), bold(TEXT)),
            Span::styled(" → 2", ACCENT),
        ]));
    }
    let high_startup = app
        .startup
        .rows
        .iter()
        .filter(|row| row.item.enabled && row.item.impact == "High")
        .count();
    if high_startup > 0 {
        lines.push(Line::from(vec![
            Span::styled("⚠ ", WARNING),
            Span::styled(
                format!("{high_startup} high-impact startup items slow boot"),
                TEXT,
            ),
            Span::styled(" → 4", ACCENT),
        ]));
    }
    for recommendation in app.stats.recommendations.iter().take(2) {
        lines.push(Line::from(vec![
            Span::styled("⚠ ", WARNING),
            Span::styled(recommendation, TEXT),
        ]));
    }
    if lines.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("✓ ", SUCCESS),
            Span::styled(
                format!("Health {} · {}", app.stats.health, app.stats.health_status),
                TEXT,
            ),
        ]));
    }
    frame.render_widget(
        Paragraph::new(lines)
            .block(pane(" RECOMMENDATIONS "))
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn render_cleanup(frame: &mut Frame, area: Rect, app: &App) {
    match app.cleanup.phase {
        WorkPhase::Scanning => render_scanning(
            frame,
            area,
            " CLEANUP · SCANNING ",
            app.tick,
            &[
                "Scanning user temp",
                "Scanning browser caches",
                "Scanning Windows caches",
                "Calculating reclaimable space",
            ],
        ),
        WorkPhase::Error => render_error(frame, area, " CLEANUP ", &app.cleanup.error),
        _ => {
            let columns =
                Layout::horizontal([Constraint::Percentage(68), Constraint::Percentage(32)])
                    .split(area);
            let items = app
                .cleanup
                .rows
                .iter()
                .map(|row| {
                    let status =
                        result_prefix(&row.result, app.cleanup.phase, row.checked, app.tick);
                    let risk = if row.target.requires_admin {
                        ("[Mod]", WARNING)
                    } else {
                        ("[Safe]", SUCCESS)
                    };
                    ListItem::new(Line::from(vec![
                        Span::styled(status, status_color(&row.result)),
                        Span::styled(
                            format!("[{}] ", if row.checked { "x" } else { " " }),
                            if row.checked { SUCCESS } else { MUTED },
                        ),
                        Span::styled(format!("{:<25}", row.target.name), TEXT),
                        Span::styled(
                            format!("{:>8} ", format_count(row.target.file_count)),
                            MUTED,
                        ),
                        Span::styled(
                            format!("{:>9} ", commands::format_size(row.target.size)),
                            WARNING,
                        ),
                        Span::styled(risk.0, risk.1),
                    ]))
                })
                .collect::<Vec<_>>();
            render_selectable_list(
                frame,
                columns[0],
                " CLEANUP · SELECT CATEGORIES ",
                items,
                app.cleanup.cursor,
            );
            render_cleanup_preview(frame, columns[1], app);
        }
    }
}

fn render_cleanup_preview(frame: &mut Frame, area: Rect, app: &App) {
    let selected: Vec<_> = app.cleanup.rows.iter().filter(|row| row.checked).collect();
    let bytes: u64 = selected.iter().map(|row| row.target.size).sum();
    let items: u64 = selected
        .iter()
        .filter_map(|row| row.target.file_count)
        .sum();
    let mut lines = vec![
        Line::styled(format!("Selected {} categories", selected.len()), TEXT),
        Line::styled(format!("Items    {items}"), MUTED),
        Line::default(),
        Line::styled("Reclaims", MUTED),
        Line::styled(commands::format_size(bytes), bold(WARNING)),
        Line::default(),
        Line::from(vec![
            Span::styled("⚠ ", WARNING),
            Span::styled("Files in use are skipped", MUTED),
        ]),
    ];
    match app.cleanup.phase {
        WorkPhase::Applying => lines.push(Line::styled(
            format!("{} Cleaning selected categories…", spinner(app.tick)),
            SELECTED,
        )),
        WorkPhase::Done => {
            lines.push(Line::styled(format!("✓ {}", app.cleanup.summary), SUCCESS));
            lines.push(Line::styled("Enter or r to rescan", MUTED));
        }
        _ => lines.push(Line::styled("press Enter to apply", MUTED)),
    }
    frame.render_widget(
        Paragraph::new(lines)
            .block(pane(" PREVIEW "))
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn render_disk(frame: &mut Frame, area: Rect, app: &App) {
    if app.disk.loading {
        render_scanning(
            frame,
            area,
            " DISK · SCANNING ",
            app.tick,
            &["Reading folders", "Calculating sizes", "Building tree"],
        );
        return;
    }
    let total: u64 = app.disk.rows.iter().map(|(_, size)| size).sum();
    let max = app.disk.rows.first().map(|(_, size)| *size).unwrap_or(1);
    let items = app
        .disk
        .rows
        .iter()
        .enumerate()
        .map(|(index, (path, size))| {
            let branch = if index + 1 == app.disk.rows.len() {
                "└──"
            } else {
                "├──"
            };
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            let bar = proportional_bar(*size, max, 16);
            ListItem::new(Line::from(vec![
                Span::styled(format!("{branch} "), ACCENT),
                Span::styled(format!("{:<32}", name), TEXT),
                Span::styled(format!("{:>10} ", commands::format_size(*size)), WARNING),
                Span::styled(bar, SUCCESS),
            ]))
        })
        .collect();
    render_selectable_list(
        frame,
        area,
        &format!(
            " DISK · {} · TREE · {} ",
            app.disk.path.display(),
            commands::format_size(total)
        ),
        items,
        app.disk.cursor,
    );
}

fn render_startup(frame: &mut Frame, area: Rect, app: &App) {
    if app.startup.phase == WorkPhase::Scanning {
        render_scanning(
            frame,
            area,
            " STARTUP · SCANNING ",
            app.tick,
            &[
                "Reading Run keys",
                "Reading Startup folders",
                "Estimating boot impact",
            ],
        );
        return;
    }
    let columns =
        Layout::horizontal([Constraint::Percentage(68), Constraint::Percentage(32)]).split(area);
    let items = app
        .startup
        .rows
        .iter()
        .map(|row| {
            ListItem::new(Line::from(vec![
                Span::styled(
                    result_prefix(
                        &row.result,
                        app.startup.phase,
                        row.item.enabled != row.staged,
                        app.tick,
                    ),
                    status_color(&row.result),
                ),
                Span::styled(
                    if row.staged { "[ON]  " } else { "[OFF] " },
                    if row.staged { SUCCESS } else { MUTED },
                ),
                Span::styled(format!("{:<30}", row.item.name), TEXT),
                Span::styled(
                    format!("[{}]", impact_short(&row.item.impact)),
                    impact_color(&row.item.impact),
                ),
            ]))
        })
        .collect();
    render_selectable_list(
        frame,
        columns[0],
        " STARTUP · STAGED CHANGES ",
        items,
        app.startup.cursor,
    );
    let enabled_cost: f64 = app
        .startup
        .rows
        .iter()
        .filter(|row| row.staged)
        .map(|row| impact_seconds(&row.item.impact))
        .sum();
    let current_cost: f64 = app
        .startup
        .rows
        .iter()
        .filter(|row| row.item.enabled)
        .map(|row| impact_seconds(&row.item.impact))
        .sum();
    let delta = current_cost - enabled_cost;
    let lines = vec![
        Line::styled("Estimated boot", MUTED),
        Line::styled(format!("{:.1}s", 10.0 + enabled_cost), bold(TEXT)),
        Line::default(),
        Line::styled(
            if delta > 0.0 {
                format!("▼ {delta:.1}s faster than current")
            } else if delta < 0.0 {
                format!("▲ {:.1}s slower than current", -delta)
            } else {
                "No staged impact change".to_string()
            },
            if delta >= 0.0 { SUCCESS } else { WARNING },
        ),
        Line::default(),
        Line::styled("Changes apply on Enter, not on toggle", MUTED),
        Line::styled(&app.startup.summary, SUCCESS),
    ];
    frame.render_widget(
        Paragraph::new(lines).block(pane(" BOOT IMPACT ")),
        columns[1],
    );
}

fn render_packages(frame: &mut Frame, area: Rect, app: &App) {
    if app.packages.phase == WorkPhase::Scanning {
        render_scanning(
            frame,
            area,
            " PACKAGES · WINGET AUDIT ",
            app.tick,
            &[
                "Refreshing winget sources",
                "Comparing installed versions",
                "Building update plan",
            ],
        );
        return;
    }
    if app.packages.phase == WorkPhase::Error {
        render_error(frame, area, " PACKAGES ", &app.packages.error);
        return;
    }
    let columns =
        Layout::horizontal([Constraint::Percentage(68), Constraint::Percentage(32)]).split(area);
    let items = app
        .packages
        .rows
        .iter()
        .map(|row| {
            ListItem::new(Line::from(vec![
                Span::styled(
                    result_prefix(&row.result, app.packages.phase, row.checked, app.tick),
                    status_color(&row.result),
                ),
                Span::styled(
                    format!("[{}] ", if row.checked { "x" } else { " " }),
                    if row.checked { SUCCESS } else { MUTED },
                ),
                Span::styled(format!("{:<28}", row.name), TEXT),
                Span::styled(format!("{} ", row.current), MUTED),
                Span::styled(format!("→ {}", row.available), WARNING),
            ]))
        })
        .collect();
    render_selectable_list(
        frame,
        columns[0],
        " PACKAGES · AVAILABLE UPDATES ",
        items,
        app.packages.cursor,
    );
    let selected = app.packages.rows.iter().filter(|row| row.checked).count();
    let current = app.packages.rows.get(app.packages.cursor);
    let lines = vec![
        Line::styled(
            format!("{} updates available", app.packages.rows.len()),
            TEXT,
        ),
        Line::styled(format!("{selected} selected"), WARNING),
        Line::default(),
        Line::styled(
            current
                .map(|row| row.name.as_str())
                .unwrap_or("No package selected"),
            bold(TEXT),
        ),
        Line::styled(current.map(|row| row.id.as_str()).unwrap_or(""), MUTED),
        Line::default(),
        Line::styled("press Enter to update", MUTED),
        Line::styled(&app.packages.summary, SUCCESS),
    ];
    frame.render_widget(
        Paragraph::new(lines).block(pane(" UPDATE PLAN ")),
        columns[1],
    );
}

fn render_registry(frame: &mut Frame, area: Rect, app: &App) {
    if app.registry.phase == WorkPhase::Scanning {
        render_scanning(
            frame,
            area,
            " REGISTRY · CONFIGURATION AUDIT ",
            app.tick,
            &[
                "Checking application paths",
                "Checking shared DLL references",
                "Reviewing uninstall records",
            ],
        );
        return;
    }
    if app.registry.phase == WorkPhase::Error {
        render_error(frame, area, " REGISTRY ", &app.registry.error);
        return;
    }
    let columns =
        Layout::horizontal([Constraint::Percentage(60), Constraint::Percentage(40)]).split(area);
    let items = app
        .registry
        .rows
        .iter()
        .map(|row| {
            let checkbox = if row.finding.remediable {
                if row.checked {
                    "[x]"
                } else {
                    "[ ]"
                }
            } else {
                "[-]"
            };
            ListItem::new(Line::from(vec![
                Span::styled(
                    result_prefix(&row.result, app.registry.phase, row.checked, app.tick),
                    status_color(&row.result),
                ),
                Span::styled(
                    format!("{checkbox} "),
                    if row.finding.remediable {
                        WARNING
                    } else {
                        MUTED
                    },
                ),
                Span::styled(format!("{:<30}", row.finding.name), TEXT),
                Span::styled(
                    if row.finding.remediable {
                        "[Fix]"
                    } else {
                        "[Review]"
                    },
                    if row.finding.remediable {
                        WARNING
                    } else {
                        MUTED
                    },
                ),
            ]))
        })
        .collect();
    render_selectable_list(
        frame,
        columns[0],
        " REGISTRY · FINDINGS ",
        items,
        app.registry.cursor,
    );
    let current = app.registry.rows.get(app.registry.cursor);
    let lines = if let Some(row) = current {
        vec![
            Line::styled(&row.finding.category, bold(ACCENT)),
            Line::styled(&row.finding.name, bold(TEXT)),
            Line::default(),
            Line::styled(&row.finding.evidence, TEXT),
            Line::default(),
            Line::styled(&row.finding.location, MUTED),
            Line::default(),
            Line::styled(
                if row.finding.remediable {
                    "Exact-value remediation available"
                } else {
                    "Review only · no automatic deletion"
                },
                if row.finding.remediable {
                    WARNING
                } else {
                    MUTED
                },
            ),
        ]
    } else {
        vec![Line::styled("✓ No configuration findings", SUCCESS)]
    };
    frame.render_widget(
        Paragraph::new(lines)
            .block(pane(" EVIDENCE "))
            .wrap(Wrap { trim: true }),
        columns[1],
    );
}

fn render_optimize(frame: &mut Frame, area: Rect, app: &App) {
    if app.optimize.phase == WorkPhase::Scanning {
        render_scanning(
            frame,
            area,
            " OPTIMIZE · DETECTING STATE ",
            app.tick,
            &[
                "Loading tweak registry",
                "Reading current Windows state",
                "Checking risk and requirements",
            ],
        );
        return;
    }
    let columns =
        Layout::horizontal([Constraint::Percentage(63), Constraint::Percentage(37)]).split(area);
    let items = app
        .optimize
        .rows
        .iter()
        .map(|row| {
            ListItem::new(Line::from(vec![
                Span::styled(
                    result_prefix(
                        &row.result,
                        app.optimize.phase,
                        row.desired != (row.state == TweakState::Applied),
                        app.tick,
                    ),
                    status_color(&row.result),
                ),
                Span::styled(
                    if row.desired { "[ON]  " } else { "[OFF] " },
                    if row.desired { SUCCESS } else { MUTED },
                ),
                Span::styled(format!("{:<31}", row.tweak.name), TEXT),
                Span::styled(
                    format!("[{}]", risk_short(row.tweak.risk)),
                    risk_color(row.tweak.risk),
                ),
            ]))
        })
        .collect();
    render_selectable_list(
        frame,
        columns[0],
        " OPTIMIZE · TWEAKS ",
        items,
        app.optimize.cursor,
    );
    let current = app.optimize.rows.get(app.optimize.cursor);
    let lines = if let Some(row) = current {
        vec![
            Line::styled(&row.tweak.name, bold(TEXT)),
            Line::styled(row.tweak.category.to_string(), ACCENT),
            Line::default(),
            Line::styled(&row.tweak.description, TEXT),
            Line::default(),
            Line::from(vec![
                Span::styled("Current  ", MUTED),
                Span::styled(row.state.to_string(), TEXT),
            ]),
            Line::from(vec![
                Span::styled("Staged   ", MUTED),
                Span::styled(if row.desired { "On" } else { "Off" }, WARNING),
            ]),
            Line::from(vec![
                Span::styled("Risk     ", MUTED),
                Span::styled(row.tweak.risk.to_string(), risk_color(row.tweak.risk)),
            ]),
            Line::from(vec![
                Span::styled("Admin    ", MUTED),
                Span::styled(
                    if row.tweak.needs_admin() {
                        "Required"
                    } else {
                        "No"
                    },
                    TEXT,
                ),
            ]),
            Line::default(),
            Line::styled("Risky changes require typed confirmation", MUTED),
        ]
    } else {
        vec![Line::styled("No optimization tweaks registered", MUTED)]
    };
    frame.render_widget(
        Paragraph::new(lines)
            .block(pane(" DETAILS "))
            .wrap(Wrap { trim: true }),
        columns[1],
    );
}

fn render_scanning(frame: &mut Frame, area: Rect, title: &str, tick: usize, steps: &[&str]) {
    let current = (tick / 7).min(steps.len().saturating_sub(1));
    let lines = steps
        .iter()
        .enumerate()
        .map(|(index, step)| {
            if index < current {
                Line::from(vec![
                    Span::styled("✓ ", SUCCESS),
                    Span::styled(*step, TEXT),
                    Span::styled("  complete", MUTED),
                ])
            } else if index == current {
                Line::from(vec![
                    Span::styled(format!("{} ", spinner(tick)), bold(SELECTED)),
                    Span::styled(format!("{step}…"), TEXT),
                ])
            } else {
                Line::from(vec![Span::styled("○ ", MUTED), Span::styled(*step, MUTED)])
            }
        })
        .collect::<Vec<_>>();
    frame.render_widget(
        Paragraph::new(lines)
            .block(pane(title))
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn render_error(frame: &mut Frame, area: Rect, title: &str, error: &str) {
    frame.render_widget(
        Paragraph::new(vec![
            Line::styled("✗ Scan failed", bold(DANGER)),
            Line::default(),
            Line::styled(error, TEXT),
            Line::default(),
            Line::styled("Press r to retry.", MUTED),
        ])
        .block(pane(title))
        .wrap(Wrap { trim: true }),
        area,
    );
}

fn render_selectable_list(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    items: Vec<ListItem>,
    cursor: usize,
) {
    let mut state = ListState::default();
    if !items.is_empty() {
        state.select(Some(cursor.min(items.len() - 1)));
    }
    let list = List::new(items)
        .block(pane(title))
        .highlight_symbol("❯ ")
        .highlight_style(
            Style::default()
                .fg(SELECTED)
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_palette(frame: &mut Frame, area: Rect, palette: &PaletteState) {
    let popup = centered_rect(72, 52, area);
    frame.render_widget(Clear, popup);
    let block = Block::default()
        .title(" COMMAND PALETTE ")
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(ACCENT))
        .style(Style::default().bg(Color::Black));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);
    let rows = Layout::vertical([Constraint::Length(2), Constraint::Min(3)]).split(inner);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(": ", bold(SELECTED)),
            Span::styled(&palette.query, TEXT),
            Span::styled("▊", SELECTED),
        ])),
        rows[0],
    );
    let filtered = palette.filtered();
    let items = filtered
        .iter()
        .take(6)
        .map(|command| {
            ListItem::new(Line::from(vec![
                Span::styled(format!("{:<30}", command.command), TEXT),
                Span::styled(command.description, MUTED),
            ]))
        })
        .collect::<Vec<_>>();
    render_selectable_list(frame, rows[1], "", items, palette.cursor);
}

fn render_confirmation(frame: &mut Frame, area: Rect, confirm: &Confirmation) {
    let popup = centered_rect(60, 32, area);
    frame.render_widget(Clear, popup);
    let required = confirm.required.as_deref().unwrap_or("Enter");
    let lines = vec![
        Line::styled(&confirm.message, TEXT),
        Line::default(),
        Line::from(vec![
            Span::styled(format!("Type {required}: "), WARNING),
            Span::styled(&confirm.input, bold(TEXT)),
            Span::styled("▊", SELECTED),
        ]),
        Line::default(),
        Line::styled("Esc cancels without making changes", MUTED),
    ];
    frame.render_widget(
        Paragraph::new(lines)
            .block(
                Block::default()
                    .title(format!(" {} ", confirm.title))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(WARNING))
                    .style(Style::default().bg(Color::Black)),
            )
            .wrap(Wrap { trim: true }),
        popup,
    );
}

fn pane(title: &str) -> Block<'_> {
    Block::default()
        .title(Span::styled(title, ACCENT))
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(MUTED))
}

fn bold(color: Color) -> Style {
    Style::default().fg(color).add_modifier(Modifier::BOLD)
}

fn move_index(index: &mut usize, len: usize, delta: isize) {
    if len == 0 {
        *index = 0;
        return;
    }
    *index = if delta < 0 {
        index.saturating_sub(delta.unsigned_abs())
    } else {
        (*index + delta as usize).min(len - 1)
    };
}

fn spinner(tick: usize) -> &'static str {
    SPINNERS[(tick / 2) % SPINNERS.len()]
}

fn percent(value: u64, total: u64) -> f64 {
    if total == 0 {
        0.0
    } else {
        value as f64 / total as f64 * 100.0
    }
}

fn threshold_color(value: f64) -> Color {
    if value >= 90.0 {
        DANGER
    } else if value >= 70.0 {
        WARNING
    } else {
        SUCCESS
    }
}

fn health_color(value: u32) -> Color {
    if value >= 80 {
        SUCCESS
    } else if value >= 60 {
        WARNING
    } else {
        DANGER
    }
}

fn cleanable_total(app: &App) -> u64 {
    app.cleanup.rows.iter().map(|row| row.target.size).sum()
}

fn display_disk_name(name: &str) -> &str {
    if name.is_empty() {
        "C:"
    } else {
        name.trim_end_matches('\\')
    }
}

fn format_uptime(seconds: u64) -> String {
    let days = seconds / 86_400;
    let hours = (seconds % 86_400) / 3_600;
    let minutes = (seconds % 3_600) / 60;
    format!("{days}d {hours:02}:{minutes:02}")
}

fn impact_short(impact: &str) -> &str {
    match impact {
        "Medium" => "Med",
        "High" => "High",
        "Low" => "Low",
        _ => "?",
    }
}

fn impact_color(impact: &str) -> Color {
    match impact {
        "High" => DANGER,
        "Medium" => WARNING,
        "Low" => SUCCESS,
        _ => MUTED,
    }
}

fn impact_seconds(impact: &str) -> f64 {
    match impact {
        "High" => 4.0,
        "Medium" => 2.0,
        _ => 0.8,
    }
}

fn risk_short(risk: TweakRisk) -> &'static str {
    match risk {
        TweakRisk::Safe => "Safe",
        TweakRisk::Moderate => "Mod",
        TweakRisk::Risky => "Risky",
        TweakRisk::Dangerous => "Danger",
    }
}

fn risk_color(risk: TweakRisk) -> Color {
    match risk {
        TweakRisk::Safe => SUCCESS,
        TweakRisk::Moderate => WARNING,
        TweakRisk::Risky | TweakRisk::Dangerous => DANGER,
    }
}

fn format_count(count: Option<u64>) -> String {
    count
        .map(|value| format!("{value} items"))
        .unwrap_or_default()
}

fn proportional_bar(value: u64, max: u64, width: usize) -> String {
    let filled = if max == 0 {
        0
    } else {
        ((value as f64 / max as f64) * width as f64).ceil() as usize
    }
    .min(width);
    format!("{}{}", "█".repeat(filled), "░".repeat(width - filled))
}

fn result_prefix(
    result: &Option<std::result::Result<String, String>>,
    phase: WorkPhase,
    active: bool,
    tick: usize,
) -> String {
    match result {
        Some(Ok(_)) => "✓ ".to_string(),
        Some(Err(_)) => "✗ ".to_string(),
        None if phase == WorkPhase::Applying && active => format!("{} ", spinner(tick)),
        _ => "  ".to_string(),
    }
}

fn status_color(result: &Option<std::result::Result<String, String>>) -> Color {
    match result {
        Some(Ok(_)) => SUCCESS,
        Some(Err(_)) => DANGER,
        None => SELECTED,
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn test_app() -> App {
        let (worker_tx, worker_rx) = mpsc::channel();
        App {
            active: ModuleId::Dashboard,
            should_quit: false,
            system: System::new(),
            disks: Disks::new(),
            stats: LiveStats {
                cpu: 12.3,
                memory_used: 18 * 1024 * 1024 * 1024,
                memory_total: 32 * 1024 * 1024 * 1024,
                disk_free: 45 * 1024 * 1024 * 1024,
                disk_total: 256 * 1024 * 1024 * 1024,
                disk_name: "C:\\".to_string(),
                uptime: 3 * 86_400 + 14 * 3_600 + 24 * 60,
                health: 87,
                health_status: "Excellent".to_string(),
                recommendations: Vec::new(),
            },
            cleanup: CleanupState::default(),
            disk: DiskState::default(),
            startup: StartupState::default(),
            packages: PackageState::default(),
            registry: RegistryState::default(),
            optimize: OptimizeState::default(),
            palette: None,
            confirm: None,
            notice: None,
            worker_tx,
            worker_rx,
            last_stats_refresh: Instant::now(),
            last_health_refresh: Instant::now(),
            tick: 0,
        }
    }

    fn buffer_text(terminal: &Terminal<TestBackend>) -> String {
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect()
    }

    #[test]
    fn palette_filters_commands_and_descriptions() {
        let palette = PaletteState {
            query: "cache".to_string(),
            cursor: 0,
        };
        let filtered = palette.filtered();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].module, ModuleId::Cleanup);
    }

    #[test]
    fn uptime_and_bars_are_stable() {
        assert_eq!(format_uptime(3 * 86_400 + 14 * 3_600 + 24 * 60), "3d 14:24");
        assert_eq!(proportional_bar(50, 100, 10), "█████░░░░░");
    }

    #[test]
    fn dashboard_renders_shell_and_primary_panels() {
        let app = test_app();
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let screen = buffer_text(&terminal);
        assert!(screen.contains("WinMole"));
        assert!(screen.contains("Dashboard"));
        assert!(screen.contains("CLEANABLE"));
        assert!(screen.contains("STARTUP IMPACT"));
        assert!(screen.contains("RECOMMENDATIONS"));
        assert!(screen.contains("health 87"));
    }

    #[test]
    fn small_terminal_renders_resize_message() {
        let app = test_app();
        let backend = TestBackend::new(60, 18);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| render(frame, &app)).unwrap();
        assert!(buffer_text(&terminal).contains("Resize the terminal"));
    }

    #[test]
    fn every_module_and_palette_render_at_reference_size() {
        let mut app = test_app();
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();

        for module in ModuleId::ALL {
            app.active = module;
            terminal.draw(|frame| render(frame, &app)).unwrap();
            assert!(buffer_text(&terminal).contains(module.label()));
        }

        app.palette = Some(PaletteState::default());
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let screen = buffer_text(&terminal);
        assert!(screen.contains("COMMAND PALETTE"));
        assert!(screen.contains("clean --dry-run"));
    }
}
