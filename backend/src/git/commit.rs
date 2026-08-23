use std::path::Path;

use anyhow::{Context, Result};

/// Initialize a new git repository at `path`
pub fn init_repo(path: &Path) -> Result<git2::Repository> {
    if path.join(".git").exists() {
        // Already a repo — open it
        return git2::Repository::open(path).context("Failed to open existing git repo");
    }
    git2::Repository::init(path).context("Failed to init git repo")
}

/// Stage all files and commit with message
pub fn commit_all(repo: &git2::Repository, message: &str) -> Result<String> {
    // Stage all changes
    let mut index = repo.index().context("Failed to open index")?;
    index
        .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
        .context("Failed to stage files")?;
    index.write().context("Failed to write index")?;

    let oid = index.write_tree().context("Failed to write tree")?;
    let tree = repo.find_tree(oid).context("Failed to find tree")?;

    let signature = git2::Signature::now("dockpot", "dockpot@local")
        .context("Failed to create signature")?;

    let parent = match repo.head() {
        Ok(head) => {
            let commit_oid = head.target().context("No target on HEAD")?;
            Some(repo.find_commit(commit_oid).context("Failed to find parent commit")?)
        }
        Err(_) => None,
    };

    let commit_oid = match &parent {
        Some(p) => repo
            .commit(Some("HEAD"), &signature, &signature, message, &tree, &[p])
            .context("Failed to commit (with parent)")?,
        None => repo
            .commit(Some("HEAD"), &signature, &signature, message, &tree, &[])
            .context("Failed to commit (initial)")?,
    };

    Ok(commit_oid.to_string())
}

/// Get the current HEAD commit hash (if any)
pub fn head_commit(repo: &git2::Repository) -> Result<Option<String>> {
    match repo.head() {
        Ok(head) => {
            let oid = head.target().context("No target on HEAD")?;
            Ok(Some(oid.to_string()))
        }
        Err(_) => Ok(None),
    }
}

/// Check if there are uncommitted changes
pub fn has_uncommitted(repo: &git2::Repository) -> Result<bool> {
    let mut status_opts = git2::StatusOptions::new();
    status_opts.include_untracked(true);
    let statuses = repo
        .statuses(Some(&mut status_opts))
        .context("Failed to get status")?;
    Ok(!statuses.is_empty())
}