mod bar_view;
mod panel_view;
mod popup_view;

pub use bar_view::{
    bump_config_version, handle_mouse_event, init_click_channel, set_panel_visible, set_popup_gap,
    update_modules, BarView, PopupInfo, ViewClickEvent,
};
pub use panel_view::{PanelContent, PanelView};
pub use popup_view::{PopupContent, PopupView};
