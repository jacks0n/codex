//! TUI control for the runtime Full Access override displayed as YOLO.

use super::App;
use crate::app_server_session::AppServerSession;
use crate::history_cell;
use codex_protocol::ThreadId;

#[derive(Debug, Default)]
pub(super) struct YoloMode {
    override_enabled: bool,
}

impl App {
    pub(super) async fn toggle_yolo_mode(&mut self, app_server: &mut AppServerSession) {
        let Some(thread_id) = self.primary_thread_id else {
            tracing::warn!("cannot toggle Full Access before the primary thread is available");
            return;
        };
        if !self.yolo_mode.override_enabled
            && history_cell::is_yolo_mode(self.chat_widget.config_ref())
        {
            self.chat_widget.add_info_message(
                "Full Access is already enabled by this session's configured permissions."
                    .to_string(),
                /*hint*/ None,
            );
            self.refresh_yolo_status();
            return;
        }

        let enabled = !self.yolo_mode.override_enabled;
        match app_server
            .thread_full_access_update(thread_id, enabled)
            .await
        {
            Ok(Some(enabled)) => self.observe_full_access(thread_id, enabled),
            Ok(None) => self.chat_widget.add_error_message(
                "The connected app server does not support the Full Access toggle.".to_string(),
            ),
            Err(err) => self
                .chat_widget
                .add_error_message(format!("Failed to update Full Access: {err}")),
        }
    }

    pub(super) fn observe_full_access(&mut self, thread_id: ThreadId, enabled: bool) {
        if self.primary_thread_id == Some(thread_id) {
            self.yolo_mode.override_enabled = enabled;
            self.refresh_yolo_status();
        }
    }

    pub(super) fn refresh_yolo_status(&mut self) {
        let enabled = self.yolo_mode.override_enabled
            || history_cell::is_yolo_mode(self.chat_widget.config_ref());
        self.chat_widget
            .set_yolo_status(enabled.then(|| "YOLO".to_string()));
    }
}
