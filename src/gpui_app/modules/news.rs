//! News module with release notes popup.
//!
//! This module provides:
//! - Bar item: News icon with count badge
//! - Popup: Full-width panel showing release notes from configured sources

use std::process::Command;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use gpui::{div, prelude::*, px, AnyElement, ParentElement, SharedString, Styled};

use super::{GpuiModule, PopupSpec};
use crate::gpui_app::theme::Theme;

/// A single release entry.
#[derive(Debug, Clone)]
pub struct ReleaseEntry {
    pub section: String,
    pub text: String,
}

/// A parsed release with version and entries.
#[derive(Debug, Clone)]
pub struct Release {
    pub version: String,
    pub items: Vec<ReleaseEntry>,
}

/// Configuration for a release source.
#[derive(Debug, Clone)]
pub struct ReleaseSource {
    pub name: String,
    pub url: String,
    pub icon: &'static str,
    pub parse_mode: ParseMode,
}

/// How to parse the release content.
#[derive(Debug, Clone, Copy)]
pub enum ParseMode {
    /// Look for "- Added ..." lines under ## version headers (Claude Code style)
    Added,
    /// Parse GitHub Releases API JSON
    GitHubRelease,
}

/// Cached release data for all sources.
#[derive(Debug, Clone, Default)]
pub struct ReleasesData {
    pub sources: Vec<(ReleaseSource, Vec<Release>)>,
    pub total_items: usize,
}

/// News module showing release updates.
pub struct NewsModule {
    id: String,
    theme: Option<Theme>,
    update_interval: Duration,
    last_update: Instant,
    data: Arc<RwLock<Option<ReleasesData>>>,
    is_loading: bool,
}

impl NewsModule {
    /// Creates a simple bar-only news module (for config-based creation).
    pub fn new(id: &str) -> Self {
        let mut module = Self {
            id: id.to_string(),
            theme: None,
            update_interval: Duration::from_secs(3600), // 1 hour
            last_update: Instant::now() - Duration::from_secs(3601),
            data: Arc::new(RwLock::new(None)),
            is_loading: true,
        };
        module.fetch_releases();
        module
    }

    /// Creates a news module with popup support.
    pub fn new_popup(theme: Theme) -> Self {
        let mut module = Self {
            id: "news".to_string(),
            theme: Some(theme),
            update_interval: Duration::from_secs(3600), // 1 hour
            last_update: Instant::now() - Duration::from_secs(3601),
            data: Arc::new(RwLock::new(None)),
            is_loading: true,
        };
        module.fetch_releases();
        module
    }

    /// Panel height for news content.
    const PANEL_HEIGHT: f64 = 280.0;

    /// Returns the configured release sources.
    fn sources() -> Vec<ReleaseSource> {
        vec![
            ReleaseSource {
                name: "Claude Code".to_string(),
                url: "https://raw.githubusercontent.com/anthropics/claude-code/main/CHANGELOG.md"
                    .to_string(),
                icon: "󰧑",
                parse_mode: ParseMode::Added,
            },
            ReleaseSource {
                name: "Codex".to_string(),
                url: "https://api.github.com/repos/openai/codex/releases/latest".to_string(),
                icon: "",
                parse_mode: ParseMode::GitHubRelease,
            },
            ReleaseSource {
                name: "OpenClaw".to_string(),
                url: "https://api.github.com/repos/openclaw/openclaw/releases/latest".to_string(),
                icon: "🦀",
                parse_mode: ParseMode::GitHubRelease,
            },
        ]
    }

    /// Fetches releases from all sources.
    fn fetch_releases(&mut self) {
        self.is_loading = true;
        let data = Arc::clone(&self.data);

        std::thread::spawn(move || {
            let sources = Self::sources();
            let mut results: Vec<(ReleaseSource, Vec<Release>)> = Vec::new();
            let mut total_items = 0;

            for source in sources {
                let releases = match source.parse_mode {
                    ParseMode::Added => Self::fetch_added_style(&source.url),
                    ParseMode::GitHubRelease => Self::fetch_github_release(&source.url),
                };
                total_items += releases.iter().map(|r| r.items.len()).sum::<usize>();
                results.push((source, releases));
            }

            let releases_data = ReleasesData {
                sources: results,
                total_items,
            };

            if let Ok(mut guard) = data.write() {
                *guard = Some(releases_data);
            }
        });
    }

    /// Fetches and parses CHANGELOG.md for "- Added ..." entries.
    fn fetch_added_style(url: &str) -> Vec<Release> {
        let output = Command::new("curl")
            .args(["-s", "-m", "10", url])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok());

        let Some(content) = output else {
            return Vec::new();
        };

