//! Runtime permission projection without changing configured or executor-owned policy.

use crate::environment_selection::EnvironmentConfigOrigin;
use crate::environment_selection::TurnEnvironmentSnapshot;
use codex_mcp::McpPermissionSource;
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
    source: &McpPermissionSource<'_>,
) -> Option<EnvironmentConfigOrigin> {
    match source {
        McpPermissionSource::Thread { .. } => Some(EnvironmentConfigOrigin::Thread),
        McpPermissionSource::Environment { environment_id, .. } => {
            environments.config_origin(environment_id)
        }
    }
}
