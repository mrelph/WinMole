//! UI strings in Simplified Chinese and English, switched at runtime.
//!
//! The default follows the OS display language (via the nsh `locale()`): a
//! Chinese locale starts in Chinese, anything else in English. A toggle in
//! the header switches at runtime.

use nsh_core::prelude::*;

/// The UI language. Stored as a `global` so every reader and the header
/// toggle share one process-wide signal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Lang {
    /// Simplified Chinese.
    Zh,
    /// English.
    #[default]
    En,
}

/// The current UI language, as a shared signal (a `global` keyed by `Lang`).
/// Called from the engine thread while a component runs.
pub fn lang() -> Signal<Lang> {
    try_global::<Lang>().expect("lang(): a reactive runtime must be entered")
}

/// The initial language from the OS locale: Chinese → `Zh`, else `En`.
pub fn initial_lang() -> Lang {
    let binding = locale();
    let locale = binding.as_str();
    if locale.starts_with("zh") || locale.starts_with("cmn") || locale.starts_with("yue") {
        Lang::Zh
    } else {
        Lang::En
    }
}

/// Every key of the UI, with the two languages' strings (zh, en).
#[derive(Clone, Copy)]
pub struct T;
// The catalog intentionally holds keys for screens to grow into; unused
// entries are not a defect.
#[allow(dead_code)]
impl T {
    pub const TAGLINE: (&'static str, &'static str) =
        ("Windows 系统优化工具", "Windows System Optimization Tool");
    pub const LANGUAGE: (&'static str, &'static str) = ("语言", "Language");
    pub const MODULES: (&'static str, &'static str) = ("功能", "Modules");

    pub const NAV_DASHBOARD: (&'static str, &'static str) = ("总览", "Dashboard");
    pub const NAV_CLEANUP: (&'static str, &'static str) = ("清理", "Cleanup");
    pub const NAV_DISK: (&'static str, &'static str) = ("磁盘", "Disk");
    pub const NAV_STARTUP: (&'static str, &'static str) = ("启动项", "Startup");
    pub const NAV_PACKAGES: (&'static str, &'static str) = ("软件包", "Packages");
    pub const NAV_REGISTRY: (&'static str, &'static str) = ("注册表", "Registry");
    pub const NAV_OPTIMIZE: (&'static str, &'static str) = ("优化", "Optimize");

    pub const HEALTH: (&'static str, &'static str) = ("健康评分", "health");
    pub const CLEANABLE: (&'static str, &'static str) = ("可清理", "cleanable");
    pub const UP: (&'static str, &'static str) = ("运行", "up");
    pub const VERSION: (&'static str, &'static str) = ("版本", "version");

    pub const SCANNING: (&'static str, &'static str) = ("扫描中…", "Scanning…");
    pub const CPU: (&'static str, &'static str) = ("CPU", "CPU");
    pub const MEMORY: (&'static str, &'static str) = ("内存", "MEMORY");
    pub const DISK: (&'static str, &'static str) = ("磁盘", "DISK");
    pub const CORES: (&'static str, &'static str) = ("逻辑核心", "logical cores");
    pub const FREE: (&'static str, &'static str) = ("可用", "free");
    pub const RECOMMENDATIONS: (&'static str, &'static str) = ("建议", "RECOMMENDATIONS");
    pub const STARTUP_IMPACT: (&'static str, &'static str) = ("启动项影响", "STARTUP IMPACT");
    pub const DISK_LOW: (&'static str, &'static str) =
        ("磁盘空间不足 — 清理可回收", "Disk space is low — cleaning can reclaim");
    pub const NO_RECOMMENDATIONS: (&'static str, &'static str) =
        ("系统健康，无需处理", "Health is good, nothing to do");
    pub const RESCAN: (&'static str, &'static str) = ("重新扫描", "Rescan");
    pub const APPLY: (&'static str, &'static str) = ("应用", "Apply");
    pub const SELECT_ALL: (&'static str, &'static str) = ("全部选择", "Select all");
    pub const BACK: (&'static str, &'static str) = ("返回总览", "Back to dashboard");

    pub const CLEANUP_TITLE: (&'static str, &'static str) =
        ("选择清理类别", "Select cleanup categories");
    pub const CLEANUP_SCANNING: (&'static str, &'static str) =
        ("扫描临时文件、浏览器与系统缓存…", "Scanning temp files, browser and Windows caches…");
    pub const SELECTED: (&'static str, &'static str) = ("已选择", "Selected");
    pub const ITEMS: (&'static str, &'static str) = ("项", "items");
    pub const RECLAIMS: (&'static str, &'static str) = ("可回收", "Reclaims");
    pub const FILES_IN_USE_SKIPPED: (&'static str, &'static str) =
        ("使用中的文件将被跳过", "Files in use are skipped");
    pub const ADMIN: (&'static str, &'static str) = ("需管理员", "Admin");
    pub const SAFE: (&'static str, &'static str) = ("安全", "Safe");
    pub const SELECT_AT_LEAST_ONE: (&'static str, &'static str) =
        ("至少选择一个清理类别", "Select at least one cleanup category");
    pub const CLEAN_COMPLETE: (&'static str, &'static str) = ("清理完成", "Cleanup complete");
    pub const RESCAN_DONE: (&'static str, &'static str) =
        ("按“重新扫描”再次扫描", "Press Rescan to scan again");

