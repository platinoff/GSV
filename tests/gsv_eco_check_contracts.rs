//! GASV-G26: read-only ecosystem 1.100 eco-check scanner contracts.
//! Fixture-based, no dependency on the live S:/rust environment.

use std::path::PathBuf;

fn fixture(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("gsv_eco_check_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("docs/gsv")).unwrap();
    std::fs::create_dir_all(d.join("docs/vision")).unwrap();
    // Host product marker + toolchain pin.
    std::fs::write(
        d.join("Cargo.toml"),
        "[package]\nname = \"fake-gsv\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    std::fs::write(
        d.join("rust-toolchain.toml"),
        "[toolchain]\nchannel = \"stable-x86_64-pc-windows-gnu\"\n",
    )
    .unwrap();
    // Fake FM with 2 function rows.
    std::fs::write(
        d.join("docs/gsv/FUNCTION_MANAGEMENT.md"),
        "# FM\n| # | Функція | Модуль | Статус |\n|---|---|---|---|\n| 1 | a | b | І |\n| 2 | c | d | І |\n",
    )
    .unwrap();
    std::fs::write(
        d.join("docs/vision/manifest.json"),
        "{\"revision\":516,\"next\":\"PH-S3129\"}",
    )
    .unwrap();
    d
}

#[test]
fn eco_check_report_reads_pins_fm_and_vision() {
    let dir = fixture("basic");
    let r = gsv::boxes::eco_check::report(&dir);
    assert_eq!(r["toolchain"]["pin"], "stable-x86_64-pc-windows-gnu");
    assert_eq!(r["toolchain"]["ok_1_100"], true);
    assert_eq!(r["fm"]["row_count"], 2);
    assert_eq!(r["vision"]["rev"], 516);
    assert_eq!(r["ok"], true);
}

#[test]
fn eco_check_rejects_wrong_toolchain_pin() {
    let dir = fixture("badpin");
    std::fs::write(
        dir.join("rust-toolchain.toml"),
        "[toolchain]\nchannel = \"1.75.0\"\n",
    )
    .unwrap();
    let r = gsv::boxes::eco_check::report(&dir);
    assert_eq!(r["toolchain"]["ok_1_100"], false);
    assert_eq!(r["ok"], false);
}

#[test]
fn eco_check_is_read_only() {
    let dir = fixture("readonly");
    let before = std::fs::read_to_string(dir.join("docs/gsv/FUNCTION_MANAGEMENT.md")).unwrap();
    let _ = gsv::boxes::eco_check::report(&dir);
    let after = std::fs::read_to_string(dir.join("docs/gsv/FUNCTION_MANAGEMENT.md")).unwrap();
    assert_eq!(before, after);
}
