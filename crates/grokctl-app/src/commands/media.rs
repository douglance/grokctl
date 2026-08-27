//! Attachment and media commands.

use incurs::cli::Cli;

use super::common::{FixedSpec, fixed_group};

const COMMANDS: &[FixedSpec] = &[
    ("search", "searchMedia", "Search media"),
    ("get-avatar", "getAgentAvatar", "Read avatar metadata"),
    ("set-avatar", "setAgentAvatarBytes", "Set avatar bytes"),
    ("upload", "uploadAttachment", "Upload attachment metadata"),
    ("read-image", "readAttachmentImage", "Read an attachment image"),
    ("read-text", "readAttachmentText", "Read attachment text"),
    ("read-chunk", "readAttachmentChunk", "Read an attachment chunk"),
    ("audit", "requestDiskSaverAudit", "Request a disk-saver audit"),
];

pub fn group() -> Cli {
    fixed_group("media", "Manage attachment and media metadata", COMMANDS)
}
