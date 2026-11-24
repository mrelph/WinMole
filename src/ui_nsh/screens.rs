//! The NSH screens: the root `WinMoleApp` (sidebar + top bar + module
//! screen + notices + the risky-confirm dialog) and one builder per module.
//!
//! Everything reactive reads signals inside `text(move || …)` / `.bg(move ||
//! …)` bindings — the component body runs once. Colors come from the
//! `nsh_base` theme tokens so light/dark follows the system. The `Ctx` is
//! shared through an `Rc` because the worker receiver is not `Clone`.

use nsh_base::{input, theme};
use nsh_core::prelude::*;

use crate::commands;
use crate::commands::optimize::common::TweakState;
use crate::ui_nsh::i18n;
use crate::ui_nsh::i18n::{Lang, T};
use crate::ui_nsh::{
    cleanable_total, drain as drain_events, format_uptime, Ctx, ItemResult, ModuleId, Phase,
};
use std::rc::Rc;
use std::time::Duration;

// ---------------------------------------------------------------- helpers --

/// Panel card: rounded surface with padding, used as every screen's root.
fn card() -> nsh_core::element::View {
    view()
        .flex_col()
        .flex_1()
        .w_full()
        .gap(px(10.0))
        .p(px(16.0))
        .rounded(move || theme().radius)
        .bg(move || theme().card)
        .text_color(move || theme().card_foreground)
}

/// A primary pill button.
fn primary(label: SharedString, on_press: impl Fn() + 'static) -> nsh_core::element::View {
    view()
        .items_center()
        .justify_center()
        .px(px(14.0))
        .py(px(8.0))
        .rounded(move || theme().radius)
        .bg(move || theme().primary)
        .hover(|s| s.opacity(0.9))
        .active(|s| s.opacity(0.8))
        .on_press(move |_: &PressEvent| on_press())
        .child(
            text(label)
                .text_size(px(13.0))
                .text_color(move || theme().primary_foreground),
        )
}

/// A secondary (muted) pill button.
fn secondary(label: SharedString, on_press: impl Fn() + 'static) -> nsh_core::element::View {
    view()
        .items_center()
        .justify_center()
        .px(px(14.0))
        .py(px(8.0))
        .rounded(move || theme().radius)
        .bg(move || theme().secondary)
        .hover(|s| s.opacity(0.9))
        .active(|s| s.opacity(0.8))
        .on_press(move |_: &PressEvent| on_press())
        .child(
            text(label)
                .text_size(px(13.0))
                .text_color(move || theme().secondary_foreground),
        )
}

/// A destructive pill button.
fn destructive(label: SharedString, on_press: impl Fn() + 'static) -> nsh_core::element::View {
    view()
        .items_center()
        .justify_center()
        .px(px(14.0))
        .py(px(8.0))
        .rounded(move || theme().radius)
        .bg(move || theme().destructive)
        .hover(|s| s.opacity(0.9))
        .on_press(move |_: &PressEvent| on_press())
        .child(
            text(label)
                .text_size(px(13.0))
                .text_color(move || theme().card_foreground),
        )
}

/// Big rounded metric card on the dashboard.
fn metric(title: SharedString, value: impl Fn() -> String + 'static) -> nsh_core::element::View {
    view()
        .flex_col()
        .flex_1()
        .gap(px(4.0))
        .p(px(14.0))
        .rounded(move || theme().radius)
        .bg(move || theme().muted)
        .child(
            text(title)
                .text_size(px(11.0))
                .text_color(move || theme().muted_foreground),
        )
        .child(
            text(value)
                .text_size(px(20.0))
                .font_weight(FontWeight::BOLD),
        )
}

