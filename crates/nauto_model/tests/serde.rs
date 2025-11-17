use nauto_model::*;
use serde_json::json;
use uuid::Uuid;

#[test]
fn job_round_trip() {
    let parameters: std::collections::HashMap<String, serde_json::Value> =
        serde_json::from_value(json!({"dry_run": true})).expect("map");
    let job = Job {
        id: Uuid::nil(),
        name: "Config Push".into(),
        kind: JobKind::ConfigPush {
            snippet: "set system ntp server 1.2.3.4".into(),
        },
        targets: TargetSelector::ByTags {
            all_of: vec!["site:oslo".into()],
        },
        parameters,
        max_parallel: Some(25),
        dry_run: true,
        approval_id: None,
    };

    let serialized = serde_json::to_string_pretty(&job).expect("serialize job");
    let restored: Job = serde_json::from_str(&serialized).expect("deserialize job");
    assert_eq!(restored.name, "Config Push");
    assert!(restored.dry_run);
    assert_eq!(restored.max_parallel, Some(25));
}

#[test]
fn device_capabilities_default() {
    let device = Device {
        id: "edge-j1".into(),
        name: "Edge-J1".into(),
        device_type: DeviceType::JuniperJunos,
        mgmt_address: "10.0.0.2".into(),
        credential: CredentialRef {
            name: "lab-default".into(),
        },
        tags: vec!["site:oslo".into(), "role:edge".into()],
        capabilities: CapabilitySet {
            supports_commit: true,
            supports_rollback: true,
            supports_diff: true,
            supports_dry_run: true,
        },
    };

    let yaml = serde_yaml::to_string(&device).expect("serialize device");
    let loaded: Device = serde_yaml::from_str(&yaml).expect("deserialize device");
    assert!(loaded.capabilities.supports_commit);
    assert!(loaded.capabilities.supports_diff);
}
