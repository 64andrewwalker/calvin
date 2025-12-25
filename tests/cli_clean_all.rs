//! Integration tests for `calvin clean --all` (Phase 2: Global Registry)

mod common;

use common::*;

#[test]
fn clean_all_removes_files_for_each_registered_project() {
    let env = TestEnv::builder().fresh_environment().without_git().build();

    let project_a = env.project_path("project-a");
    let project_b = env.project_path("project-b");
    std::fs::create_dir_all(&project_a).unwrap();
    std::fs::create_dir_all(&project_b).unwrap();
    std::fs::create_dir_all(project_a.join(".git")).unwrap();
    std::fs::create_dir_all(project_b.join(".git")).unwrap();

    env.write_project_file("project-a/.promptpack/policy.md", SIMPLE_POLICY);
    env.write_project_file("project-b/.promptpack/policy.md", SIMPLE_POLICY);

    let deploy_a = env.run_from(&project_a, &["deploy", "--yes", "--targets", "cursor"]);
    assert!(
        deploy_a.success,
        "deploy (project-a) failed:\n{}",
        deploy_a.combined_output()
    );
    let deploy_b = env.run_from(&project_b, &["deploy", "--yes", "--targets", "cursor"]);
    assert!(
        deploy_b.success,
        "deploy (project-b) failed:\n{}",
        deploy_b.combined_output()
    );

    let a_output = project_a.join(".cursor/rules/policy/RULE.md");
    let b_output = project_b.join(".cursor/rules/policy/RULE.md");
    assert!(a_output.exists(), "expected deploy output to exist");
    assert!(b_output.exists(), "expected deploy output to exist");

    // Run registry-wide clean.
    let clean = env.run(&["clean", "--all", "--yes"]);
    assert!(
        clean.success,
        "clean --all failed:\n{}",
        clean.combined_output()
    );

    assert!(
        !a_output.exists(),
        "expected project-a output to be removed"
    );
    assert!(
        !b_output.exists(),
        "expected project-b output to be removed"
    );
}
