//! Standard type to use with the `--approval-mode` CLI option.

use clap::ValueEnum;

use codex_protocol::protocol::AskForApproval;

#[derive(Clone, Copy, Debug, ValueEnum)]
#[value(rename_all = "kebab-case")]
pub enum ApprovalModeCliArg {
    /// Require approval unless an explicit exec policy rule allows the command.
    Untrusted,

    /// The model decides when to ask the user for approval.
    OnRequest,

    /// Never ask for user approval
    /// Execution failures are immediately returned to the model.
    Never,
}

impl From<ApprovalModeCliArg> for AskForApproval {
    fn from(value: ApprovalModeCliArg) -> Self {
        match value {
            ApprovalModeCliArg::Untrusted => AskForApproval::UnlessTrusted,
            ApprovalModeCliArg::OnRequest => AskForApproval::OnRequest,
            ApprovalModeCliArg::Never => AskForApproval::Never,
        }
    }
}
