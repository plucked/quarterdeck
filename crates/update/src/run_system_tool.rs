use std::process::Command;

use crate::UpdateError;

/// What a system tool that ran but exited with an error means.
#[derive(Clone, Copy)]
pub(crate) enum ToolRefusal {
    /// The tool failed at its task: [`UpdateError::Tool`].
    Failed,
    /// The tool rejected the signature it checked: [`UpdateError::Signature`].
    SignatureRejected,
}

/// Runs a system tool and returns what it printed, trimmed. A tool that
/// cannot be started is [`UpdateError::Tool`]; one that exits with an
/// error is reported as `refusal` says, carrying its error output.
pub(crate) fn run_system_tool(
    tool: &'static str,
    command: &mut Command,
    refusal: ToolRefusal,
) -> Result<String, UpdateError> {
    let output = command.output().map_err(|error| UpdateError::Tool {
        tool,
        message: error.to_string(),
    })?;
    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned());
    }
    let message = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    Err(match refusal {
        ToolRefusal::Failed => UpdateError::Tool { tool, message },
        ToolRefusal::SignatureRejected => UpdateError::Signature(message),
    })
}
