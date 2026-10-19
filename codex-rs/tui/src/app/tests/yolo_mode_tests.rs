use super::*;
use codex_protocol::models::PermissionProfile;
use pretty_assertions::assert_eq;

#[tokio::test]
async fn full_access_status_follows_configured_permissions_and_displayed_thread() -> Result<()> {
    let mut app = make_test_app().await;
    app.chat_widget
        .set_approval_policy(AskForApproval::UnlessTrusted);
    app.chat_widget
        .set_permission_profile_from_session_snapshot(PermissionProfileSnapshot::legacy(
            PermissionProfile::read_only(),
        ))?;
    let primary = ThreadId::new();
    app.primary_thread_id = Some(primary);
    app.active_thread_id = Some(primary);
    app.refresh_status_line();
    let configured_status = app.chat_widget.status_line_text();
    app.observe_full_access(primary, /*enabled*/ true);
    assert!(
        app.chat_widget
            .status_line_text()
            .expect("runtime status")
            .contains("YOLO")
    );
    app.observe_full_access(primary, /*enabled*/ false);
    assert_eq!(app.chat_widget.status_line_text(), configured_status);
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
    let mut statuses = vec![app.chat_widget.status_line_text().unwrap_or_default()];
    let fork = ThreadId::new();
    let session = ThreadSessionState {
        approval_policy: AskForApproval::UnlessTrusted,
        permission_profile: PermissionProfile::Disabled,
        ..test_thread_session(primary, test_path_buf("/tmp/project"))
    };
    app.enqueue_primary_thread_session(session, Vec::new())
        .await?;
    statuses.push(app.chat_widget.status_line_text().unwrap_or_default());
    app.observe_full_access(primary, /*enabled*/ true);
    statuses.push(app.chat_widget.status_line_text().unwrap_or_default());
    app.active_thread_id = Some(fork);
    app.refresh_yolo_status();
    statuses.push(app.chat_widget.status_line_text().unwrap_or_default());
    app.active_thread_id = Some(primary);
    app.refresh_yolo_status();
    statuses.push(app.chat_widget.status_line_text().unwrap_or_default());
    app.observe_full_access(primary, /*enabled*/ false);
    statuses.push(app.chat_widget.status_line_text().unwrap_or_default());
    app.upsert_agent_picker_thread(
        fork, /*agent_nickname*/ None, /*agent_role*/ None, /*is_closed*/ false,
    );
    app.observe_full_access(fork, /*enabled*/ true);
    statuses.push(app.chat_widget.status_line_text().unwrap_or_default());
    app.active_thread_id = Some(fork);
    app.refresh_yolo_status();
    statuses.push(app.chat_widget.status_line_text().unwrap_or_default());
    app.active_thread_id = Some(primary);
    app.refresh_yolo_status();
    statuses.push(app.chat_widget.status_line_text().unwrap_or_default());
    insta::assert_debug_snapshot!(statuses, @r###"
    [
        "Full Access",
        "Custom permissions",
        "Full Access",
        "Custom permissions",
        "Full Access",
        "Custom permissions",
        "Custom permissions",
        "Full Access",
        "Custom permissions",
    ]
    "###);
    Ok(())
}

#[tokio::test]
async fn full_access_toggle_uses_primary_permissions_while_viewing_child() -> Result<()> {
    let mut app = make_test_app().await;
    app.config
        .permissions
        .set_permission_profile(PermissionProfile::read_only())?;
    let mut server = start_config_write_test_app_server(&app).await?;
    let started = server.start_thread(&app.config).await?;
    app.observe_started_thread_full_access(&started);
    let primary_session = started.session.clone();
    app.enqueue_primary_thread_session(started.session, started.turns)
        .await?;
    app.chat_widget.handle_thread_session(ThreadSessionState {
        approval_policy: AskForApproval::Never,
        permission_profile: PermissionProfile::Disabled,
        ..test_thread_session(ThreadId::new(), app.config.cwd.to_path_buf())
    });
    app.toggle_yolo_mode(&mut server).await;
    app.chat_widget.handle_thread_session(primary_session);
    app.refresh_yolo_status();
    app.chat_widget.setup_status_line(
        vec![crate::bottom_pane::StatusLineItem::Permissions],
        /*use_theme_colors*/ true,
    );
    insta::assert_snapshot!(app.chat_widget.status_line_text().unwrap_or_default(), @"Full Access");
    app.toggle_yolo_mode(&mut server).await;
    insta::assert_snapshot!(app.chat_widget.status_line_text().unwrap_or_default(), @"Read Only");
    server.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn read_only_history_preserves_observed_full_access() -> Result<()> {
    let mut app = make_test_app().await;
    app.config
        .permissions
        .set_permission_profile(PermissionProfile::read_only())?;
    let primary = ThreadId::from_string(
        &app_test_support::create_fake_paginated_rollout(
            app.config.codex_home.as_path(),
            "2025-01-05T12-00-00",
            "2025-01-05T12:00:00Z",
            "Saved user message",
            Some(app.config.model_provider_id.as_str()),
            /*git_info*/ None,
        )
        .expect("materialized rollout"),
    )?;
    let mut server = start_config_write_test_app_server(&app).await?;
    let started = server
        .resume_thread(
            &app.local_settings,
            app.config.clone(),
            primary,
            crate::app_server_session::ResumeModelSettings::PreserveExistingThread,
        )
        .await?;
    app.enqueue_primary_thread_session(started.session, started.turns)
        .await?;
    app.toggle_yolo_mode(&mut server).await;
    let (history, _) = server
        .read_thread_for_viewing(&app.config, &app.local_settings, primary)
        .await?;
    let mut snapshot = ThreadEventStore::new(THREAD_EVENT_CHANNEL_CAPACITY).snapshot();
    app.apply_refreshed_snapshot_thread(primary, history, &mut snapshot)
        .await;
    app.replay_thread_snapshot(snapshot, /*resume_restored_queue*/ false);
    app.chat_widget.setup_status_line(
        vec![crate::bottom_pane::StatusLineItem::Permissions],
        /*use_theme_colors*/ true,
    );
    insta::assert_snapshot!(app.chat_widget.status_line_text().unwrap_or_default(), @"Full Access");
    let cached = app
        .primary_session_configured
        .as_mut()
        .expect("primary settings");
    cached.approval_policy = AskForApproval::Never;
    cached.permission_profile = PermissionProfile::Disabled;
    app.yolo_mode = Default::default();
    app.toggle_yolo_mode(&mut server).await;
    insta::assert_snapshot!(app.chat_widget.status_line_text().unwrap_or_default(), @"Read Only");
    app.toggle_yolo_mode(&mut server).await;
    insta::assert_snapshot!(app.chat_widget.status_line_text().unwrap_or_default(), @"Full Access");
    server.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn refreshed_and_replayed_session_restores_full_access_status() -> Result<()> {
    let mut statuses = Vec::new();
    for child in [false, true] {
        for enabled in [true, false] {
            let mut app = make_test_app().await;
            let primary = ThreadId::new();
            let thread_id = if child { ThreadId::new() } else { primary };
            app.primary_thread_id = Some(primary);
            app.upsert_agent_picker_thread(
                thread_id, /*agent_nickname*/ None, /*agent_role*/ None,
                /*is_closed*/ false,
            );
            app.observe_full_access(primary, !enabled);
            let session = ThreadSessionState {
                approval_policy: AskForApproval::UnlessTrusted,
                permission_profile: PermissionProfile::Disabled,
                ..test_thread_session(thread_id, app.config.cwd.to_path_buf())
            };
            let mut snapshot = ThreadEventStore::new(THREAD_EVENT_CHANNEL_CAPACITY).snapshot();
            app.apply_refreshed_snapshot_thread(
                thread_id,
                AppServerStartedThread {
                    session,
                    turns: Vec::new(),
                    blocks_direct_input: child,
                    task_tools_available: false,
                    runtime_full_access: Some(enabled),
                },
                &mut snapshot,
            )
            .await;
            let mut tui = crate::tui::test_support::make_test_tui()?;
            let init = app.chatwidget_init_for_forked_or_resumed_thread(
                &mut tui,
                app.config.clone(),
                /*initial_user_message*/ None,
            );
            app.replace_chat_widget(ChatWidget::new_with_app_event(init));
            app.replay_thread_snapshot(snapshot, /*resume_restored_queue*/ false);
            app.chat_widget.setup_status_line(
                vec![crate::bottom_pane::StatusLineItem::Permissions],
                /*use_theme_colors*/ true,
            );
            statuses.push(app.chat_widget.status_line_text().unwrap_or_default());
        }
    }
    insta::assert_debug_snapshot!(statuses, @r###"
    [
        "Full Access",
        "Custom permissions",
        "Full Access",
        "Custom permissions",
    ]
    "###);
    Ok(())
}