        Self::parse_added_style(&content, 2)
    }

    /// Parses CHANGELOG.md for version headers and "- Added ..." lines.
    fn parse_added_style(content: &str, max_releases: usize) -> Vec<Release> {
        let mut results = Vec::new();
        let mut current_version: Option<String> = None;
        let mut current_items: Vec<ReleaseEntry> = Vec::new();

        for line in content.lines() {
            if let Some(version) = line.strip_prefix("## ") {
                if let Some(ver) = current_version.take() {
                    if !current_items.is_empty() {
                        results.push(Release {
                            version: ver,
                            items: std::mem::take(&mut current_items),
                        });
                        if results.len() >= max_releases {
                            break;
                        }
                    }
                }
                let version = version.split_whitespace().next().unwrap_or(version);
                current_version = Some(version.to_string());
            } else if current_version.is_some() {
                if let Some(rest) = line.strip_prefix("- Added ") {
                    current_items.push(ReleaseEntry {
                        section: "Added".to_string(),
                        text: rest.to_string(),
                    });
                } else if let Some(rest) = line.strip_prefix("- added ") {
                    current_items.push(ReleaseEntry {
                        section: "Added".to_string(),
                        text: rest.to_string(),
                    });
                }
            }
        }

        if let Some(ver) = current_version {
            if !current_items.is_empty() && results.len() < max_releases {
                results.push(Release {
                    version: ver,
                    items: current_items,
                });
            }
        }

        results
    }

    /// Fetches and parses GitHub Releases API.
    fn fetch_github_release(url: &str) -> Vec<Release> {
        let output = Command::new("curl")
            .args([
                "-s",
                "-m",
                "10",
                "-H",
                "Accept: application/vnd.github.v3+json",
                url,
            ])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok());

        let Some(content) = output else {
            return Vec::new();
        };

        Self::parse_github_release(&content)
    }

    /// Parses GitHub release JSON for tag_name and body.
    fn parse_github_release(content: &str) -> Vec<Release> {
        let tag_name = content
            .find("\"tag_name\"")
            .and_then(|i| {
                let rest = &content[i..];
                let start = rest.find(':')?;
                let rest = &rest[start + 1..];
                let start = rest.find('"')? + 1;
                let rest = &rest[start..];
                let end = rest.find('"')?;
                Some(rest[..end].to_string())
            })
            .unwrap_or_default();

        let body = content
            .find("\"body\"")
            .and_then(|i| {
                let rest = &content[i..];
                let start = rest.find(':')?;
                let rest = &rest[start + 1..];
                let start = rest.find('"')? + 1;
                let rest = &rest[start..];
                let mut end = 0;
                let mut escaped = false;
                for (i, c) in rest.char_indices() {
                    if escaped {
                        escaped = false;
                        continue;
                    }
                    if c == '\\' {
                        escaped = true;
                        continue;
                    }
                    if c == '"' {
                        end = i;
                        break;
                    }
                }
                Some(
                    rest[..end]
                        .replace("\\n", "\n")
                        .replace("\\r", "")
                        .replace("\\\"", "\""),
                )
            })
            .unwrap_or_default();

        if tag_name.is_empty() {
            return Vec::new();
        }

        let mut items = Vec::new();
        let mut current_section: Option<String> = None;
        let wanted_sections = [
            "New Features",
            "Bug Fixes",
            "Features",
            "Fixes",
            "Changes",
            "Added",
        ];

        for line in body.lines() {
            if let Some(section) = line
                .strip_prefix("## ")
                .or_else(|| line.strip_prefix("### "))
            {
                let section = section.trim();
                if wanted_sections
                    .iter()
                    .any(|s| section.eq_ignore_ascii_case(s))
                {
                    current_section = Some(section.to_string());
                } else {
                    current_section = None;
                }
            } else if let Some(ref section) = current_section {
                if let Some(text) = line.strip_prefix("- ") {
                    if !text.starts_with('#') && items.len() < 10 {
                        items.push(ReleaseEntry {
                            section: section.clone(),
                            text: text.to_string(),
                        });
                    }
                }
            }
        }

        if items.is_empty() {
            return Vec::new();
        }

        let version = tag_name
            .trim_start_matches('v')
            .trim_start_matches("rust-v")
            .to_string();

        vec![Release { version, items }]
    }

    /// Returns the current release data.
    fn get_data(&self) -> Option<ReleasesData> {
        self.data.read().ok().and_then(|guard| guard.clone())
    }

    /// Renders a single source column.
    fn render_source_column(
        &self,
        theme: &Theme,
        name: &str,
        icon: &str,
        releases: Vec<Release>,
    ) -> gpui::Div {
        let name_str: SharedString = name.to_string().into();
        let icon_str: SharedString = icon.to_string().into();

        let mut column = div()
            .flex()
            .flex_col()
            .flex_1()
            .min_w(px(0.0))
            .overflow_hidden()
            .gap(px(8.0))
            .p(px(12.0))
            .rounded(px(6.0))
            .bg(theme.surface)
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(6.0))
                    .child(
                        div()
                            .text_color(theme.accent)
                            .text_size(px(14.0))
                            .child(icon_str),
                    )
                    .child(
                        div()
                            .text_color(theme.foreground)
                            .text_size(px(13.0))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child(name_str),
                    ),
            );

        if releases.is_empty() {
            return column.child(
                div()
                    .text_color(theme.foreground_muted)
                    .text_size(px(11.0))
                    .child("No releases"),
            );
        }

        for release in releases.into_iter().take(1) {
            let version_str: SharedString = format!("v{}", release.version).into();
            let mut release_div = div().flex().flex_col().gap(px(4.0)).child(
                div()
                    .text_color(theme.accent)
                    .text_size(px(11.0))
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .child(version_str),
            );

            for item in release.items.into_iter().take(6) {
                let text: SharedString = if item.text.chars().count() > 60 {
                    format!("{}...", item.text.chars().take(57).collect::<String>()).into()
                } else {
                    item.text.into()
                };
                release_div = release_div.child(
                    div()
                        .flex()
                        .flex_row()
                        .gap(px(4.0))
                        .child(
                            div()
                                .text_color(theme.success)
                                .text_size(px(11.0))
                                .child("+"),
                        )
                        .child(
                            div()
                                .text_color(theme.foreground)
                                .text_size(px(11.0))
                                .child(text),
                        ),
                );
            }
            column = column.child(release_div);
        }

        column
    }
}

