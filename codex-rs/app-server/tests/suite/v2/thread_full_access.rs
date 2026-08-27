use anyhow::Result;
use app_test_support::MockResponsesConfig;
use app_test_support::TestAppServer;
use app_test_support::create_mock_responses_server_repeating_assistant;
use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::ThreadFullAccessUpdateParams;
use codex_app_server_protocol::ThreadFullAccessUpdateResponse;
use codex_app_server_protocol::ThreadFullAccessUpdatedNotification;
use codex_app_server_protocol::ThreadStartParams;
use codex_app_server_protocol::ThreadStartResponse;
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
