use crate::app::CommandExt as _;
use anyhow::{anyhow, bail, Result};
use git2::{BranchType, ErrorCode, Repository};
use std::{collections::HashSet, process::Command};

pub fn current_branch() -> Result<String> {
    let output = run_and_check_output(&["rev-parse", "--abbrev-ref", "HEAD"])?;
    Ok(output.trim_end().to_string())
}

pub fn checkout_or_create_branch(branch_name: &str) -> Result<()> {
    if branch_exists(branch_name)? {
        checkout_branch(branch_name)?;
    } else {
        create_branch(branch_name)?;
    }
    Ok(())
}

pub fn merge_branch(branch_name: &str) -> Result<()> {
    let _output = run_and_check_output(&["merge", "--ff", branch_name])?;
    Ok(())
}

pub fn tag_version(version: &str) -> Result<()> {
    let _output = run_and_check_output(&["tag", "--annotate", version, "--message", version])?;
    Ok(())
}

pub fn push_branch(branch_name: &str) -> Result<()> {
    let _output = run_and_check_output(&["push", "origin", branch_name])?;
    Ok(())
}

pub fn changed_files() -> Result<Vec<String>> {
    let mut files = HashSet::new();

    // Committed e.g.:
    // A   relative/path/to/file.added
    // M   relative/path/to/file.modified
    let output = run_and_check_output(&["diff", "--name-status", "origin/master..."])?;
    for line in output.lines() {
        if !is_warning_line(line) {
            if let Some((_, path)) = line.split_once('\t') {
                files.insert(path.to_string());
            }
        }
    }

    // Tracked
    let output = run_and_check_output(&["diff", "--name-only", "HEAD"])?;
    for line in output.lines() {
        if !is_warning_line(line) {
            files.insert(line.to_string());
        }
    }

    // Untracked
    let output = run_and_check_output(&["ls-files", "--others", "--exclude-standard"])?;
    for line in output.lines() {
        files.insert(line.to_string());
    }

    let mut sorted = Vec::from_iter(files);
    sorted.sort();

    Ok(sorted)
}

pub fn list_files() -> Result<Vec<String>> {
    Ok(run_and_check_output(&["ls-files"])?
        .lines()
        .map(str::to_owned)
        .collect())
}

pub fn get_git_sha() -> Result<String> {
    run_and_check_output(&["rev-parse", "--short", "HEAD"])
        .map(|output| output.trim_end().to_string())
}

/// Get a list of files that have been modified, as a vector of strings
pub fn get_modified_files() -> Result<Vec<String>> {
    let args = vec![
        "ls-files",
        "--full-name",
        "--modified",
        "--others",
        "--exclude-standard",
    ];
    Ok(run_and_check_output(&args)?
        .lines()
        .map(str::to_owned)
        .collect())
}

pub fn set_config_value(key: &str, value: &str) -> Result<String> {
    Command::new("git")
        .args(["config", key, value])
        .stdout(std::process::Stdio::null())
        .check_output()
}

/// Checks if the current directory's repo is clean
pub fn check_git_repository_clean() -> Result<bool> {
    Ok(Command::new("git")
        .args(["diff-index", "--quiet", "HEAD"])
        .stdout(std::process::Stdio::null())
        .status()
        .map(|status| status.success())?)
}

pub fn add_files_in_current_dir() -> Result<String> {
    Command::new("git").args(["add", "."]).check_output()
}

/// Commits changes from the current repo
pub fn commit(commit_message: &str) -> Result<String> {
    Command::new("git")
        .args(["commit", "--all", "--message", commit_message])
        .check_output()
}

/// Pushes changes from the current repo
pub fn push() -> Result<String> {
    Command::new("git").args(["push"]).check_output()
}

pub fn push_and_set_upstream(branch_name: &str) -> Result<String> {
    Command::new("git")
        .args(["push", "-u", "origin", branch_name])
        .check_output()
}

pub fn clone(repo_url: &str) -> Result<String> {
    // We cannot use capture_output since this will need to run in the CWD
    Command::new("git").args(["clone", repo_url]).check_output()
}

/// Walks up from the current working directory until it finds a `.git`
/// and opens that repo.  Panics (Err) if none is found.
fn find_repo() -> Result<Repository, git2::Error> {
    Repository::discover(".")
}

pub fn branch_exists(branch: &str) -> Result<bool> {
    let repo = find_repo()?;

    // Do the lookup inside its own scope so the temporary Branch<'_> is dropped
    // before we try to drop `repo` at function exit.
    let exists = {
        match repo.find_branch(branch, BranchType::Local) {
            Ok(_) => true,
            Err(e) if e.code() == ErrorCode::NotFound => false,
            Err(e) => bail!(e),
        }
    };

    Ok(exists)
}
pub fn checkout_branch(branch_name: &str) -> Result<()> {
    let _output = run_and_check_output(&["checkout", branch_name])?;
    Ok(())
}

pub fn checkout_main_branch() -> Result<()> {
    let _output = run_and_check_output(&["switch", "master"])?;
    Ok(())
}

pub fn create_branch(branch_name: &str) -> Result<()> {
    let repo = find_repo()?;

    let head_ref = repo.head()?;
    let target_oid = head_ref
        .target()
        .ok_or_else(|| anyhow!("HEAD is not pointing at a valid commit"))?;
    let target_commit = repo.find_commit(target_oid)?;

    let branch = repo.branch(branch_name, &target_commit, false)?;
    let reference = branch.into_reference();
    let full_ref_name = reference
        .name()
        .ok_or_else(|| git2::Error::from_str("branch reference has no name"))?;
    repo.set_head(full_ref_name)?;
    repo.checkout_head(None)?;

    Ok(())
}

pub fn run_and_check_output(args: &[&str]) -> Result<String> {
    Command::new("git").in_repo().args(args).check_output()
}

fn is_warning_line(line: &str) -> bool {
    line.starts_with("warning: ") || line.contains("original line endings")
}

/// Returns a list of tracked files. If `pattern` is specified, it filters using that pattern.
pub fn git_ls_files(pattern: Option<&str>) -> Result<Vec<String>> {
    let args = match pattern {
        Some(p) => vec!["ls-files", p],
        None => vec!["ls-files"],
    };

    let output = run_and_check_output(&args)?;
    Ok(output.lines().map(str::to_owned).collect())
}