/// One row of a selectable list: `[✓] title · detail`, whole row presses.
fn list_row(
    title: impl Fn() -> String + 'static,
    detail: impl Fn() -> String + 'static,
    checked: impl Fn() -> bool + Clone + 'static,
    status: impl Fn() -> String + 'static,
    on_press: impl Fn() + 'static,
) -> nsh_core::element::View {
    let bg_checked = checked.clone();
    let box_checked = checked.clone();
    let mark_checked = checked;
    view()
        .flex_row()
        .w_full()
        .items_center()
        .gap(px(10.0))
        .px(px(10.0))
        .py(px(8.0))
        .rounded(move || theme().radius)
        .bg(move || {
            if bg_checked() {
                theme().accent
            } else {
                theme().muted
            }
        })
        .hover(|s| s.opacity(0.92))
        .on_press(move |_: &PressEvent| on_press())
        .child(
            // checkbox square
            view()
                .size(px(18.0))
                .items_center()
                .justify_center()
                .rounded(px(4.0))
                .bg(move || {
                    if box_checked() {
                        theme().primary
                    } else {
                        theme().secondary
                    }
                })
                .child(
                    text(move || {
                        if mark_checked() { "✓".to_string() } else { String::new() }
                    })
                    .text_size(px(12.0))
                    .font_weight(FontWeight::BOLD)
                    .text_color(move || theme().primary_foreground),
                ),
        )
        .child(
            view()
                .flex_col()
                .flex_1()
                .gap(px(2.0))
                .child(
                    text(title)
                        .text_size(px(13.0))
                        .truncate(),
                )
                .child(
                    text(detail)
                        .text_size(px(11.0))
                        .text_color(move || theme().muted_foreground)
                        .number_of_lines(1)
                        .truncate(),
                ),
        )
        .child(
            text(status)
                .text_size(px(11.0))
                .text_color(move || theme().muted_foreground),
        )
}

/// A scan / apply state banner (spinner-ish label) or error banner.
fn phase_banner(label: SharedString, error: bool) -> nsh_core::element::View {
    view()
        .w_full()
        .items_center()
        .px(px(12.0))
        .py(px(8.0))
        .rounded(move || theme().radius)
        .bg(move || {
            if error {
                theme().destructive
            } else {
                theme().muted
            }
        })
        .child(
            text(label)
                .text_size(px(12.0))
                .text_color(move || {
                    if error {
                        theme().card_foreground
                    } else {
                        theme().muted_foreground
                    }
                }),
        )
}

/// Banner text for a scan error.
fn error_text(msg: &str) -> SharedString {
    format!("{} {msg}", i18n::t(T::SCAN_FAILED)).into()
}

/// Banner text for a completed scan/apply.
fn done_text(msg: &str) -> SharedString {
    msg.to_string().into()
}

/// Result text for a row's apply outcome.
fn result_text(result: &ItemResult) -> String {
    match result {
        ItemResult::Pending => String::new(),
        ItemResult::Ok(msg) => format!("✓ {msg}"),
        ItemResult::Err(msg) => format!("✗ {msg}"),
    }
}

// ------------------------------------------------------------ root app ----

/// The whole window: sidebar + top bar + the active module's screen, plus
/// the toast notice and the typed-confirmation dialog for risky tweaks.
#[component]
pub fn WinMoleApp() -> impl IntoElement {
    let ctx = Rc::new(Ctx::new());

    // Drain worker events + refresh the live gauges on a fixed tick. The
    // scan/apply work itself runs on detached worker threads; this only
    // moves their results into signals.
    let drain_ctx = Rc::clone(&ctx);
    let refresh_ctx = Rc::clone(&ctx);
    set_interval(Duration::from_millis(120), move || drain_events(&drain_ctx));
    set_interval(Duration::from_secs(2), move || {
        if refresh_ctx.active.get() == ModuleId::Dashboard {
            refresh_ctx.refresh_stats();
        }
    });

    view()
        .flex_row()
        .flex_1()
        .w_full()
        .bg(move || theme().background)
        .text_color(move || theme().foreground)
        .child(sidebar(Rc::clone(&ctx)))
        .child(main_pane(Rc::clone(&ctx)))
        .child(overlay_notice(Rc::clone(&ctx)))
        .child(overlay_confirm(ctx))
}

