use anyhow::Result;
use assert_cmd::Command;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_run_job_e2e_command_batch() -> Result<()> {
    let temp = tempdir()?;
    let job_path = temp.path().join("job.yaml");
    let inventory_path = temp.path().join("inventory.yaml");
    
    // Mock job
    let job_yaml = r#"
name: "E2E Test Job"
kind:
  type: command_batch
  commands:
    - "show version"
targets:
  mode: all
dry_run: false
"#;
    fs::write(&job_path, job_yaml)?;

    // Mock inventory
    let inv_yaml = r#"
devices:
  - id: "mock-r1"
    name: "mock-r1"
    device_type: "cisco_ios"
    mgmt_address: "1.1.1.1"
    credential:
      name: "default"
    tags: []
    capabilities: {}
"#;
    fs::write(&inventory_path, inv_yaml)?;

    let mut cmd = Command::cargo_bin("nauto_cli")?;
    cmd.env("NAUTO_USE_MOCK_DRIVERS", "1")
       .env("NAUTO_KEYRING_FILE", temp.path().join("creds.json")) // Isolate keyring
       .arg("run")
       .arg("--job")
       .arg(&job_path)
       .arg("--inventory")
       .arg(&inventory_path);

    cmd.assert()
        .success();
        // .stdout(predicate::str::contains("E2E Test Job")); // Verify output if needed

    Ok(())
}
