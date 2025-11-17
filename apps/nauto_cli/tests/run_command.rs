use assert_cmd::Command;
use predicates::str::contains;
use std::path::Path;
use tempfile::TempDir;

#[test]
fn run_command_uses_mock_drivers() {
    let audit_dir = TempDir::new().expect("temp dir");
    let audit_path = audit_dir.path().join("audit.log");

    Command::cargo_bin("nauto_cli")
        .expect("binary")
        .env("NAUTO_USE_MOCK_DRIVERS", "1")
        .arg("run")
        .arg("--job")
        .arg(path("examples/jobs/show_version.yaml"))
        .arg("--inventory")
        .arg(path("examples/inventory.yaml"))
        .arg("--audit-log")
        .arg(&audit_path)
        .arg("--dry-run")
        .assert()
        .success()
        .stdout(contains("Job complete"));

    assert!(audit_path.exists(), "audit log should be written");
}

fn path(relative: &str) -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join(relative)
        .canonicalize()
        .expect("resolve path")
        .display()
        .to_string()
}
