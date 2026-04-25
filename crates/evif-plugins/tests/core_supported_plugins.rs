use std::collections::BTreeSet;

use evif_plugins::{core_supported_plugins, experimental_plugins};

#[test]
fn core_supported_plugins_inventory_matches_supported_surface() {
    let core_ids: BTreeSet<&str> = core_supported_plugins()
        .iter()
        .map(|plugin| plugin.id)
        .collect();
    let expected = BTreeSet::from([
        "contextfs",
        "heartbeatfs",
        "hellofs",
        "kvfs",
        "localfs",
        "memfs",
        "pipefs",
        "proxyfs",
        "queuefs",
        "serverinfofs",
        "skillfs",
        "sqlfs2",
        "streamfs",
    ]);

    assert_eq!(core_ids, expected);
    assert!(core_supported_plugins()
        .iter()
        .all(|plugin| plugin.is_mountable));
}

#[test]
fn experimental_plugins_are_not_marked_as_core_supported() {
    assert!(experimental_plugins()
        .iter()
        .all(|plugin| plugin.support_tier.as_str() == "experimental"));
    assert!(experimental_plugins()
        .iter()
        .any(|plugin| !plugin.is_mountable));
}
