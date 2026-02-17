#![allow(missing_docs)]

use std::process::Command;
use tempfile::TempDir;

use flow::cycle::config::FlowConfig;
use flow::cycle::executor::CycleExecutor;
use flow::cycle::rules::find_triggered_cycles;
use flow::log::jsonl::JsonlLogger;
use flow::log::CycleOutcome;

const TEST_CONFIG: &str = r#"
[global]
permissions = ["Read", "Edit(./src/**)"]

[[cycle]]
name = "coding"
description = "Pick a task and implement with TDD"
prompt = "echo integration test"
permissions = ["Edit(./tests/**)", "Bash(cargo test *)"]
after = []
context = "summaries"

[[cycle]]
name = "gardening"
description = "Deps, refactoring, docs"
prompt = "echo gardening"
permissions = ["Edit(./Cargo.toml)"]
after = ["coding"]
context = "none"

[[cycle]]
name = "review"
description = "Code review"
prompt = "echo review"
permissions = []
after = []
context = "full"
"#;

/// Integration test: Full end-to-end coding cycle execution.
///
/// Tests the complete data flow: config → prepare → execute (with mock command)
/// → log outcome → verify JSONL output.
#[tokio::test]
async fn test_coding_cycle_end_to_end() {
    // Setup: parse config and create logger in temp dir
    let config = FlowConfig::parse(TEST_CONFIG).unwrap();
    let temp_dir = TempDir::new().unwrap();
    let logger = JsonlLogger::new(temp_dir.path()).unwrap();

    // Step 1: Prepare the cycle (validates config + resolves permissions)
    let executor = CycleExecutor::new(config);
    let prepared = executor.prepare("coding").unwrap();

    assert_eq!(prepared.cycle_name, "coding");
    assert_eq!(
        prepared.permissions,
        vec![
            "Read",
            "Edit(./src/**)",
            "Edit(./tests/**)",
            "Bash(cargo test *)",
        ]
    );

    // Step 2: Execute a mock command (simulates Claude Code returning successfully)
    let mut cmd = Command::new("echo");
    cmd.arg("task completed successfully");

    let (stdout, stderr, exit_code, duration_secs) =
        flow::cycle::executor::run_command(cmd).await.unwrap();

    assert_eq!(stdout, "task completed successfully");
    assert!(stderr.is_empty());
    assert_eq!(exit_code, Some(0));

    // Step 3: Build CycleResult and log it
    let result = flow::CycleResult {
        cycle_name: prepared.cycle_name.clone(),
        success: exit_code == Some(0),
        exit_code,
        stderr,
        duration_secs,
        result_text: None,
        num_turns: None,
        total_cost_usd: None,
        permission_denial_count: None,
        permission_denials: None,
        files_changed: vec![],
        tests_passed: 0,
    };

    let outcome = CycleOutcome {
        iteration: 1,
        cycle: result.cycle_name.clone(),
        timestamp: chrono::Utc::now(),
        outcome: if result.success {
            "Completed successfully".to_string()
        } else {
            format!("Failed with exit code {:?}", result.exit_code)
        },
        files_changed: vec![],
        tests_passed: 0,
        duration_secs: result.duration_secs,
        num_turns: None,
        total_cost_usd: None,
        permission_denial_count: None,
        permission_denials: None,
        steps: None,
    };

    logger.append(&outcome).unwrap();

    // Step 4: Verify JSONL log was written correctly
    let entries = logger.read_all().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].cycle, "coding");
    assert_eq!(entries[0].iteration, 1);
    assert_eq!(entries[0].outcome, "Completed successfully");

    // Step 5: Verify cycle completed successfully
    assert!(result.success);
}