/// Left sidebar: app name, module nav, language toggle at the bottom.
fn sidebar(ctx: Rc<Ctx>) -> nsh_core::element::View {
    let active_sig = ctx.active;
    let nav_ctx = Rc::clone(&ctx);
    let buttons = ModuleId::ALL
        .into_iter()
        .map(move |module| {
            let on_press_ctx = Rc::clone(&nav_ctx);
            let active = move || active_sig.get() == module;
            let label = move || i18n::t(match module {
                ModuleId::Dashboard => T::NAV_DASHBOARD,
                ModuleId::Cleanup => T::NAV_CLEANUP,
                ModuleId::Disk => T::NAV_DISK,
                ModuleId::Startup => T::NAV_STARTUP,
                ModuleId::Packages => T::NAV_PACKAGES,
                ModuleId::Registry => T::NAV_REGISTRY,
                ModuleId::Optimize => T::NAV_OPTIMIZE,
            });
            view()
                .w_full()
                .items_center()
                .px(px(12.0))
                .py(px(9.0))
                .rounded(move || theme().radius)
                .bg(move || {
                    if active() {
                        theme().primary
                    } else {
                        theme().sidebar
                    }
                })
                .hover(|s| s.opacity(0.88))
                .on_press(move |_: &PressEvent| on_press_ctx.set_active(module))
                .child(
                    text(label)
                        .text_size(px(13.0))
                        .text_color(move || {
                            if active() {
                                theme().primary_foreground
                            } else {
                                theme().sidebar_foreground
                            }
                        }),
                )
        })
        .collect::<Vec<_>>();

    let toggle_ctx = Rc::clone(&ctx);
    view()
        .flex_col()
        .w(px(180.0))
        .h_full()
        .gap(px(6.0))
        .p(px(12.0))
        .bg(move || theme().sidebar)
        .border_r_1()
        .border_color(move || theme().sidebar_border)
        .child(
            view()
                .flex_col()
                .gap(px(2.0))
                .p(px(6.0))
                .child(
                    text("WinMole")
                        .text_size(px(18.0))
                        .font_weight(FontWeight::BOLD)
                        .text_color(move || theme().sidebar_foreground),
                )
                .child(
                    text(move || i18n::t(T::TAGLINE))
                        .text_size(px(10.0))
                        .text_color(move || theme().muted_foreground),
                ),
        )
        .child(
            text(move || i18n::t(T::MODULES))
                .text_size(px(10.0))
                .px(px(6.0))
                .text_color(move || theme().muted_foreground),
        )
        .children(buttons)
        .child(view().flex_1())
        .child(
            secondary(lang_switch_label(), move || toggle_ctx.toggle_lang())
                .items_center(),
        )
}

/// Label for the language-switch button (read once at build time).
fn lang_switch_label() -> SharedString {
    match i18n::lang().get() {
        Lang::Zh => "English / 涓枃".into(),
        Lang::En => "涓枃 / English".into(),
    }
}

/// The center pane: one screen per module, swapped by `ctx.active`.
fn main_pane(ctx: Rc<Ctx>) -> nsh_core::element::View {
    let active_sig = ctx.active;
    let module = move || active_sig.get();

    let mut actions: Vec<AnyElement> = Vec::new();
    {
        let rescan_ctx = Rc::clone(&ctx);
        actions.push(
            secondary(rescan_label(), move || rescan_ctx.scan_active(true)).into_element(),
        );
    }
    if show_apply(module()) {
        let apply_ctx = Rc::clone(&ctx);
        actions.push(primary(apply_label(), move || apply_ctx.commit_active()).into_element());
    }
    if module() == ModuleId::Cleanup || module() == ModuleId::Packages {
        let sel_ctx = Rc::clone(&ctx);
        actions
            .push(secondary(select_all_label(), move || sel_ctx.select_all_safe()).into_element());
    }

    view()
        .flex_col()
        .flex_1()
        .gap(px(8.0))
        .p(px(16.0))
        .child(
            view()
                .flex_row()
                .w_full()
                .items_center()
                .justify_between()
                .gap(px(8.0))
                .child(
                    text(move || header_title(module()))
                        .text_size(px(16.0))
                        .font_weight(FontWeight::BOLD),
                )
                .child(view().flex_row().gap(px(8.0)).children(actions)),
        )
        .child(view().flex_1().w_full().child(module_screen(ctx)))
}

