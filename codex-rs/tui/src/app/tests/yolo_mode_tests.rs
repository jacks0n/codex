use super::*;
use codex_protocol::models::PermissionProfile;

#[tokio::test]
async fn attaching_primary_session_refreshes_full_access_from_session_permissions() -> Result<()> {
    let mut app = make_test_app().await;
    app.chat_widget.setup_status_line(
        vec![crate::bottom_pane::StatusLineItem::Permissions],
        /*use_theme_colors*/ true,
    );
    app.chat_widget.set_approval_policy(AskForApproval::Never);
    app.chat_widget
        .set_permission_profile_from_session_snapshot(PermissionProfileSnapshot::legacy(
            PermissionProfile::Disabled,
        ))?;
    app.refresh_yolo_status();
    insta::assert_snapshot!(
        &app.chat_widget.status_line_text().unwrap_or_default(),
        @"Full Access"
    );

    let thread_id = ThreadId::new();
    let session = ThreadSessionState {
        approval_policy: AskForApproval::UnlessTrusted,
        permission_profile: PermissionProfile::Disabled,
        ..test_thread_session(thread_id, test_path_buf("/tmp/project"))
    };
    app.enqueue_primary_thread_session(session, Vec::new())
        .await?;

    insta::assert_snapshot!(
        &app.chat_widget.status_line_text().unwrap_or_default(),
        @"Custom permissions"
    );
    Ok(())
}