/// Integration test: Failed cycle is logged correctly.
///
/// Tests the data flow when the subprocess fails.
#[tokio::test]
async fn test_failed_cycle_logged_correctly() {
    let config = FlowConfig::parse(TEST_CONFIG).unwrap();
    let temp_dir = TempDir::new().unwrap();
    let logger = JsonlLogger::new(temp_dir.path()).unwrap();

    let executor = CycleExecutor::new(config);
    let prepared = executor.prepare("coding").unwrap();

    // Execute a command that fails
    let mut cmd = Command::new("sh");
    cmd.arg("-c").arg("echo 'error occurred' >&2; exit 1");

    let (_stdout, stderr, exit_code, duration_secs) =
        flow::cycle::executor::run_command(cmd).await.unwrap();

    assert_eq!(exit_code, Some(1));
    assert_eq!(stderr, "error occurred");

    let result = flow::CycleResult {
        cycle_name: prepared.cycle_name.clone(),
        success: exit_code == Some(0),
        exit_code,
        stderr,
        duration_secs,
        result_text: None,
        num_turns: None,
        total_cost_usd: None,
        permission_denial_count: None,
        permission_denials: None,
        files_changed: vec![],
        tests_passed: 0,
    };

    let outcome = CycleOutcome {
        iteration: 1,
        cycle: result.cycle_name.clone(),
        timestamp: chrono::Utc::now(),
        outcome: format!(
            "Failed with exit code {}",
            result
                .exit_code
                .map_or_else(|| "unknown".to_string(), |c| c.to_string())
        ),
        files_changed: vec![],
        tests_passed: 0,
        duration_secs: result.duration_secs,
        num_turns: None,
        total_cost_usd: None,
        permission_denial_count: None,
        permission_denials: None,
        steps: None,
    };

    logger.append(&outcome).unwrap();

    let entries = logger.read_all().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].cycle, "coding");
    assert_eq!(entries[0].outcome, "Failed with exit code 1");
    assert!(!result.success);
}

/// Integration test: Gardening auto-triggers after coding.
///
/// Tests the full flow: coding cycle succeeds → rules engine finds
/// gardening as dependent → gardening executes → both logged to JSONL.
#[tokio::test]
async fn test_gardening_auto_triggers_after_coding() {
    let config = FlowConfig::parse(TEST_CONFIG).unwrap();
    let temp_dir = TempDir::new().unwrap();
    let logger = JsonlLogger::new(temp_dir.path()).unwrap();
    let mut iteration: u32 = 1;

    // Execute coding cycle (mock success)
    let executor = CycleExecutor::new(config.clone());
    let coding_prepared = executor.prepare("coding").unwrap();

    let mut cmd = Command::new("echo");
    cmd.arg("coding done");
    let (_stdout, stderr, exit_code, duration_secs) =
        flow::cycle::executor::run_command(cmd).await.unwrap();

    let coding_result = flow::CycleResult {
        cycle_name: coding_prepared.cycle_name.clone(),
        success: exit_code == Some(0),
        exit_code,
        stderr,
        duration_secs,
        result_text: None,
        num_turns: None,
        total_cost_usd: None,
        permission_denial_count: None,
        permission_denials: None,
        files_changed: vec![],
        tests_passed: 0,
    };

    // Log coding result
    let coding_outcome = CycleOutcome {
        iteration,
        cycle: coding_result.cycle_name.clone(),
        timestamp: chrono::Utc::now(),
        outcome: "Completed successfully".to_string(),
        files_changed: vec![],
        tests_passed: 0,
        duration_secs: coding_result.duration_secs,
        num_turns: None,
        total_cost_usd: None,
        permission_denial_count: None,
        permission_denials: None,
        steps: None,
    };
    logger.append(&coding_outcome).unwrap();
    iteration += 1;

    // Find triggered cycles after coding (pass log for frequency checking)
    let log_entries = logger.read_all().unwrap();
    let triggered = find_triggered_cycles(&config, &coding_result.cycle_name, &log_entries);
    assert_eq!(
        triggered,
        vec!["gardening"],
        "Gardening should auto-trigger after coding"
    );

    // Execute each triggered cycle
    for dep_cycle in &triggered {
        let dep_prepared = executor.prepare(dep_cycle).unwrap();

        let mut cmd = Command::new("echo");
        cmd.arg("gardening done");
        let (_stdout, stderr, exit_code, duration_secs) =
            flow::cycle::executor::run_command(cmd).await.unwrap();

        let dep_result = flow::CycleResult {
            cycle_name: dep_prepared.cycle_name.clone(),
            success: exit_code == Some(0),
            exit_code,
            stderr,
            duration_secs,
            result_text: None,
            num_turns: None,
            total_cost_usd: None,
            permission_denial_count: None,
            permission_denials: None,
            files_changed: vec![],
            tests_passed: 0,
        };

        let dep_outcome = CycleOutcome {
            iteration,
            cycle: dep_result.cycle_name.clone(),
            timestamp: chrono::Utc::now(),
            outcome: "Completed successfully".to_string(),
            files_changed: vec![],
            tests_passed: 0,
            duration_secs: dep_result.duration_secs,
            num_turns: None,
            total_cost_usd: None,
            permission_denial_count: None,
            permission_denials: None,
            steps: None,
        };
        logger.append(&dep_outcome).unwrap();
        iteration += 1;

        assert!(dep_result.success, "Dependent cycle should succeed");
    }

    // Verify both cycles were logged
    let entries = logger.read_all().unwrap();
    assert_eq!(
        entries.len(),
        2,
        "Both coding and gardening should be logged"
    );
    assert_eq!(entries[0].cycle, "coding");
    assert_eq!(entries[0].iteration, 1);
    assert_eq!(entries[1].cycle, "gardening");
    assert_eq!(entries[1].iteration, 2);
}

