//! TUI control for the runtime Full Access override displayed as YOLO.

use super::App;
use crate::app_server_session::AppServerSession;
use crate::app_server_session::AppServerStartedThread;
use crate::app_server_session::thread_session_state_from_thread_resume_response;
use crate::history_cell;
use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::ThreadResumeParams;
use codex_app_server_protocol::ThreadResumeResponse;
use codex_protocol::ThreadId;
use std::collections::HashMap;

#[derive(Debug, Default)]
pub(super) struct YoloMode {
    full_access: HashMap<ThreadId, bool>,
}

impl App {
    pub(super) async fn toggle_yolo_mode(&mut self, app_server: &mut AppServerSession) {
        let Some(thread_id) = self.primary_thread_id else {
            tracing::warn!("cannot toggle Full Access before the primary thread is available");
            return;
        };
        let (override_enabled, configured_full_access) =
            if let Some(enabled) = self.yolo_mode.full_access.get(&thread_id).copied() {
                (
                    enabled,
                    self.primary_session_configured.as_ref().map_or_else(
                        || history_cell::is_yolo_mode(self.chat_widget.config_ref()),
                        |session| {
                            history_cell::has_yolo_permissions(
                                session.approval_policy,
                                &session.permission_profile,
                            )
                        },
                    ),
                )
            } else {
                // Reconnecting a displayed child does not observe the primary's runtime state.
                let request_id = app_server.next_request_id();
                let refreshed: color_eyre::Result<_> = async {
                    let response: ThreadResumeResponse = app_server
                        .request_handle()
                        .request_typed(ClientRequest::ThreadResume {
                            request_id,
                            params: ThreadResumeParams {
                                thread_id: thread_id.to_string(),
                                exclude_turns: true,
                                ..Default::default()
                            },
                        })
                        .await?;
                    let session = thread_session_state_from_thread_resume_response(
                        &response,
                        &self.local_settings,
                    )
                    .await
                    .map_err(color_eyre::eyre::Report::msg)?;
                    Ok((response.full_access, session))
                }
                .await;
                let (enabled, mut session) = match refreshed {
                    Ok(refreshed) => refreshed,
                    Err(err) => {
                        self.chat_widget
                            .add_error_message(format!("Failed to read Full Access: {err}"));
                        return;
                    }
                };
                // The resume response omits the locally resolved parent title.
                if let Some(cached) = self.primary_session_configured.as_ref() {
                    session.fork_parent_title = cached.fork_parent_title.clone();
                }
                let configured_full_access = history_cell::has_yolo_permissions(
                    session.approval_policy,
                    &session.permission_profile,
                );
                if let Some(blank) = self.agents_overview.blank_sessions.get_mut(&thread_id) {
                    blank.session = session.clone();
                }
                if let Some(channel) = self.thread_event_channels.get(&thread_id) {
                    channel.store.lock().await.session = Some(session.clone());
                }
                self.primary_session_configured = Some(session);
                self.observe_full_access(thread_id, enabled);
                (enabled, configured_full_access)
            };
        if !override_enabled && configured_full_access {
            self.chat_widget.add_info_message(
                "Full Access is already enabled by this session's configured permissions."
                    .to_string(),
                /*hint*/ None,
            );
            self.refresh_yolo_status();
            return;
        }

        let enabled = !override_enabled;
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

    pub(super) fn observe_started_thread_full_access(&mut self, started: &AppServerStartedThread) {
        if let Some(enabled) = started.runtime_full_access {
            self.observe_full_access(started.session.thread_id, enabled);
        }
    }

    pub(super) fn observe_full_access(&mut self, thread_id: ThreadId, enabled: bool) {
        self.yolo_mode.full_access.insert(thread_id, enabled);
        self.refresh_yolo_status();
    }

    pub(super) fn refresh_yolo_status(&mut self) {
        let enabled = self
            .current_displayed_thread_id()
            .or(self.primary_thread_id)
            .is_some_and(|thread_id| self.yolo_mode.full_access.get(&thread_id) == Some(&true))
            || history_cell::is_yolo_mode(self.chat_widget.config_ref());
        self.chat_widget
            .set_yolo_status(enabled.then(|| "YOLO".to_string()));
    }
}