impl GpuiModule for NewsModule {
    fn id(&self) -> &str {
        &self.id
    }

    fn render(&self, theme: &Theme) -> AnyElement {
        let data = self.get_data();
        let count = data.as_ref().map(|d| d.total_items).unwrap_or(0);

        let label = if self.is_loading && data.is_none() {
            "...".to_string()
        } else if count > 0 {
            format!("{}", count)
        } else {
            "News".to_string()
        };

        div()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(6.0)) // Gap between icon and text
            .child(
                div()
                    .text_size(px(14.0))
                    .text_color(theme.accent)
                    .child(SharedString::from("󰋼")),
            )
            .child(
                div()
                    .text_size(px(13.0))
                    .text_color(theme.foreground)
                    .child(SharedString::from(label)),
            )
            .into_any_element()
    }

    fn update(&mut self) -> bool {
        // Check if loading completed
        if self.is_loading {
            if let Ok(guard) = self.data.read() {
                if guard.is_some() {
                    self.is_loading = false;
                    self.last_update = Instant::now();
                    return true;
                }
            }
        }

        // Check if we need to refresh
        if self.last_update.elapsed() > self.update_interval {
            self.fetch_releases();
            return true;
        }

        false
    }

    fn is_loading(&self) -> bool {
        self.is_loading
    }

    fn popup_spec(&self) -> Option<PopupSpec> {
        if self.theme.is_some() {
            Some(PopupSpec::panel(Self::PANEL_HEIGHT))
        } else {
            None
        }
    }

    fn render_popup(&self, theme: &Theme) -> Option<AnyElement> {
        if self.theme.is_none() {
            log::debug!("NewsModule::render_popup: theme is None, returning None");
            return None;
        }

        let news_data = self.get_data();
        log::debug!(
            "NewsModule::render_popup: has_data={}, sources={}",
            news_data.is_some(),
            news_data.as_ref().map(|d| d.sources.len()).unwrap_or(0)
        );

        let mut container = div()
            .flex()
            .flex_col()
            .gap(px(12.0))
            .p(px(16.0))
            .w_full()
            .child(
                div()
                    .text_color(theme.foreground)
                    .text_size(px(16.0))
                    .font_weight(gpui::FontWeight::BOLD)
                    .child("Release Notes"),
            );

        if let Some(data) = news_data {
            let columns = div().flex().flex_row().gap(px(12.0)).w_full().children(
                data.sources.into_iter().map(|(source, releases)| {
                    self.render_source_column(theme, &source.name, source.icon, releases)
                }),
            );
            container = container.child(columns);
        } else {
            container = container.child(
                div()
                    .text_color(theme.foreground_muted)
                    .text_size(px(12.0))
                    .child("Loading..."),
            );
        }

        Some(container.into_any_element())
    }
}
