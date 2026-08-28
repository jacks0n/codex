use anyhow::Result;
use app_test_support::MockResponsesConfig;
use app_test_support::TestAppServer;
use app_test_support::create_command_execution_sse_response;
use app_test_support::create_final_assistant_message_sse_response;
use app_test_support::create_mock_responses_server_repeating_assistant;
use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::ThreadFullAccessUpdateParams;
use codex_app_server_protocol::ThreadFullAccessUpdateResponse;
use codex_app_server_protocol::ThreadFullAccessUpdatedNotification;
use codex_app_server_protocol::ThreadStartParams;
use codex_app_server_protocol::ThreadStartResponse;
use codex_app_server_protocol::TurnCompletedNotification;
use codex_app_server_protocol::TurnStartParams;
use codex_app_server_protocol::TurnStartResponse;
use codex_app_server_protocol::UserInput;
use core_test_support::responses;
use core_test_support::skip_if_no_network;
use pretty_assertions::assert_eq;
use std::time::Duration;
use tempfile::TempDir;
use tokio::time::timeout;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

#[tokio::test]
async fn thread_full_access_update_returns_and_notifies_enabled_state() -> Result<()> {
    let server = create_mock_responses_server_repeating_assistant("Done").await;
    let codex_home = TempDir::new()?;
    MockResponsesConfig::new(&server.uri()).write(codex_home.path())?;
    let mut mcp = TestAppServer::builder()
        .with_codex_home(codex_home.path())
        .build_initialized_with_timeout(DEFAULT_TIMEOUT)
        .await?;

    let request_id = mcp
        .send_thread_start_request_with_auto_env(ThreadStartParams {
            model: Some("mock-model".to_string()),
            ..Default::default()
        })
        .await?;
    let ThreadStartResponse { thread, .. } =
        timeout(DEFAULT_TIMEOUT, mcp.read_response(request_id)).await??;

    let response: ThreadFullAccessUpdateResponse = mcp
        .request(|request_id| ClientRequest::ThreadFullAccessUpdate {
            request_id,
            params: ThreadFullAccessUpdateParams {
                thread_id: thread.id.clone(),
                enabled: true,
            },
        })
        .await?;
    assert_eq!(response, ThreadFullAccessUpdateResponse { enabled: true });

    let notification: ThreadFullAccessUpdatedNotification = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_notification("thread/fullAccess/updated"),
    )
    .await??;
    assert_eq!(
        notification,
        ThreadFullAccessUpdatedNotification {
            thread_id: thread.id,
            enabled: true,
        }
    );
    Ok(())
}

#[tokio::test]
async fn thread_full_access_update_applies_to_an_active_turn() -> Result<()> {
    skip_if_no_network!(Ok(()));

    const CALL_ID: &str = "active-turn-command";
    let server = responses::start_mock_server().await;
    let command_response = create_command_execution_sse_response(
        vec![
            "python3".to_string(),
            "-c".to_string(),
            "print(42)".to_string(),
        ],
        /*workdir*/ None,
        Some(5_000),
        CALL_ID,
    )?;
    let final_response = create_final_assistant_message_sse_response("done")?;
    let response_mock = responses::mount_response_sequence(
        &server,
        vec![
            responses::sse_response(command_response).set_delay(Duration::from_secs(1)),
            responses::sse_response(final_response),
        ],
    )
    .await;

    let codex_home = TempDir::new()?;
    MockResponsesConfig::new(&server.uri())
        .with_approval_policy("untrusted")
        .with_sandbox_mode("danger-full-access")
        .write(codex_home.path())?;
    let mut mcp = TestAppServer::builder()
        .with_codex_home(codex_home.path())
        .build_initialized_with_timeout(DEFAULT_TIMEOUT)
        .await?;

    let ThreadStartResponse { thread, .. } = mcp
        .start_thread(ThreadStartParams {
            model: Some("mock-model".to_string()),
            ..Default::default()
        })
        .await?;
    let _: TurnStartResponse = mcp
        .request(|request_id| ClientRequest::TurnStart {
            request_id,
            params: TurnStartParams {
                thread_id: thread.id.clone(),
                input: vec![UserInput::Text {
                    text: "run python".to_string(),
                    text_elements: Vec::new(),
                }],
                ..Default::default()
            },
        })
        .await?;

    let response: ThreadFullAccessUpdateResponse = mcp
        .request(|request_id| ClientRequest::ThreadFullAccessUpdate {
            request_id,
            params: ThreadFullAccessUpdateParams {
                thread_id: thread.id.clone(),
                enabled: true,
            },
        })
        .await?;
    assert_eq!(response, ThreadFullAccessUpdateResponse { enabled: true });

    let completed: TurnCompletedNotification =
        timeout(DEFAULT_TIMEOUT, mcp.read_notification("turn/completed")).await??;
    assert_eq!(completed.thread_id, thread.id);
    assert!(
        response_mock
            .function_call_output_text(CALL_ID)
            .is_some_and(|output| output.contains("42")),
        "active turn should execute without requesting approval after Full Access is enabled"
    );

    Ok(())
}
