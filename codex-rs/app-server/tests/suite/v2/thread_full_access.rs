use anyhow::Result;
use app_test_support::MockResponsesConfig;
use app_test_support::TestAppServer;
use app_test_support::create_command_execution_sse_response;
use app_test_support::create_final_assistant_message_sse_response;
use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::ThreadFullAccessUpdateParams;
use codex_app_server_protocol::ThreadFullAccessUpdateResponse;
use codex_app_server_protocol::ThreadFullAccessUpdatedNotification;
use codex_app_server_protocol::ThreadResumeParams;
use codex_app_server_protocol::ThreadResumeResponse;
use codex_app_server_protocol::ThreadStartParams;
use codex_app_server_protocol::ThreadStartResponse;
use codex_app_server_protocol::TurnCompletedNotification;
use codex_app_server_protocol::TurnStartParams;
use codex_app_server_protocol::TurnStartResponse;
use codex_app_server_protocol::UserInput;
use core_test_support::skip_if_no_network;
use core_test_support::skip_if_wine_exec;
use core_test_support::streaming_sse::StreamingSseChunk;
use core_test_support::streaming_sse::start_streaming_sse_server;
use pretty_assertions::assert_eq;
use std::time::Duration;
use tempfile::TempDir;
use tokio::sync::oneshot;
use tokio::time::timeout;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(/*secs*/ 10);

#[tokio::test]
async fn thread_full_access_update_applies_to_an_active_turn() -> Result<()> {
    skip_if_no_network!(Ok(()));
    skip_if_wine_exec!(
        Ok(()),
        "exec_command approval routing is unavailable for Wine executors"
    );

    const CALL_ID: &str = "active-turn-command";
    let command_response = create_command_execution_sse_response(
        vec![
            "python3".to_string(),
            "-c".to_string(),
            "from pathlib import Path; Path('full-access.txt').write_text('done'); print('full-access-complete')".to_string(),
        ],
        /*workdir*/ None,
        /*timeout_ms*/ Some(5_000),
        CALL_ID,
    )?;
    let final_response = create_final_assistant_message_sse_response("done")?;
    let (release, gate) = oneshot::channel();
    let (server, _) = start_streaming_sse_server(vec![
        vec![StreamingSseChunk {
            gate: Some(gate),
            body: command_response,
        }],
        vec![StreamingSseChunk {
            gate: None,
            body: final_response,
        }],
    ])
    .await;

    let codex_home = TempDir::new()?;
    MockResponsesConfig::new(server.uri())
        .with_approval_policy("untrusted")
        .with_sandbox_mode("read-only")
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

    timeout(DEFAULT_TIMEOUT, server.wait_for_request_count(/*count*/ 1)).await?;
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
            thread_id: thread.id.clone(),
            enabled: true
        }
    );
    let resumed: ThreadResumeResponse = mcp
        .request(|request_id| ClientRequest::ThreadResume {
            request_id,
            params: ThreadResumeParams {
                thread_id: thread.id.clone(),
                ..Default::default()
            },
        })
        .await?;
    assert!(resumed.full_access);
    release.send(()).expect("release sampled tool call");

    let completed: TurnCompletedNotification =
        timeout(DEFAULT_TIMEOUT, mcp.read_notification("turn/completed")).await??;
    assert_eq!(completed.thread_id, thread.id);
    let requests = server.requests().await;
    let request: serde_json::Value = serde_json::from_slice(&requests[1])?;
    let output = request["input"]
        .as_array()
        .expect("input")
        .iter()
        .find(|item| item["type"] == "function_call_output" && item["call_id"] == CALL_ID)
        .expect("command output");
    assert!(
        output["output"]
            .to_string()
            .contains("full-access-complete"),
        "{output}"
    );
    let disabled: ThreadFullAccessUpdateResponse = mcp
        .request(|request_id| ClientRequest::ThreadFullAccessUpdate {
            request_id,
            params: ThreadFullAccessUpdateParams {
                thread_id: thread.id.clone(),
                enabled: false,
            },
        })
        .await?;
    assert_eq!(disabled, ThreadFullAccessUpdateResponse { enabled: false });
    let resumed: ThreadResumeResponse = mcp
        .request(|request_id| ClientRequest::ThreadResume {
            request_id,
            params: ThreadResumeParams {
                thread_id: thread.id.clone(),
                ..Default::default()
            },
        })
        .await?;
    assert!(!resumed.full_access);

    Ok(())
}
