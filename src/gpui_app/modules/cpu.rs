//! CPU module for displaying CPU usage via Mach host_statistics.

use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::Arc;
use std::time::Duration;

use gpui::{div, prelude::*, px, AnyElement, SharedString, Styled};

use super::{GpuiModule, LabelAlign};
use crate::gpui_app::theme::Theme;

/// Mach host_statistics FFI for CPU ticks (no process spawn needed).
mod mach_cpu {
    use std::ffi::c_uint;
    use std::mem::MaybeUninit;

    const HOST_CPU_LOAD_INFO: c_uint = 3;
    const CPU_STATE_USER: usize = 0;
    const CPU_STATE_SYSTEM: usize = 1;
    const CPU_STATE_IDLE: usize = 2;
    const CPU_STATE_NICE: usize = 3;
    const CPU_STATE_MAX: usize = 4;

    #[repr(C)]
    struct HostCpuLoadInfo {
        cpu_ticks: [u32; CPU_STATE_MAX],
    }

    extern "C" {
        fn mach_host_self() -> c_uint;
        fn host_statistics(
            host: c_uint,
            flavor: c_uint,
            info: *mut HostCpuLoadInfo,
            count: *mut c_uint,
        ) -> c_uint;
    }

    /// Returns cumulative (active_ticks, total_ticks).
    pub fn cpu_ticks() -> Option<(u64, u64)> {
        unsafe {
            let mut info = MaybeUninit::<HostCpuLoadInfo>::uninit();
            let mut count =
                (std::mem::size_of::<HostCpuLoadInfo>() / std::mem::size_of::<u32>()) as c_uint;
            let status = host_statistics(
                mach_host_self(),
                HOST_CPU_LOAD_INFO,
                info.as_mut_ptr(),
                &mut count,
            );
            if status != 0 {
                return None;
            }
            let info = info.assume_init();
            let user = info.cpu_ticks[CPU_STATE_USER] as u64;
            let system = info.cpu_ticks[CPU_STATE_SYSTEM] as u64;
            let idle = info.cpu_ticks[CPU_STATE_IDLE] as u64;
            let nice = info.cpu_ticks[CPU_STATE_NICE] as u64;
            let total = user + system + idle + nice;
            let active = user + system + nice;
            Some((active, total))
        }
    }
}

/// CPU module that displays CPU usage percentage.
pub struct CpuModule {
    id: String,
    label: Option<String>,
    label_align: LabelAlign,
    fixed_width: bool,
    usage: Arc<AtomicU8>,
    dirty: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
}

impl CpuModule {
    /// Creates a new CPU module.
    pub fn new(id: &str, label: Option<&str>, label_align: LabelAlign, fixed_width: bool) -> Self {
        let usage = Arc::new(AtomicU8::new(0));
        let dirty = Arc::new(AtomicBool::new(true));
        let stop = Arc::new(AtomicBool::new(false));

        let usage_handle = Arc::clone(&usage);
        let dirty_handle = Arc::clone(&dirty);
        let stop_handle = Arc::clone(&stop);
        std::thread::spawn(move || {
            let mut prev_ticks: Option<(u64, u64)> = None;
            let mut last = 0u8;
            while !stop_handle.load(Ordering::Relaxed) {
                if let Some(current) = mach_cpu::cpu_ticks() {
                    if let Some(prev) = prev_ticks {
                        let d_active = current.0.saturating_sub(prev.0);
                        let d_total = current.1.saturating_sub(prev.1);
                        let pct = if d_total > 0 {
                            ((d_active as f64 / d_total as f64) * 100.0).round() as u8
                        } else {
                            0
                        };
                        if pct != last {
                            usage_handle.store(pct, Ordering::Relaxed);
                            dirty_handle.store(true, Ordering::Relaxed);
                            last = pct;
                        }
                    }
                    prev_ticks = Some(current);
                }
                std::thread::sleep(Duration::from_secs(2));
            }
        });

        Self {
            id: id.to_string(),
            label: label.map(|s| s.to_string()),
            label_align,
            fixed_width,
            usage,
            dirty,
            stop,
        }
    }
}

impl GpuiModule for CpuModule {
    fn id(&self) -> &str {
        &self.id
    }

    fn render(&self, theme: &Theme) -> AnyElement {
        let usage = self.usage.load(Ordering::Relaxed);
        let text = format!("{}%", usage);

        if let Some(ref label) = self.label {
            // Two-line layout with label - configurable alignment
            let mut container = div().flex().flex_col().gap(px(0.0));

            // Apply alignment
            container = match self.label_align {
                LabelAlign::Left => container.items_start(),
                LabelAlign::Center => container.items_center(),
                LabelAlign::Right => container.items_end(),
            };

            // Fixed width for percentage to prevent reflow (fits "100%")
            let value_width = theme.font_size * 0.85 * 2.5; // ~2.5 chars width

            container
                .child(
                    div()
                        .text_color(theme.foreground_muted)
                        .text_size(px(theme.font_size * 0.6))
                        .line_height(px(theme.font_size * 0.65))
                        .child(SharedString::from(label.clone())),
                )
                .child(
                    div()
                        .min_w(px(if self.fixed_width { value_width } else { 0.0 }))
                        .flex()
                        .justify_end()
                        .text_color(theme.foreground)
                        .text_size(px(theme.font_size * 0.85))
                        .line_height(px(theme.font_size * 0.9))
                        .child(SharedString::from(text)),
                )
                .into_any_element()
        } else {
            div()
                .flex()
                .items_center()
                .text_color(theme.foreground)
                .text_size(px(theme.font_size * 0.85))
                .child(SharedString::from(text))
                .into_any_element()
        }
    }

    fn update(&mut self) -> bool {
        self.dirty.swap(false, Ordering::Relaxed)
    }

    fn value(&self) -> Option<u8> {
        let usage = self.usage.load(Ordering::Relaxed);
        Some(100 - usage) // Invert so low CPU is "good"
    }
}

impl Drop for CpuModule {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}