fn header_title(module: ModuleId) -> SharedString {
    i18n::t(match module {
        ModuleId::Dashboard => T::NAV_DASHBOARD,
        ModuleId::Cleanup => T::CLEANUP_TITLE,
        ModuleId::Disk => T::DISK_TITLE,
        ModuleId::Startup => T::STARTUP_TITLE,
        ModuleId::Packages => T::PACKAGES_TITLE,
        ModuleId::Registry => T::REGISTRY_TITLE,
        ModuleId::Optimize => T::OPTIMIZE_TITLE,
    })
}

fn rescan_label() -> SharedString {
    i18n::t(T::RESCAN)
}

fn apply_label() -> SharedString {
    i18n::t(T::APPLY)
}

fn select_all_label() -> SharedString {
    i18n::t(T::SELECT_ALL)
}

/// The Apply button is meaningless on the dashboard and the disk browser.
fn show_apply(module: ModuleId) -> bool {
    !matches!(module, ModuleId::Dashboard | ModuleId::Disk)
}

// ------------------------------------------------------------- screens ----

fn module_screen(ctx: Rc<Ctx>) -> AnyElement {
    match ctx.active.get() {
        ModuleId::Dashboard => dashboard(ctx).into_element(),
        ModuleId::Cleanup => cleanup_screen(ctx).into_element(),
        ModuleId::Disk => disk_screen(ctx).into_element(),
        ModuleId::Startup => startup_screen(ctx).into_element(),
        ModuleId::Packages => packages_screen(ctx).into_element(),
        ModuleId::Registry => registry_screen(ctx).into_element(),
        ModuleId::Optimize => optimize_screen(ctx).into_element(),
    }
}

fn dashboard(ctx: Rc<Ctx>) -> nsh_core::element::View {
    let stats = ctx.stats;
    let health = ctx.health;
    let rec_a = Rc::clone(&ctx);
    let rec_b = Rc::clone(&ctx);
    view()
        .flex_col()
        .flex_1()
        .w_full()
        .gap(px(12.0))
        .child(
            view()
                .flex_row()
                .w_full()
                .gap(px(12.0))
                .child(metric(cpu_title(), move || {
                    format!("{:.0}%", stats.get().cpu)
                }))
                .child(metric(mem_title(), move || {
                    let s = stats.get();
                    let pct = if s.memory_total > 0 {
                        (s.memory_used as f64 / s.memory_total as f64 * 100.0) as u32
                    } else {
                        0
                    };
                    format!("{pct}% · {}", commands::format_size(s.memory_used))
                }))
                .child(metric(disk_title(), move || {
                    let s = stats.get();
                    let pct = if s.disk_total > 0 {
                        (s.disk_free as f64 / s.disk_total as f64 * 100.0) as u32
                    } else {
                        0
                    };
                    format!("{pct}% {}", i18n::t(T::FREE))
                }))
                .child(metric(uptime_title(), move || {
                    format_uptime(stats.get().uptime)
                })),
        )
        .child(
            view()
                .flex_row()
                .w_full()
                .gap(px(12.0))
                .child(metric(health_title(), move || {
                    match health.get() {
                        Some(h) => format!("{} / 100 · {}", h.score, h.status),
                        None => i18n::t(T::UNKNOWN).to_string(),
                    }
                }))
                .child(metric(cleanable_title(), move || {
                    commands::format_size(cleanable_total(&rec_a))
                })),
        )
        .child(
            card().child(
                text(move || recommendations_text(&rec_b))
                    .text_size(px(12.0))
                    .number_of_lines(8),
            ),
        )
}

