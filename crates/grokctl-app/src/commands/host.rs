//! Host and channel commands.

use incurs::cli::Cli;

use super::common::{FixedSpec, fixed_group};

const COMMANDS: &[FixedSpec] = &[
    ("status", "getHostStatus", "Read host status and capabilities"),
    ("channels", "getAgentChannels", "List one Bot's channels"),
    ("connect", "connectChannel", "Connect a channel"),
    ("disconnect", "disconnectChannel", "Disconnect a channel"),
    ("refresh-channel", "refreshChannel", "Refresh a channel"),
    ("listeners", "getListenerIntegrations", "List listener integrations"),
    ("listener-url", "getListenerConnectUrl", "Get a listener connection URL"),
    ("settings", "getHostSettings", "Read host settings"),
    ("set-settings", "setHostSettings", "Update host settings"),
    ("update-now", "updateHostNow", "Update the host now"),
    ("reset-box", "resetForeverBox", "Reset the Bot box"),
];

pub fn group() -> Cli {
    fixed_group("host", "Inspect and administer the Bot host", COMMANDS)
}
