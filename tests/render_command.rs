use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use chrono::Days;
use git_snake::behavior::{Behavior, select_behavior};
use git_snake::coat::SnakeMorph;
use git_snake::history::{AccountSnapshot, ContributionDay};
use git_snake::snake::{SnakeDevelopment, derive_snake_development};
use git_snake::motion::frame_count;
use git_snake::output::{encode_gif, encode_png};
use git_snake::render::{SnakeFrame, render_snake, render_slither_frame};

const HISTORY_FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/account-with-history.json"
);

#[test]
fn render_writes_the_days_loop_still_and_preview_as_the_renderer_draws_them() {
    // Two quiet weeks make the snake rest, so the loop can't pass as the fallback cruise.
    let mut account_snapshot = read_fixture_snapshot();
    let contribution_history = account_snapshot.contribution_history.as_mut().unwrap();
    for day_offset in 1..=14 {
        contribution_history.days.push(ContributionDay {
            date: contribution_history.observed_on + Days::new(day_offset),
            contribution_count: 0,
        });
    }
    contribution_history.observed_on = contribution_history.days.last().unwrap().date;
    let scratch_directory = fresh_directory("render-default");
    let snapshot_path = scratch_directory.join("snapshot.json");
    fs::write(
        &snapshot_path,
        serde_json::to_string(&account_snapshot).unwrap(),
    )
    .unwrap();
    let output_directory = scratch_directory.join("snake");
    let command_output = run_git_snake(&[
        "render",
        snapshot_path.to_str().unwrap(),
        "--output-dir",
        output_directory.to_str().unwrap(),
    ]);
    assert_eq!(
        command_output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&command_output.stderr)
    );

    let snake_development = derive_snake_development(&account_snapshot).unwrap();
    let behavior = select_behavior(&snake_development);
    assert_eq!(behavior, Behavior::Rest);
    let slither_frames: Vec<SnakeFrame> = (0..frame_count(behavior))
        .map(|frame_index| {
            render_slither_frame(&snake_development, SnakeMorph::Normal, behavior, frame_index)
        })
        .collect();
    assert_eq!(
        fs::read(output_directory.join("snake.gif")).unwrap(),
        encode_gif(&slither_frames).unwrap()
    );
    assert_eq!(
        fs::read(output_directory.join("snake.png")).unwrap(),
        encode_png(&render_snake(&snake_development, SnakeMorph::Normal)).unwrap()
    );
    let preview_html = fs::read_to_string(output_directory.join("preview.html")).unwrap();
    assert!(preview_html.contains(r#"src="snake.gif""#));
    assert!(preview_html.contains(r#"src="snake.png""#));
    assert!(!preview_html.contains("<script"));
    assert!(preview_html.contains(&snake_development.observed_on.to_string()));
    assert!(command_output.stdout.is_empty());
}

#[test]
fn the_morph_option_redraws_the_snake_in_that_morph() {
    let output_directory = fresh_directory("render-pastel");
    let command_output = run_git_snake(&[
        "render",
        HISTORY_FIXTURE,
        "--output-dir",
        output_directory.to_str().unwrap(),
        "--morph",
        "pastel",
    ]);
    assert_eq!(command_output.status.code(), Some(0));
    let snake_development = read_fixture_development();
    let pastel_png = encode_png(&render_snake(&snake_development, SnakeMorph::Pastel)).unwrap();
    assert_ne!(
        pastel_png,
        encode_png(&render_snake(&snake_development, SnakeMorph::Normal)).unwrap()
    );
    assert_eq!(
        fs::read(output_directory.join("snake.png")).unwrap(),
        pastel_png
    );
    let preview_html = fs::read_to_string(output_directory.join("preview.html")).unwrap();
    assert!(preview_html.contains("pastel morph"));
}

#[test]
fn a_failed_render_exits_one_and_keeps_the_previous_files() {
    let output_directory = fresh_directory("render-failure");
    for file_name in ["snake.gif", "snake.png", "preview.html"] {
        fs::write(
            output_directory.join(file_name),
            format!("previous {file_name}"),
        )
        .unwrap();
    }
    let command_output = run_git_snake(&[
        "render",
        concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/account.json"),
        "--output-dir",
        output_directory.to_str().unwrap(),
    ]);
    assert_eq!(command_output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&command_output.stderr).starts_with("git-snake: "));
    for file_name in ["snake.gif", "snake.png", "preview.html"] {
        assert_eq!(
            fs::read_to_string(output_directory.join(file_name)).unwrap(),
            format!("previous {file_name}")
        );
    }
    assert_eq!(fs::read_dir(&output_directory).unwrap().count(), 3);
}

#[test]
fn render_usage_errors_exit_two_and_help_exits_zero() {
    for invalid_arguments in [
        &["render", HISTORY_FIXTURE][..],
        &["render", "--output-dir", "snake"],
        &[
            "render",
            HISTORY_FIXTURE,
            "--output-dir",
            "snake",
            "--morph",
            "red",
        ],
    ] {
        let command_output = run_git_snake(invalid_arguments);
        assert_eq!(
            command_output.status.code(),
            Some(2),
            "{invalid_arguments:?}"
        );
        assert!(String::from_utf8_lossy(&command_output.stderr).starts_with("Usage:"));
    }
    let help_output = run_git_snake(&["render", "--help"]);
    assert_eq!(help_output.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&help_output.stdout).contains("cinnamon"));
}

#[test]
fn version_prints_the_manifest_version_and_exits_zero() {
    let version_output = run_git_snake(&["--version"]);
    assert_eq!(version_output.status.code(), Some(0));
    assert_eq!(
        String::from_utf8_lossy(&version_output.stdout),
        format!("git-snake {}\n", env!("CARGO_PKG_VERSION"))
    );
}

fn read_fixture_snapshot() -> AccountSnapshot {
    serde_json::from_str(&fs::read_to_string(HISTORY_FIXTURE).unwrap()).unwrap()
}

fn read_fixture_development() -> SnakeDevelopment {
    derive_snake_development(&read_fixture_snapshot()).unwrap()
}

/// An empty directory of the given name under Cargo's scratch space for tests.
fn fresh_directory(directory_name: &str) -> PathBuf {
    let directory_path = Path::new(env!("CARGO_TARGET_TMPDIR")).join(directory_name);
    let _ = fs::remove_dir_all(&directory_path);
    fs::create_dir_all(&directory_path).unwrap();
    directory_path
}

fn run_git_snake(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_git-snake"))
        .args(arguments)
        .output()
        .unwrap()
}