fn cpu_title() -> SharedString {
    i18n::t(T::CPU)
}
fn mem_title() -> SharedString {
    i18n::t(T::MEMORY)
}
fn disk_title() -> SharedString {
    i18n::t(T::DISK)
}
fn uptime_title() -> SharedString {
    i18n::t(T::UP)
}
fn health_title() -> SharedString {
    i18n::t(T::HEALTH)
}
fn cleanable_title() -> SharedString {
    i18n::t(T::CLEANABLE)
}

fn recommendations_text(ctx: &Ctx) -> String {
    let mut lines: Vec<String> = Vec::new();
    if let Some(h) = ctx.health.get() {
        if h.recommendations.is_empty() {
            lines.push(i18n::t(T::NO_RECOMMENDATIONS).to_string());
        } else {
            for r in &h.recommendations {
                lines.push(format!("• {r}"));
            }
        }
    }
    let free_pct = {
        let s = ctx.stats.get();
        if s.disk_total > 0 {
            s.disk_free as f64 / s.disk_total as f64
        } else {
            1.0
        }
    };
    if free_pct < 0.15 {
        lines.push(format!("• {}", i18n::t(T::DISK_LOW)));
    }
    if lines.is_empty() {
        lines.push(i18n::t(T::NO_RECOMMENDATIONS).to_string());
    }
    lines.join("\n")
}

fn cleanup_screen(ctx: Rc<Ctx>) -> nsh_core::element::View {
    let rows = ctx
        .cleanup_rows
        .get()
        .into_iter()
        .enumerate()
        .map(|(i, row)| {
            let press_ctx = Rc::clone(&ctx);
            let title = row.target.name.clone();
            let size = row.target.size;
            let admin = row.target.requires_admin;
            let checked = row.checked;
            let result = row.result.clone();
            list_row(
                move || title.clone(),
                move || {
                    format!(
                        "{} · {}",
                        commands::format_size(size),
                        if admin {
                            i18n::t(T::ADMIN).to_string()
                        } else {
                            i18n::t(T::SAFE).to_string()
                        }
                    )
                },
                move || checked,
                move || result_text(&result),
                move || press_ctx.toggle_row(i),
            )
        })
        .collect::<Vec<_>>();

    let mut children: Vec<AnyElement> = Vec::new();
    let phase = ctx.cleanup_phase.get();
    if phase == Phase::Scanning {
        children.push(phase_banner(i18n::t(T::CLEANUP_SCANNING), false).into_element());
    } else if phase == Phase::Error {
        children.push(
            phase_banner(error_text(&ctx.cleanup_error.get()), true).into_element(),
        );
    } else if phase == Phase::Done {
        children.push(
            phase_banner(done_text(&ctx.cleanup_summary.get()), false).into_element(),
        );
    }
    children.push(
        scroll_rows(rows)
            .into_element(),
    );
    children.push(
        text(move || i18n::t(T::FILES_IN_USE_SKIPPED))
            .text_size(px(11.0))
            .text_color(move || theme().muted_foreground)
            .into_element(),
    );

    view()
        .flex_col()
        .flex_1()
        .w_full()
        .gap(px(10.0))
        .children(children)
}

