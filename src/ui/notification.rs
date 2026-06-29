use gpui::{App, SharedString, Styled, px};
use gpui_component::{
    ActiveTheme,
    notification::{Notification, NotificationType},
};

const STATUS_NOTIFICATION_WIDTH: f32 = 360.;

pub(crate) fn status_notification(
    message: impl Into<SharedString>,
    notification_type: NotificationType,
    cx: &mut App,
) -> Notification {
    let theme = cx.theme();
    let color = match notification_type {
        NotificationType::Info => theme.info,
        NotificationType::Success => theme.success,
        NotificationType::Warning => theme.warning,
        NotificationType::Error => theme.danger,
    };

    Notification::new()
        .message(message)
        .with_type(notification_type)
        .w(px(STATUS_NOTIFICATION_WIDTH))
        .pr_10()
        .border_color(color.opacity(0.75))
        .bg(theme.popover.blend(color.opacity(0.12)))
}