    pub const RESIDUAL_PROCESSES: (&'static str, &'static str) =
        ("残留的开发工具进程", "Residual developer processes");
    pub const RESIDUAL_PROCESSES_HINT: (&'static str, &'static str) =
        ("仅识别无可见窗口的 IDE、opencode 与 Git 残留；活动进程受保护，孤立 OpenCode 服务仅在桌面端关闭后列出",
         "Only windowless IDE, opencode, and Git leftovers are listed; orphaned OpenCode services require the desktop app to be closed");
    pub const RESIDUAL_SCANNING: (&'static str, &'static str) =
        ("正在检查残留的 IDE、opencode 与 Git 进程…", "Checking for residual IDE, opencode, and Git processes…");
    pub const NO_RESIDUAL_PROCESSES: (&'static str, &'static str) =
        ("未发现残留进程", "No residual processes found");
    pub const TERMINATE_SELECTED: (&'static str, &'static str) =
        ("终止所选进程", "Terminate selected");
    pub const TERMINATING: (&'static str, &'static str) =
        ("正在终止所选进程…", "Terminating selected processes…");
    pub const SELECT_RESIDUAL_PROCESS: (&'static str, &'static str) =
        ("请选择要终止的残留进程", "Select a residual process to terminate");
    pub const PROCESS_SKIPPED: (&'static str, &'static str) =
        ("已跳过（进程已变化）", "Skipped (process changed)");
    pub const RESIDUAL_COMPLETE: (&'static str, &'static str) =
        ("残留进程清理完成", "Residual process cleanup complete");
    pub const CONFIRM_PROCESS_TITLE: (&'static str, &'static str) =
        ("确认终止残留进程", "Confirm residual process termination");
    pub const CONFIRM_PROCESS_MSG: (&'static str, &'static str) =
        ("将强制终止所选进程。输入 APPLY 以继续。",
         "The selected processes will be forcefully terminated. Type APPLY to continue.");
    pub const TERMINATE: (&'static str, &'static str) = ("终止", "Terminate");

    pub const DISK_TITLE: (&'static str, &'static str) = ("磁盘占用 · 文件夹", "DISK · FOLDERS");
    pub const DISK_SCANNING: (&'static str, &'static str) =
        ("正在分析最大文件夹…", "Analyzing largest folders…");

    pub const STARTUP_TITLE: (&'static str, &'static str) =
        ("启动项 · 待应用变更", "STARTUP · STAGED CHANGES");
    pub const STARTUP_SCANNING: (&'static str, &'static str) =
        ("读取启动项…", "Reading startup items…");
    pub const ON: (&'static str, &'static str) = ("开", "ON");
    pub const OFF: (&'static str, &'static str) = ("关", "OFF");
    pub const PARTIAL: (&'static str, &'static str) = ("部分", "PARTIAL");
    pub const ESTIMATED_BOOT: (&'static str, &'static str) = ("预计启动时间", "Estimated boot");
    pub const FASTER_THAN_CURRENT: (&'static str, &'static str) =
        ("比当前快", "faster than current");
    pub const SLOWER_THAN_CURRENT: (&'static str, &'static str) =
        ("比当前慢", "slower than current");
    pub const NO_IMPACT_CHANGE: (&'static str, &'static str) =
        ("未改变启动影响", "No staged impact change");
    pub const CHANGES_ON_APPLY: (&'static str, &'static str) =
        ("点击“应用”后写入，下次登录生效", "Written on Apply; takes effect at next sign-in");
    pub const NO_STARTUP_CHANGES: (&'static str, &'static str) =
        ("没有待应用的启动项变更", "No startup changes are staged");
    pub const STARTUP_RESULT: (&'static str, &'static str) =
        ("启动项变更：成功 {ok} 项，失败 {failed} 项", "Startup changes: {ok} applied, {failed} failed");