fn disk_screen(ctx: Rc<Ctx>) -> nsh_core::element::View {
    let mut children: Vec<AnyElement> = Vec::new();
    if ctx.disk_loading.get() {
        children.push(phase_banner(i18n::t(T::DISK_SCANNING), false).into_element());
    }
    children.push(
        {
            let path_ctx = Rc::clone(&ctx);
            text(move || path_ctx.disk_path.get().display().to_string())
                .text_size(px(12.0))
                .text_color(move || theme().muted_foreground)
                .into_element()
        },
    );

    let mut rows = Vec::new();
    let entries = ctx.disk_rows.get();
    let total = entries
        .iter()
        .map(|(_, size)| *size)
        .max()
        .unwrap_or(1)
        .max(1);
    for (path, size) in &entries {
        let path_str = path.display().to_string();
        let size_v = *size;
        let pct = (size_v as f64 / total as f64 * 100.0) as u32;
        rows.push(
            view()
                .flex_row()
                .w_full()
                .items_center()
                .gap(px(8.0))
                .px(px(10.0))
                .py(px(4.0))
                .rounded(px(4.0))
                .bg(move || theme().muted)
                .child(
                    view()
                        .h(px(8.0))
                        .w_full()
                        .flex_1()
                        .rounded(px(4.0))
                        .bg(move || theme().secondary)
                        .child(
                            view()
                                .h(px(8.0))
                                .w(move || {
                                    let pct = pct as f32;
                                    px(6.0 + (pct / 100.0) * 200.0)
                                })
                                .rounded(px(4.0))
                                .bg(move || theme().primary),
                        ),
                )
                .child(
                    text(move || path_str.clone())
                        .text_size(px(12.0))
                        .truncate(),
                )
                .child(
                    text(move || commands::format_size(size_v))
                        .text_size(px(11.0))
                        .text_color(move || theme().muted_foreground),
                ),
        );
    }
    children.push(scroll_rows(rows).into_element());

    view()
        .flex_col()
        .flex_1()
        .w_full()
        .gap(px(10.0))
        .children(children)
}

fn staged_count_line(count: usize) -> nsh_core::element::View {
    view().child(
        text(move || {
            format!("{} {} {}", i18n::t(T::SELECTED), count, i18n::t(T::ITEMS))
        })
        .text_size(px(11.0))
        .text_color(move || theme().muted_foreground),
    )
}

fn startup_screen(ctx: Rc<Ctx>) -> nsh_core::element::View {
    let rows = ctx
        .startup_rows
        .get()
        .into_iter()
        .enumerate()
        .map(|(i, row)| {
            let press_ctx = Rc::clone(&ctx);
            let name = row.item.name.clone();
            let category = row.item.category.clone();
            let enabled = row.item.enabled;
            let staged = row.staged;
            list_row(
                move || name.clone(),
                move || category.clone(),
                move || staged,
                move || {
                    if enabled {
                        i18n::t(T::ON).to_string()
                    } else {
                        i18n::t(T::OFF).to_string()
                    }
                },
                move || press_ctx.toggle_row(i),
            )
        })
        .collect::<Vec<_>>();

    let mut children: Vec<AnyElement> = Vec::new();
    let phase = ctx.startup_phase.get();
    if phase == Phase::Scanning {
        children.push(phase_banner(i18n::t(T::STARTUP_SCANNING), false).into_element());
    } else if phase == Phase::Error {
        children.push(phase_banner(error_text(&ctx.startup_summary.get()), true).into_element());
    } else if phase == Phase::Done {
        children.push(phase_banner(done_text(&ctx.startup_summary.get()), false).into_element());
    }
    let staged_startup = ctx.startup_rows.get().iter().filter(|r| r.staged).count();
    children.push(scroll_rows(rows).into_element());
    children.push(staged_count_line(staged_startup).into_element());
    children.push(
        text(i18n::t(T::CHANGES_ON_APPLY))
            .text_size(px(11.0))
            .text_color(move || theme().muted_foreground)
            .into_element(),
    );

    view()
        .flex_col()
        .flex_1()
        .w_full()
        .gap(px(10.0))
        .children(children)
}

