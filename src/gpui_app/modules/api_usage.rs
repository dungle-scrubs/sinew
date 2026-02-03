//! API usage/costs module for LLM providers.

use gpui::{div, prelude::*, px, AnyElement, SharedString, Styled, TextAlign};
use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, RwLock,
};
use std::time::{Duration, Instant};

use crate::gpui_app::theme::Theme;

#[derive(Clone, Debug, Default)]
struct ApiUsageRow {
    name: String,
    value: String,
    period: String,
}

/// API usage module showing provider costs in a popup panel.
pub struct ApiUsageModule {
    id: String,
    theme: Option<Theme>,
    rows: Arc<RwLock<Vec<ApiUsageRow>>>,
    dirty: Arc<AtomicBool>,
    loading: Arc<AtomicBool>,
    update_interval: Duration,
    last_update: Instant,
    stop: Arc<AtomicBool>,
}

impl ApiUsageModule {
    /// Panel height baseline for popup content.
    const ROW_HEIGHT: f64 = 20.0;
    const HEADER_HEIGHT: f64 = 24.0;
    const ROW_GAP: f64 = 6.0;
    const CONTAINER_PADDING_TOP: f64 = 16.0;
    const CONTAINER_PADDING_BOTTOM: f64 = 24.0;

    pub fn new(id: &str) -> Self {
        let rows = Arc::new(RwLock::new(Vec::new()));
        let dirty = Arc::new(AtomicBool::new(true));
        let loading = Arc::new(AtomicBool::new(true));
        let stop = Arc::new(AtomicBool::new(false));
        let mut module = Self {
            id: id.to_string(),
            theme: None,
            rows,
            dirty,
            loading,
            update_interval: Duration::from_secs(1800),
            last_update: Instant::now() - Duration::from_secs(1801),
            stop,
        };
        module.spawn_updater();
        module
    }

    pub fn new_popup(theme: Theme) -> Self {
        let rows = Arc::new(RwLock::new(Vec::new()));
        let dirty = Arc::new(AtomicBool::new(true));
        let loading = Arc::new(AtomicBool::new(true));
        let stop = Arc::new(AtomicBool::new(false));
        let mut module = Self {
            id: "api_usage".to_string(),
            theme: Some(theme),
            rows,
            dirty,
            loading,
            update_interval: Duration::from_secs(1800),
            last_update: Instant::now() - Duration::from_secs(1801),
            stop,
        };
        module.spawn_updater();
        module
    }

    fn spawn_updater(&mut self) {
        let rows = self.rows.clone();
        let dirty = self.dirty.clone();
        let loading = self.loading.clone();
        let interval = self.update_interval;
        let stop = self.stop.clone();

        std::thread::spawn(move || loop {
            if stop.load(Ordering::Relaxed) {
                break;
            }
            let data = fetch_api_usage_rows();
            if let Ok(mut guard) = rows.write() {
                *guard = data;
            }
            loading.store(false, Ordering::SeqCst);
            dirty.store(true, Ordering::SeqCst);
            std::thread::sleep(interval);
        });
    }

    fn estimated_height(&self, rows: usize) -> f64 {
        let rows = rows.max(1) as f64;
        Self::CONTAINER_PADDING_TOP
            + Self::CONTAINER_PADDING_BOTTOM
            + Self::HEADER_HEIGHT
            + Self::ROW_GAP
            + rows * Self::ROW_HEIGHT
            + (rows - 1.0) * Self::ROW_GAP
    }
}

fn run_shell_with_timeout(script: &str, timeout: Duration) -> String {
    let mut child = match Command::new("sh")
        .arg("-c")
        .arg(script)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(child) => child,
        Err(_) => return String::new(),
    };

    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_status)) => {
                let mut output = String::new();
                if let Some(mut stdout) = child.stdout.take() {
                    let _ = stdout.read_to_string(&mut output);
                }
                return output.trim().to_string();
            }
            Ok(None) => {
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    return String::new();
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(_) => return String::new(),
        }
    }
}

