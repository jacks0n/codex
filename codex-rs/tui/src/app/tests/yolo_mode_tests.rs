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

#[tokio::test]
async fn runtime_full_access_toggle_updates_permissions_status() -> Result<()> {
    let mut app = make_test_app().await;
    app.chat_widget.setup_status_line(
        vec![crate::bottom_pane::StatusLineItem::Permissions],
        /*use_theme_colors*/ true,
    );
    app.chat_widget
        .set_approval_policy(AskForApproval::UnlessTrusted);
    app.chat_widget
        .set_permission_profile_from_session_snapshot(PermissionProfileSnapshot::legacy(
            PermissionProfile::Disabled,
        ))?;
    let thread_id = ThreadId::new();
    app.primary_thread_id = Some(thread_id);
    app.refresh_yolo_status();

    insta::assert_snapshot!(
        app.chat_widget.status_line_text().unwrap_or_default(),
        @"Custom permissions"
    );

    app.observe_full_access(thread_id, true);

    insta::assert_snapshot!(
        app.chat_widget.status_line_text().unwrap_or_default(),
        @"Full Access"
    );

    app.observe_full_access(thread_id, false);

    insta::assert_snapshot!(
        app.chat_widget.status_line_text().unwrap_or_default(),
        @"Custom permissions"
    );
    Ok(())
}

#[tokio::test]
async fn runtime_full_access_status_tracks_the_primary_thread() -> Result<()> {
    let mut app = make_test_app().await;
    app.chat_widget.setup_status_line(
        vec![crate::bottom_pane::StatusLineItem::Permissions],
        /*use_theme_colors*/ true,
    );
    app.chat_widget
        .set_approval_policy(AskForApproval::UnlessTrusted);
    app.chat_widget
        .set_permission_profile_from_session_snapshot(PermissionProfileSnapshot::legacy(
            PermissionProfile::Disabled,
        ))?;
    let enabled_thread = ThreadId::new();
    let other_thread = ThreadId::new();

    app.primary_thread_id = Some(enabled_thread);
    app.observe_full_access(enabled_thread, true);
    let enabled_status = app.chat_widget.status_line_text().unwrap_or_default();

    app.primary_thread_id = Some(other_thread);
    app.refresh_yolo_status();
    let other_status = app.chat_widget.status_line_text().unwrap_or_default();

    app.primary_thread_id = Some(enabled_thread);
    app.refresh_yolo_status();
    let restored_status = app.chat_widget.status_line_text().unwrap_or_default();

    pretty_assertions::assert_eq!(
        (
            enabled_status.as_str(),
            other_status.as_str(),
            restored_status.as_str()
        ),
        ("Full Access", "Custom permissions", "Full Access")
    );
    Ok(())
}