fn packages_screen(ctx: Rc<Ctx>) -> nsh_core::element::View {
    let rows = ctx
        .packages_rows
        .get()
        .into_iter()
        .enumerate()
        .map(|(i, row)| {
            let press_ctx = Rc::clone(&ctx);
            let name = row.name.clone();
            let current = row.current.clone();
            let available = row.available.clone();
            let checked = row.checked;
            let result = row.result.clone();
            list_row(
                move || name.clone(),
                move || format!("{} → {}", current, available),
                move || checked,
                move || result_text(&result),
                move || press_ctx.toggle_row(i),
            )
        })
        .collect::<Vec<_>>();

    let mut children: Vec<AnyElement> = Vec::new();
    let phase = ctx.packages_phase.get();
    if phase == Phase::Scanning {
        children.push(phase_banner(i18n::t(T::PACKAGES_SCANNING), false).into_element());
    } else if phase == Phase::Error {
        children.push(phase_banner(error_text(&ctx.packages_error.get()), true).into_element());
    } else if phase == Phase::Done {
        children.push(phase_banner(done_text(&ctx.packages_summary.get()), false).into_element());
    }
    let staged_here = ctx.packages_rows.get().iter().filter(|r| r.checked).count();
    children.push(scroll_rows(rows).into_element());
    children.push(staged_count_line(staged_here).into_element());

    view()
        .flex_col()
        .flex_1()
        .w_full()
        .gap(px(10.0))
        .children(children)
}

fn registry_screen(ctx: Rc<Ctx>) -> nsh_core::element::View {
    let rows = ctx
        .registry_rows
        .get()
        .into_iter()
        .enumerate()
        .map(|(i, row)| {
            let press_ctx = Rc::clone(&ctx);
            let name = row.finding.name.clone();
            let evidence = row.finding.evidence.clone();
            let location = row.finding.location.clone();
            let category = row.finding.category.clone();
            let remediable = row.finding.remediable;
            let checked = row.checked;
            list_row(
                move || name.clone(),
                move || format!("{} · {}", evidence, location),
                move || checked,
                move || {
                    if remediable {
                        format!("{} {}", i18n::t(T::OK), category)
                    } else {
                        i18n::t(T::REVIEW_ONLY).to_string()
                    }
                },
                move || press_ctx.toggle_row(i),
            )
        })
        .collect::<Vec<_>>();

    let mut children: Vec<AnyElement> = Vec::new();
    let phase = ctx.registry_phase.get();
    if phase == Phase::Scanning {
        children.push(phase_banner(i18n::t(T::REGISTRY_SCANNING), false).into_element());
    } else if phase == Phase::Error {
        children.push(phase_banner(error_text(&ctx.registry_error.get()), true).into_element());
    } else if phase == Phase::Done {
        children.push(phase_banner(done_text(&ctx.registry_summary.get()), false).into_element());
    }
    let staged_here = ctx.registry_rows.get().iter().filter(|r| r.checked).count();
    children.push(scroll_rows(rows).into_element());
    children.push(staged_count_line(staged_here).into_element());

    view()
        .flex_col()
        .flex_1()
        .w_full()
        .gap(px(10.0))
        .children(children)
}

fn optimize_screen(ctx: Rc<Ctx>) -> nsh_core::element::View {
    let rows = ctx
        .optimize_rows
        .get()
        .into_iter()
        .enumerate()
        .map(|(i, row)| {
            let press_ctx = Rc::clone(&ctx);
            let title = row.tweak.name.clone();
            let desc = row.tweak.description.clone();
            let state = row.state;
            let desired = row.desired;
            let result = row.result.clone();
            list_row(
                move || title.clone(),
                move || desc.clone(),
                move || desired,
                move || {
                    match &result {
                        ItemResult::Pending => tweak_state_text(&state),
                        r => result_text(r),
                    }
                },
                move || press_ctx.toggle_row(i),
            )
        })
        .collect::<Vec<_>>();

    let mut children: Vec<AnyElement> = Vec::new();
    let phase = ctx.optimize_phase.get();
    if phase == Phase::Scanning {
        children.push(phase_banner(i18n::t(T::OPTIMIZE_SCANNING), false).into_element());
    } else if phase == Phase::Error {
        children.push(phase_banner(error_text(&ctx.optimize_summary.get()), true).into_element());
    } else if phase == Phase::Done {
        children.push(phase_banner(done_text(&ctx.optimize_summary.get()), false).into_element());
    }
    let staged_opt = ctx.optimize_rows.get().iter().filter(|r| r.desired).count();
    children.push(scroll_rows(rows).into_element());
    children.push(staged_count_line(staged_opt).into_element());

    view()
        .flex_col()
        .flex_1()
        .w_full()
        .gap(px(10.0))
        .children(children)
}

