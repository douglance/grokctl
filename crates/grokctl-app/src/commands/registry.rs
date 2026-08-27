//! Root command-group registry.

use incurs::cli::Cli;

use super::{
    bot, catalog, computer, gateway, group, host, manifest, media, memory, profile, receipt,
    routine, workflow,
};

pub fn groups() -> Vec<Cli> {
    vec![
        profile::group(),
        gateway::group(),
        manifest::group(),
        bot::group(),
        group::group(),
        memory::group(),
        routine::group(),
        workflow::group(),
        catalog::group(),
        computer::group(),
        host::group(),
        media::group(),
        receipt::group(),
    ]
}
