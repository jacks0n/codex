//! Runtime permission projection without changing configured or executor-owned policy.

use crate::environment_selection::EnvironmentConfigOrigin;
use crate::environment_selection::TurnEnvironmentSnapshot;
use codex_mcp::McpConfig;
use codex_mcp::McpServerSource;
use codex_protocol::models::PermissionProfile;

pub(crate) fn full_access_profile(configured: &PermissionProfile) -> PermissionProfile {
    let (mut file_system, network) = PermissionProfile::Disabled.to_runtime_permissions();
    file_system.preserve_deny_read_restrictions_from(&configured.file_system_sandbox_policy());
    PermissionProfile::from_runtime_permissions_with_enforcement(
        PermissionProfile::Disabled.enforcement(),
        &file_system,
        network,
    )
}

pub(crate) fn mcp_config_origin(
    environments: &TurnEnvironmentSnapshot,
    config: &McpConfig,
    server_name: &str,
) -> Option<EnvironmentConfigOrigin> {
    let server = config.mcp_server_catalog.server(server_name)?;
    let server_config = server.config();
    if server
        .source()
        .is_host_owned_apps(server_name, server_config)
    {
        return Some(EnvironmentConfigOrigin::Thread);
    }
    environments
        .config_origin(&server_config.environment_id)
        .or_else(|| {
            (server_config.is_local_environment()
                || matches!(server.source(), McpServerSource::SelectedPlugin(_)))
            .then_some(EnvironmentConfigOrigin::Thread)
        })
}