fn tweak_state_text(state: &TweakState) -> String {
    match state {
        TweakState::Applied => i18n::t(T::ON).to_string(),
        TweakState::NotApplied => i18n::t(T::OFF).to_string(),
        TweakState::PartiallyApplied => i18n::t(T::PARTIAL).to_string(),
        TweakState::Unknown => i18n::t(T::UNKNOWN).to_string(),
    }
}

// --------------------------------------------------------- scroll helper ----

/// Wrap a pre-built Vec of row views in a scroll view.
fn scroll_rows(rows: Vec<nsh_core::element::View>) -> nsh_core::element::View {
    view()
        .flex_col()
        .flex_1()
        .w_full()
        .gap(px(6.0))
        .child(
            view()
                .flex_col()
                .flex_1()
                .w_full()
                .gap(px(6.0))
                .children(rows),
        )
}

// ------------------------------------------------------------- overlays ----

fn overlay_notice(ctx: Rc<Ctx>) -> nsh_core::element::View {
    let notice_ctx = Rc::clone(&ctx);
    view()
        .absolute()
        .top(px(16.0))
        .right(px(16.0))
        .when(ctx.notice.get().is_some(), move |_| {
            view()
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .px(px(14.0))
                .py(px(10.0))
                .rounded(move || theme().radius)
                .bg(move || theme().popover)
                .child(
                    text(move || {
                        notice_ctx.notice.get().map(|(s, _)| s).unwrap_or_default()
                    })
                    .text_size(px(12.0))
                    .text_color(move || theme().popover_foreground),
                )
        })
}

fn overlay_confirm(ctx: Rc<Ctx>) -> nsh_core::element::View {
    view()
        .absolute()
        .inset_0()
        .when(ctx.confirm_open.get(), |_| {
            let confirm_ctx = Rc::clone(&ctx);
            let confirm_ctx2 = Rc::clone(&ctx);
            let confirm_ctx3 = Rc::clone(&ctx);
            let confirm_ctx4 = Rc::clone(&ctx);
            let cancel_ctx = Rc::clone(&ctx);
            view()
                .flex_col()
                .items_center()
                .justify_center()
                .w_full()
                .h_full()
                .bg(hsla(0.0, 0.0, 0.0, 0.55))
                .child(
                    card()
                        .w(px(360.0))
                        .child(
                            text(move || i18n::t(T::CONFIRM_RISKY_TITLE))
                                .text_size(px(14.0))
                                .font_weight(FontWeight::BOLD),
                        )
                        .child(
                            input()
                                .value(move || confirm_ctx2.confirm_input.get())
                                .placeholder(confirm_placeholder())
                                .on_change(move |s: &str| confirm_ctx4.set_confirm_input(s.to_string()))
                                .on_submit(move |s: &str| confirm_ctx3.confirm_try_apply_typed(s)),
                        )
                        .child(
                            text(move || confirm_text(&ctx))
                                .text_size(px(12.0))
                                .text_color(move || theme().muted_foreground),
                        )
                        .child(
                            view()
                                .flex_row()
                                .justify_end()
                                .gap(px(8.0))
                                .child(secondary(cancel_label(), move || cancel_ctx.confirm_cancel()))
                                .child(
                                    destructive(confirm_apply_label(), move || {
                                        confirm_ctx.confirm_apply()
                                    }),
                                ),
                        ),
                )
        })
}

fn confirm_text(ctx: &Ctx) -> String {
    let count = ctx
        .pending_changes
        .get()
        .iter()
        .filter(|(_, _, desired)| *desired)
        .count();
    format!("{} {count}", i18n::t(T::CONFIRM_RISKY_MSG))
}

fn cancel_label() -> SharedString {
    i18n::t(T::CANCEL)
}

fn confirm_placeholder() -> String {
    "APPLY".to_string()
}

fn confirm_apply_label() -> SharedString {
    i18n::t(T::APPLY)
}
