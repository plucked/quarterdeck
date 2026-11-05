use gpui::{App, IntoElement, div, prelude::*};
use qc_cache::CacheTtl;
use qc_github::{ApiUsageSnapshot, RateLimit};

use super::setting_row::SettingRow;
use crate::app_services::AppServices;
use crate::app_settings::AppSettings;
use crate::segmented_control::SegmentedControl;
use crate::theme;

/// The GitHub page: how long cached data is kept, clearing it, and how much
/// of the API this session used.
pub fn render_github_page(
    has_cleared_cache: bool,
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
            CacheTtl::ALL.into_iter().fold(SegmentedControl::new("cache-ttl"), |control, option| {
                control.option(ttl_label(option), option == ttl, move |_, cx| {
                    AppSettings::set_cache_ttl(cx, option);
                })
            }),
        ))
        .child(SettingRow::new(
            "Cached data",
            "Removes every cached pull request and file; they are fetched again when needed. Viewed marks stay.",
            SegmentedControl::new("clear-cache").option(
                if has_cleared_cache { "Cleared" } else { "Clear cache" },
                has_cleared_cache,
                on_clear_cache,
            ),
        ))
        .child(SettingRow::new(
            "API usage",
            usage.map_or_else(|| "Not connected to GitHub.".to_owned(), describe_usage),
            div().text_color(theme::muted()).child("since launch"),
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