/// Integration test: Config loads from file and cycle executes.
///
/// Tests the file-based config loading path used in the real CLI.
#[tokio::test]
async fn test_config_from_file_and_execute() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("cycles.toml");
    std::fs::write(&config_path, TEST_CONFIG).unwrap();

    let config = FlowConfig::from_path(&config_path).unwrap();
    let executor = CycleExecutor::new(config);

    // Prepare cycle - proves config loading + validation works
    let prepared = executor.prepare("coding").unwrap();
    assert_eq!(prepared.cycle_name, "coding");
    // coding has context = "summaries", so even with empty log the prompt has context prepended
    assert!(
        prepared.prompt.contains("echo integration test"),
        "Prompt should contain original text: {}",
        prepared.prompt
    );

    // Execute with mock command
    let mut cmd = Command::new("echo");
    cmd.arg("file-based config works");
    let (stdout, _stderr, exit_code, _duration) =
        flow::cycle::executor::run_command(cmd).await.unwrap();

    assert_eq!(stdout, "file-based config works");
    assert_eq!(exit_code, Some(0));
}

/// Integration test: Multiple iterations with logging.
///
/// Simulates multiple cycle iterations and verifies the JSONL log
/// accumulates correctly.
#[tokio::test]
async fn test_multiple_iterations_logged() {
    let config = FlowConfig::parse(TEST_CONFIG).unwrap();
    let temp_dir = TempDir::new().unwrap();
    let logger = JsonlLogger::new(temp_dir.path()).unwrap();
    let executor = CycleExecutor::new(config);

    let cycle_names = ["coding", "gardening", "review"];

    for (i, cycle_name) in cycle_names.iter().enumerate() {
        let prepared = executor.prepare(cycle_name).unwrap();

        let mut cmd = Command::new("echo");
        cmd.arg(format!("{cycle_name} iteration"));
        let (_stdout, stderr, exit_code, duration_secs) =
            flow::cycle::executor::run_command(cmd).await.unwrap();

        let result = flow::CycleResult {
            cycle_name: prepared.cycle_name.clone(),
            success: exit_code == Some(0),
            exit_code,
            stderr,
            duration_secs,
            result_text: None,
            num_turns: None,
            total_cost_usd: None,
            permission_denial_count: None,
            permission_denials: None,
            files_changed: vec![],
            tests_passed: 0,
        };

        let outcome = CycleOutcome {
            iteration: u32::try_from(i + 1).unwrap(),
            cycle: result.cycle_name.clone(),
            timestamp: chrono::Utc::now(),
            outcome: "Completed successfully".to_string(),
            files_changed: vec![],
            tests_passed: 0,
            duration_secs: result.duration_secs,
            num_turns: None,
            total_cost_usd: None,
            permission_denial_count: None,
            permission_denials: None,
            steps: None,
        };
        logger.append(&outcome).unwrap();
    }

    let entries = logger.read_all().unwrap();
    assert_eq!(entries.len(), 3);
    assert_eq!(entries[0].cycle, "coding");
    assert_eq!(entries[1].cycle, "gardening");
    assert_eq!(entries[2].cycle, "review");
    assert_eq!(entries[0].iteration, 1);
    assert_eq!(entries[1].iteration, 2);
    assert_eq!(entries[2].iteration, 3);
}

/// Integration test: Unknown cycle name is rejected.
///
/// Tests that the prepare step properly validates cycle names.
#[test]
fn test_unknown_cycle_rejected() {
    let config = FlowConfig::parse(TEST_CONFIG).unwrap();
    let executor = CycleExecutor::new(config);

    let result = executor.prepare("nonexistent");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Unknown cycle"),);
}
