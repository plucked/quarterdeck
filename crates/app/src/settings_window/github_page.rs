use gpui::{App, IntoElement, SharedString, div, prelude::*};
use qd_cache::{CacheTtl, CacheUsage, CachedData};
use qd_github::{ApiUsageSnapshot, RateLimit};

use super::setting_row::SettingRow;
use crate::app_services::AppServices;
use crate::app_settings::AppSettings;
use crate::button::Button;
use crate::display_size::byte_size;
use crate::segmented_control::SegmentedControl;
use crate::theme;

/// Width of a breakdown line's name column.
const BREAKDOWN_NAME_WIDTH: f32 = 160.;

/// The GitHub page: how long cached data is kept, what the cache holds and
/// clearing it, and how much of the API this session used.
pub fn render_github_page(
    has_cleared_cache: bool,
    cache_usage: Option<CacheUsage>,
    on_clear_cache: impl Fn(&mut gpui::Window, &mut App) + 'static,
    cx: &App,
) -> impl IntoElement {
    let ttl = AppSettings::cache_ttl(cx);
    let usage = cx
        .global::<AppServices>()
        .github
        .as_ref()
        .ok()
        .map(|client| client.usage().snapshot());

    div()
        .flex()
        .flex_col()
        .child(SettingRow::new(
            "Keep cached data for",
            "Pull requests and files not used for this long are removed from the local cache.",
            CacheTtl::ALL.into_iter().fold(
                SegmentedControl::new("cache-ttl"),
                |control, option| {
                    control.option(ttl_label(option), option == ttl, move |_, cx| {
                        AppSettings::set_cache_ttl(cx, option);
                    })
                },
            ),
        ))
        .child(SettingRow::new(
            "Cached data",
            describe_disk_use(cache_usage),
            Button::new(
                "clear-cache",
                if has_cleared_cache {
                    "Cleared"
                } else {
                    "Clear cache"
                },
            )
            .disabled(has_cleared_cache)
            .on_click(move |_, window, cx| on_clear_cache(window, cx)),
        ))
        .children(cache_usage.map(render_cache_breakdown))
        .child(SettingRow::new(
            "API usage",
            usage.map_or_else(|| "Not connected to GitHub.".to_owned(), describe_usage),
            div().text_color(theme::muted()).child("since launch"),
        ))
}

/// The database's size on disk and what clearing does.
fn describe_disk_use(usage: Option<CacheUsage>) -> String {
    let clearing = "Clearing removes every cached pull request and file (fetched again when needed) and shrinks the file; viewed marks and notes stay.";
    match usage {
        None => clearing.to_owned(),
        Some(usage) if usage.free_bytes > 0 => format!(
            "{} on disk, {} of it free space. {clearing}",
            byte_size(usage.disk_bytes),
            byte_size(usage.free_bytes)
        ),
        Some(usage) => format!("{} on disk. {clearing}", byte_size(usage.disk_bytes)),
    }
}

/// What the cache holds, one line per kind: name, how many, size.
fn render_cache_breakdown(usage: CacheUsage) -> impl IntoElement {
    let line = |name: &'static str, count: SharedString, data: CachedData| {
        div()
            .flex()
            .gap_3()
            .py_1()
            .text_xs()
            .child(div().w(gpui::px(BREAKDOWN_NAME_WIDTH)).child(name))
            .child(div().flex_1().text_color(theme::muted()).child(count))
            .child(
                div()
                    .text_color(theme::muted())
                    .child(byte_size(data.bytes)),
            )
    };
    div()
        .flex()
        .flex_col()
        .py_2()
        .border_b_1()
        .border_color(theme::border())
        .child(line(
            "Pull request files",
            format!(
                "{} pull requests · {} files",
                usage.pull_request_files.entries, usage.pull_request_files.files
            )
            .into(),
            usage.pull_request_files,
        ))
        .child(line(
            "Combined stack diffs",
            format!(
                "{} ranges · {} files",
                usage.comparisons.entries, usage.comparisons.files
            )
            .into(),
            usage.comparisons,
        ))
        .child(line(
            "Pull requests",
            format!("{} pull requests", usage.pull_requests.entries).into(),
            usage.pull_requests,
        ))
}

/// The label of a TTL option.
fn ttl_label(ttl: CacheTtl) -> &'static str {
    match ttl {
        CacheTtl::SixHours => "6 hours",
        CacheTtl::OneDay => "1 day",
        CacheTtl::ThreeDays => "3 days",
        CacheTtl::OneWeek => "1 week",
    }
}

/// Requests sent and budgets left, in one line.
fn describe_usage(usage: ApiUsageSnapshot) -> String {
    let budget = |name: &str, limit: Option<RateLimit>| {
        limit.map_or_else(
            || format!("{name}: not used yet"),
            |limit| format!("{name}: {} of {} left", limit.remaining, limit.limit),
        )
    };
    format!(
        "{} requests, {} answered unchanged at no cost · {} · {} · {}",
        usage.requests,
        usage.not_modified,
        budget("core", usage.core),
        budget("search", usage.search),
        budget("GraphQL", usage.graphql)
    )
}
