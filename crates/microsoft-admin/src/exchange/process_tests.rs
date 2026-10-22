use super::process;
use crowsi_process_adapter::{Launch, ProcessBudget};
use std::time::{Duration, Instant};
use zeroize::Zeroizing;
fn launch(exe: &str, args: &[&str]) -> Launch {
    // Fixture only: finite pipe/file tests do not assert whole-tree memory enforcement.
    Launch::new(exe)
        .args(args.iter().copied())
        .budget(ProcessBudget {
            max_file_bytes: Some(0),
            ..Default::default()
        })
}
#[test]
fn exact_bounded_pipe_roundtrip_without_files_or_authentication() {
    let bytes = Zeroizing::new(b"synthetic pipe request\n".to_vec());
    assert_eq!(
        process::run(launch("/bin/cat", &[]), bytes, Duration::from_secs(3)).unwrap(),
        b"synthetic pipe request\n"
    );
}
#[test]
fn finite_oversized_output_is_rejected_and_owned_child_reaped() {
    let start = Instant::now();
    let r = process::run(
        launch("/usr/bin/head", &["-c", "1048577", "/dev/zero"]),
        Zeroizing::new(Vec::new()),
        Duration::from_secs(4),
    );
    assert!(r.is_err());
    assert!(start.elapsed() < Duration::from_secs(7));
}
#[test]
fn timeout_stops_only_the_owned_fixture_tree() {
    let start = Instant::now();
    let r = process::run(
        launch("/bin/sh", &["-c", "sleep 60 & wait"]),
        Zeroizing::new(Vec::new()),
        Duration::from_millis(150),
    );
    assert!(r.is_err());
    assert!(start.elapsed() < Duration::from_secs(4));
}
#[test]
fn invalid_launch_has_no_unbounded_fallback() {
    let r = process::run(
        Launch::new("relative-command"),
        Zeroizing::new(Vec::new()),
        Duration::from_secs(1),
    );
    assert!(r.is_err());
}