    pub const PACKAGES_TITLE: (&'static str, &'static str) =
        ("软件包 · 可用更新", "PACKAGES · AVAILABLE UPDATES");
    pub const PACKAGES_SCANNING: (&'static str, &'static str) =
        ("刷新 winget 源并比对版本…", "Refreshing winget sources and comparing versions…");
    pub const UPDATES_AVAILABLE: (&'static str, &'static str) = ("个可用更新", "updates available");
    pub const UPDATE_PLAN: (&'static str, &'static str) = ("更新计划", "UPDATE PLAN");
    pub const NO_PACKAGE_SELECTED: (&'static str, &'static str) = ("未选择软件包", "No package selected");
    pub const SELECT_AT_LEAST_ONE_PKG: (&'static str, &'static str) =
        ("至少选择一个软件包", "Select at least one package");
    pub const PACKAGES_COMPLETE: (&'static str, &'static str) =
        ("已更新软件包", "Packages updated");

    pub const REGISTRY_TITLE: (&'static str, &'static str) =
        ("注册表 · 配置审计", "REGISTRY · CONFIG AUDIT");
    pub const REGISTRY_SCANNING: (&'static str, &'static str) =
        ("检查路径、DLL 引用与卸载记录…", "Checking paths, DLL references and uninstall records…");
    pub const EVIDENCE: (&'static str, &'static str) = ("证据", "EVIDENCE");
    pub const FIX: (&'static str, &'static str) = ("修复", "Fix");
    pub const REVIEW: (&'static str, &'static str) = ("查看", "Review");
    pub const REMEDIABLE: (&'static str, &'static str) =
        ("可精确值修复", "Exact-value remediation available");
    pub const REVIEW_ONLY: (&'static str, &'static str) =
        ("仅查看 · 不做自动删除", "Review only · no automatic deletion");
    pub const NO_FINDINGS: (&'static str, &'static str) =
        ("✓ 未发现配置问题", "✓ No configuration findings");
    pub const SELECT_REMEDIABLE: (&'static str, &'static str) =
        ("请选择一个可修复的项", "Select a remediable finding");
    pub const REGISTRY_COMPLETE: (&'static str, &'static str) =
        ("已修复配置项", "Findings remediated");

    pub const OPTIMIZE_TITLE: (&'static str, &'static str) =
        ("优化 · 调整项", "OPTIMIZE · TWEAKS");
    pub const OPTIMIZE_SCANNING: (&'static str, &'static str) =
        ("读取当前 Windows 状态并评估风险…", "Reading current Windows state and assessing risk…");
    pub const DETAILS: (&'static str, &'static str) = ("详情", "DETAILS");
    pub const CURRENT: (&'static str, &'static str) = ("当前", "Current");
    pub const STAGED: (&'static str, &'static str) = ("待应用", "Staged");
    pub const RISK: (&'static str, &'static str) = ("风险", "Risk");
    pub const NO_TWEAKS: (&'static str, &'static str) = ("未注册优化项", "No optimization tweaks registered");
    pub const RISKY_CONFIRM: (&'static str, &'static str) =
        ("高风险变更需要输入确认", "Risky changes require typed confirmation");
    pub const NO_OPTIMIZE_CHANGES: (&'static str, &'static str) =
        ("没有待应用的优化变更", "No optimization changes are staged");
    pub const OPTIMIZE_COMPLETE: (&'static str, &'static str) =
        ("已应用优化变更", "Optimization changes applied");

    pub const CONFIRM_RISKY_TITLE: (&'static str, &'static str) =
        ("确认高风险优化变更", "Confirm risky optimization changes");
    pub const CONFIRM_RISKY_MSG: (&'static str, &'static str) =
        ("这些变更可能影响 Windows 行为。输入 APPLY 以继续。",
         "These changes can affect Windows behavior. Type APPLY to continue.");
    pub const CANCEL: (&'static str, &'static str) = ("取消", "Cancel");
    pub const OK: (&'static str, &'static str) = ("确定", "OK");

    pub const SCAN_FAILED: (&'static str, &'static str) = ("扫描失败", "Scan failed");
    pub const RETRY: (&'static str, &'static str) = ("重试", "Retry");
    pub const ERROR: (&'static str, &'static str) = ("错误", "Error");
    pub const UNKNOWN: (&'static str, &'static str) = ("未知", "Unknown");

    pub const SELECT_CATEGORIES: (&'static str, &'static str) =
        ("选择清理类别", "Select cleanup categories");
    pub const PREVIEW: (&'static str, &'static str) = ("预览", "PREVIEW");
}

/// Pick the pair's value for a specific language (safe on any thread).
pub fn pick(lang: Lang, pair: (&'static str, &'static str)) -> SharedString {
    match lang {
        Lang::Zh => pair.0.into(),
        Lang::En => pair.1.into(),
    }
}

/// Pick the pair's value for the current UI language (engine thread only,
/// since it reads a signal).
pub fn t(pair: (&'static str, &'static str)) -> SharedString {
    match lang().get() {
        Lang::Zh => pair.0.into(),
        Lang::En => pair.1.into(),
    }
}