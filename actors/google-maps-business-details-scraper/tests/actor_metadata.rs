use std::{fs, path::PathBuf};

use serde_json::Value;

fn actor_file(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(path)
}

fn read_json(path: &str) -> Value {
    serde_json::from_str(&fs::read_to_string(actor_file(path)).unwrap()).unwrap()
}

#[test]
fn preserves_input_schema_and_prefill() {
    let schema = read_json(".actor/input_schema.json");

    assert_eq!(schema["schemaVersion"], 1);
    assert!(schema.get("anyOf").is_none());
    assert_eq!(schema["properties"]["business_ids"]["maxItems"], 10);
    assert_eq!(
        schema["properties"]["business_ids"]["prefill"][0],
        "0x808fba02425dad8f:0x6c296c66619367e0"
    );
    assert_eq!(
        schema["properties"]["business_id"]["prefill"],
        "0x808fba02425dad8f:0x6c296c66619367e0"
    );
    assert_eq!(schema["properties"]["use_cache"]["default"], true);
    assert_eq!(schema["properties"]["maximum_cache_age"]["default"], 3600);
}

#[test]
fn keeps_dataset_overview_fields_and_actor_runtime_settings() {
    let actor = read_json(".actor/actor.json");
    let view = &actor["storages"]["dataset"]["views"]["overview"];

    assert_eq!(
        view["transformation"]["fields"],
        serde_json::json!([
            "name",
            "rating",
            "review_count",
            "full_address",
            "phone_number",
            "website",
            "type"
        ])
    );
    assert_eq!(
        view["display"]["properties"]["full_address"]["label"],
        "Address"
    );
    assert_eq!(
        view["display"]["properties"]["phone_number"]["label"],
        "Phone"
    );
    assert_eq!(actor["dockerfile"], "./Dockerfile");
    assert_eq!(
        actor["environmentVariables"]["SCRAPPA_API_KEY"],
        "@SCRAPPA_API_KEY"
    );
    assert_eq!(actor["resources"]["memoryMbytes"], 128);
    assert_eq!(actor["defaultRunOptions"]["timeoutSecs"], 720);
}
