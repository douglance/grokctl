//! Pinned compatibility seed derived from the authorized 0.18 reconstruction.

use crate::{HostManifest, ManifestNotes, classify_command};

const COMMANDS: &[&str] = &[
    "getTranscript",
    "getAgentTranscript",
    "getAgentTranscriptPage",
    "openAgentWindowed",
    "getAgentTranscriptWindow",
    "openAgentTail",
    "getAgentTranscriptTail",
    "getAgentThread",
    "sendPrompt",
    "promptAcceptanceStatus",
    "respondToWidget",
    "resolveAutoReviewApproval",
    "resolveLocalToolPermission",
    "dismissWidget",
    "submitSecret",
    "reactToMessage",
    "appendConnectorCard",
    "listAgents",
    "countAgents",
    "searchAgents",
    "searchMedia",
    "createAgent",
    "kickstartAgent",
    "requestDiskSaverAudit",
    "createGroup",
    "setGroupMembers",
    "updateAgent",
    "deleteAgent",
    "deleteAgents",
    "duplicateAgent",
    "setAgentUnread",
    "setAgentNotificationsEnabled",
    "setAgentNotifyOnUpdates",
    "setAgentHiddenFromSidebar",
    "openAgent",
    "setWindowFocused",
    "getAgentMemories",
    "deleteAgentMemory",
    "clearAgentMemories",
    "getAgentAutomations",
    "listAllAutomations",
    "isAgentNetworkEnabled",
    "isGlobalSearchEnabled",
    "isEgressTunnelAvailable",
    "getSharingState",
    "createRoomFromAgent",
    "createRoomInvite",
    "joinSharedRoom",
    "respondToRoomJoinRequest",
    "createSharedRoom",
    "addOwnAgentToSharedRoom",
    "removeOwnAgentFromSharedRoom",
    "setSharedRoomTyping",
    "leaveSharedRoom",
    "setAgentAutomationEnabled",
    "createAgentAutomation",
    "updateAgentAutomation",
    "deleteAgentAutomation",
    "runAgentAutomationNow",
    "broadcastToAgents",
    "getAgentWorkflows",
    "createAgentWorkflow",
    "updateAgentWorkflow",
    "setAgentWorkflowEnabled",
    "deleteAgentWorkflow",
    "runAgentWorkflowNow",
    "importAgentWorkflowText",
    "importAgentWorkflowUrl",
    "portAgentLocalSkills",
    "getConversationOutline",
    "skillsCatalog",
    "syncPluginSkills",
    "getPluginSyncStatus",
    "getSkillPublishTargets",
    "publishSkill",
    "resyncPublishedSkill",
    "unpublishSkill",
    "getAgentChannels",
    "connectChannel",
    "disconnectChannel",
    "refreshChannel",
    "getListenerIntegrations",
    "getListenerConnectUrl",
    "getSubagents",
    "getAsyncTasks",
    "setAgentAvatarBytes",
    "getAgentAvatar",
    "getForeverBoxStatus",
    "getCloudAgentInfo",
    "ensureForeverBox",
    "resetForeverBox",
    "updateForeverBox",
    "autoUpdateBoxNow",
    "snapshotBoxStoreNow",
    "getBoxStoreStatus",
    "clearBoxStoreNow",
    "updateHostNow",
    "getHostStatus",
    "setBoxMigrating",
    "prepareBoxForRecreate",
    "resumeBoxAfterRecreate",
    "handBackForeverBox",
    "startTeachRecording",
    "stopTeachRecording",
    "getTeachRecordingStatus",
    "getTrays",
    "dismissTray",
    "clearTrays",
    "uploadAttachment",
    "readAttachmentImage",
    "readAttachmentText",
    "readAttachmentChunk",
    "getHostSettings",
    "setHostSettings",
    "setBoxSecrets",
    "getBoxSecretsStatus",
    "completeMcpOAuth",
    "requestWebAuthnCeremony",
    "refreshMcp",
    "listRoutedMcpTools",
    "executeRoutedMcpTool",
    "listBoxMcpServers",
];

pub fn is_seed_command(name: &str) -> bool {
    COMMANDS.contains(&name)
}

/// Return the checked-in prior-art manifest used for drift warnings.
#[must_use]
pub fn seed_manifest() -> HostManifest {
    let commands = COMMANDS.iter().map(ToString::to_string).collect::<Vec<_>>();
    let policies = COMMANDS.iter().map(|name| classify_command(name)).collect();
    HostManifest {
        host_version: "0.18.0-reconstructed".to_owned(),
        capabilities: vec!["orderedReplicasV1".to_owned(), "sendAcceptanceV1".to_owned()],
        commands,
        policies,
        notes: ManifestNotes {
            commands:
                "Authorized source-oriented 0.18 reconstruction; refresh from a host snapshot"
                    .to_owned(),
            capabilities: "Pinned reconstruction markers".to_owned(),
            schemas: "Typed locally; host schemas were not available".to_owned(),
        },
        source_sha256: "30c492d9c634bff3ed8cc22eccd4b128a5940b45734973973eedd2f72c8231bf"
            .to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn seed_contains_the_complete_unique_reconstructed_table() {
        let manifest = seed_manifest();
        let unique = manifest.commands.iter().collect::<BTreeSet<_>>();

        assert_eq!(manifest.commands.len(), 122);
        assert_eq!(unique.len(), manifest.commands.len());
        assert_eq!(manifest.policies.len(), manifest.commands.len());
        assert_eq!(
            manifest.source_sha256,
            "30c492d9c634bff3ed8cc22eccd4b128a5940b45734973973eedd2f72c8231bf"
        );
    }
}