fn fetch_api_usage_rows() -> Vec<ApiUsageRow> {
    let anthropic = run_shell_with_timeout(
        r#"
export OP_SERVICE_ACCOUNT_TOKEN=$(security find-generic-password -a dev-secrets -s OP_SERVICE_ACCOUNT_TOKEN -w 2>/dev/null)
if [ -z "$OP_SERVICE_ACCOUNT_TOKEN" ]; then echo "0"; exit 0; fi
KEY=$(op read "op://Models/anthropic/admin-key" 2>/dev/null)
if [ -z "$KEY" ]; then echo "0"; exit 0; fi
START=$(date -v-30d -u +%Y-%m-%dT00:00:00Z)
curl -s "https://api.anthropic.com/v1/organizations/cost_report?starting_at=$START&bucket_width=1d&limit=31" \
  -H "x-api-key: $KEY" \
  -H "anthropic-version: 2023-06-01" \
  -H "Content-Type: application/json" 2>/dev/null \
| jq '[.data[].amount] | add // 0'
"#,
        Duration::from_secs(10),
    );
    let anthropic_cost = anthropic.parse::<f64>().unwrap_or(0.0);

    let openai = run_shell_with_timeout(
        r#"
export OP_SERVICE_ACCOUNT_TOKEN=$(security find-generic-password -a dev-secrets -s OP_SERVICE_ACCOUNT_TOKEN -w 2>/dev/null)
if [ -z "$OP_SERVICE_ACCOUNT_TOKEN" ]; then echo "0"; exit 0; fi
KEY=$(op read "op://Models/openai/admin-key" 2>/dev/null)
if [ -z "$KEY" ]; then echo "0"; exit 0; fi
START=$(date -v-30d +%s)
END=$(date +%s)
curl -s "https://api.openai.com/v1/organization/costs?start_time=$START&end_time=$END&bucket_width=1d" \
  -H "Authorization: Bearer $KEY" \
  -H "Content-Type: application/json" 2>/dev/null \
| jq '[.data[].results[].amount.value] | add // 0'
"#,
        Duration::from_secs(10),
    );
    let openai_cost = openai.parse::<f64>().unwrap_or(0.0);

    let openrouter = run_shell_with_timeout(
        r#"
export OP_SERVICE_ACCOUNT_TOKEN=$(security find-generic-password -a dev-secrets -s OP_SERVICE_ACCOUNT_TOKEN -w 2>/dev/null)
if [ -z "$OP_SERVICE_ACCOUNT_TOKEN" ]; then echo "0"; exit 0; fi
KEY=$(op read "op://Models/openrouter/api-key" 2>/dev/null)
if [ -z "$KEY" ]; then echo "0"; exit 0; fi
curl -s "https://openrouter.ai/api/v1/credits" \
  -H "Authorization: Bearer $KEY" 2>/dev/null \
| jq '(.data.total_credits - .data.total_usage) // 0'
"#,
        Duration::from_secs(10),
    );
    let openrouter_bal = openrouter.parse::<f64>().unwrap_or(0.0);

    let codex = run_shell_with_timeout(
        r#"
LATEST=$(find ~/.codex/sessions -name "rollout-*.jsonl" -type f 2>/dev/null | sort -r | head -1)
if [ -z "$LATEST" ]; then echo "0"; exit 0; fi
grep '"token_count"' "$LATEST" 2>/dev/null | tail -1 | jq -r '.payload.rate_limits.primary.used_percent // 0'
"#,
        Duration::from_secs(10),
    );
    let codex_pct = codex.parse::<f64>().unwrap_or(0.0);

    vec![
        ApiUsageRow {
            name: "Anthropic".to_string(),
            value: format!("${:.2}", anthropic_cost),
            period: "30d".to_string(),
        },
        ApiUsageRow {
            name: "Codex".to_string(),
            value: format!("{:.0}%", codex_pct),
            period: "session".to_string(),
        },
        ApiUsageRow {
            name: "MiniMax".to_string(),
            value: "-".to_string(),
            period: "no plan".to_string(),
        },
        ApiUsageRow {
            name: "OpenAI".to_string(),
            value: format!("${:.2}", openai_cost),
            period: "30d".to_string(),
        },
        ApiUsageRow {
            name: "OpenRouter".to_string(),
            value: format!("${:.2}", openrouter_bal),
            period: "bal".to_string(),
        },
        ApiUsageRow {
            name: "Z.ai".to_string(),
            value: "-".to_string(),
            period: "no bal".to_string(),
        },
    ]
}

impl super::GpuiModule for ApiUsageModule {
    fn id(&self) -> &str {
        &self.id
    }

    fn render(&self, theme: &Theme) -> AnyElement {
        div()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(6.0))
            .child(
                div()
                    .text_size(px(14.0))
                    .text_color(theme.accent)
                    .child(SharedString::from("󰧑")),
            )
            .child(
                div()
                    .text_size(px(13.0))
                    .text_color(theme.foreground)
                    .child(SharedString::from("AI")),
            )
            .into_any_element()
    }

    fn update(&mut self) -> bool {
        if self.dirty.swap(false, Ordering::SeqCst) {
            self.last_update = Instant::now();
            return true;
        }
        if self.last_update.elapsed() > self.update_interval {
            self.dirty.store(true, Ordering::SeqCst);
            return true;
        }
        false
    }

    fn is_loading(&self) -> bool {
        self.loading.load(Ordering::SeqCst)
    }

    fn popup_spec(&self) -> Option<super::PopupSpec> {
        if self.theme.is_some() {
            let count = self.rows.read().ok().map(|r| r.len()).unwrap_or(0);
            Some(
                super::PopupSpec::new(320.0, self.estimated_height(count))
                    .with_anchor(super::PopupAnchor::Center),
            )
        } else {
            None
        }
    }

    fn render_popup(&self, theme: &Theme) -> Option<AnyElement> {
        if self.theme.is_none() {
            return None;
        }

        let rows = self
            .rows
            .read()
            .ok()
            .map(|rows| rows.clone())
            .unwrap_or_default();
        let rows = if rows.is_empty() {
            vec![ApiUsageRow {
                name: "Loading".to_string(),
                value: "...".to_string(),
                period: "".to_string(),
            }]
        } else {
            rows
        };

        let mut container = div()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .px(px(16.0))
            .pt(px(16.0))
            .pb(px(24.0))
            .w_full()
            .child(
                div()
                    .text_color(theme.foreground)
                    .text_size(px(18.0))
                    .font_weight(gpui::FontWeight::BOLD)
                    .child("API Usage"),
            );

        for row in rows {
            container = container.child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .gap(px(12.0))
                    .child(
                        div()
                            .w(px(140.0))
                            .text_color(theme.foreground)
                            .text_size(px(13.0))
                            .child(SharedString::from(row.name)),
                    )
                    .child(
                        div()
                            .w(px(90.0))
                            .text_align(TextAlign::Right)
                            .text_color(theme.foreground)
                            .text_size(px(13.0))
                            .child(SharedString::from(row.value)),
                    )
                    .child(
                        div()
                            .w(px(70.0))
                            .text_align(TextAlign::Right)
                            .text_color(theme.foreground_muted)
                            .text_size(px(12.0))
                            .child(SharedString::from(row.period)),
                    ),
            );
        }

        Some(container.into_any_element())
    }
}

impl Drop for ApiUsageModule {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}
