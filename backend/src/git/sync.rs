use std::path::Path;

use anyhow::{Context, Result};

/// Initialize or open a git repository
pub fn init_repo(path: &Path) -> Result<git2::Repository> {
    if path.join(".git").exists() {
        git2::Repository::open(path).context("Failed to open existing git repo")
    } else {
        git2::Repository::init(path).context("Failed to init git repo")
    }
}

/// Stage all files and commit
pub fn commit_all(repo: &git2::Repository, message: &str) -> Result<String> {
    let mut index = repo.index().context("Failed to open index")?;
    index
        .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
        .context("Failed to stage files")?;
    index.write().context("Failed to write index")?;

    let oid = index.write_tree().context("Failed to write tree")?;
    let tree = repo.find_tree(oid).context("Failed to find tree")?;

    let signature =
        git2::Signature::now("dockpot", "dockpot@local").context("Failed to create signature")?;

    let parent = match repo.head() {
        Ok(head) => {
            let commit_oid = head.target().context("No target on HEAD")?;
            Some(
                repo.find_commit(commit_oid)
                    .context("Failed to find parent commit")?,
            )
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

/// Get HEAD commit hash
pub fn head_commit(repo: &git2::Repository) -> Result<Option<String>> {
    match repo.head() {
        Ok(head) => {
            let oid = head.target().context("No target on HEAD")?;
            Ok(Some(oid.to_string()))
        }
        Err(_) => Ok(None),
    }
}

/// Check for uncommitted changes
pub fn has_uncommitted(repo: &git2::Repository) -> Result<bool> {
    let mut opts = git2::StatusOptions::new();
    opts.include_untracked(true);
    let statuses = repo
        .statuses(Some(&mut opts))
        .context("Failed to get status")?;
    Ok(!statuses.is_empty())
}

/// Clone a remote repository
pub fn clone_remote(
    url: &str,
    path: &Path,
    branch: &str,
    _auth_token: Option<&str>,
) -> Result<git2::Repository> {
    let mut fetch_opts = git2::FetchOptions::new();
    fetch_opts.download_tags(git2::AutotagOption::All);

    let mut builder = git2::build::RepoBuilder::new();
    builder.branch(branch).fetch_options(fetch_opts);

    let repo = builder
        .clone(url, path)
        .context(format!("Failed to clone {} into {:?}", url, path))?;

    Ok(repo)
}

/// Fetch from remote origin
pub fn fetch(repo: &git2::Repository) -> Result<()> {
    let mut remote = repo.find_remote("origin").context("No 'origin' remote")?;
    let mut fetch_opts = git2::FetchOptions::new();
    fetch_opts.download_tags(git2::AutotagOption::All);

    remote
        .fetch(&["refs/heads/*:refs/heads/*"], Some(&mut fetch_opts), None)
        .context("Failed to fetch")?;

    Ok(())
}

/// Pull: fetch + fast-forward
pub fn pull(repo: &git2::Repository, branch: &str) -> Result<String> {
    fetch(repo)?;

    let remote_branch = format!("origin/{}", branch);
    let remote_ref = repo
        .find_reference(&remote_branch)
        .context(format!("Remote branch '{}' not found", remote_branch))?;
    let remote_commit = remote_ref
        .peel_to_commit()
        .context("Failed to peel remote ref")?;
    let remote_oid = remote_commit.id();

    // Fast-forward or reset to remote
    repo.find_reference("refs/heads/main")
        .or_else(|_| repo.find_reference("refs/heads/master"))
        .ok();

    // Set head directly (fast-forward)
    repo.set_head_detached(remote_oid)
        .context("Failed to set HEAD detached")?;

    // Create/update the local branch
    repo.branch(branch, &remote_commit, false).ok();

    // Re-attach HEAD
    repo.set_head(&format!("refs/heads/{}", branch))
        .context("Failed to set HEAD")?;

    let tree = remote_commit.tree().context("Failed to get remote tree")?;
    repo.checkout_tree(&tree.into_object(), None)
        .context("Failed to checkout")?;

    Ok(format!("Updated to {}", remote_oid))
}

/// Push to remote
pub fn push(repo: &git2::Repository, branch: &str) -> Result<()> {
    let mut remote = repo.find_remote("origin").context("No 'origin' remote")?;

    let mut push_opts = git2::PushOptions::new();
    let refspec = format!("refs/heads/{}:refs/heads/{}", branch, branch);

    remote
        .push(&[&refspec], Some(&mut push_opts))
        .context("Failed to push")?;

    Ok(())
}

/// Get working tree diff against HEAD
pub fn get_diff(repo: &git2::Repository) -> Result<crate::models::GitDiff> {
    let tree = match repo.head() {
        Ok(head) => head.peel_to_commit().ok().and_then(|c| c.tree().ok()),
        Err(_) => None,
    };

    let mut opts = git2::DiffOptions::new();
    opts.include_untracked(true);
    opts.recurse_untracked_dirs(true);

    let diff = repo
        .diff_tree_to_workdir(tree.as_ref(), Some(&mut opts))
        .context("Failed to diff")?;

    let mut files_changed = Vec::new();
    let mut additions = 0u64;
    let mut deletions = 0u64;
    let mut diff_text = String::new();

    diff.foreach(
        &mut |file, _| {
            if let Some(path) = file.new_file().path() {
                files_changed.push(path.to_string_lossy().to_string());
            }
            true
        },
        None,
        None,
        Some(&mut |_delta, _hunk, _line| true),
    )
    .ok();

    diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
        match line.origin() {
            '+' => additions += 1,
            '-' => deletions += 1,
            _ => {}
        }
        if let Ok(text) = std::str::from_utf8(line.content()) {
            diff_text.push_str(text);
        }
        true
    })
    .ok();

    Ok(crate::models::GitDiff {
        files_changed,
        additions,
        deletions,
        diff_text,
    })
}

/// Determine sync status (local vs remote)
pub fn sync_status(repo: &git2::Repository, branch: &str) -> Result<String> {
    fetch(repo)?;

    let head_oid = match repo.head() {
        Ok(h) => h.target().context("No HEAD target")?,
        Err(_) => return Ok("pending".to_string()),
    };

    let remote_ref = match repo.find_reference(&format!("origin/{}", branch)) {
        Ok(r) => r,
        Err(_) => return Ok("pending".to_string()),
    };

    let remote_oid = match remote_ref.target() {
        Some(oid) => oid,
        None => return Ok("pending".to_string()),
    };

    if head_oid == remote_oid {
        if has_uncommitted(repo)? {
            return Ok("pending".to_string());
        }
        return Ok("synced".to_string());
    }

    match repo.merge_base(head_oid, remote_oid) {
        Ok(base) if base == remote_oid => Ok("pending".to_string()),
        Ok(base) if base == head_oid => Ok("pending".to_string()),
        Ok(_) => Ok("conflict".to_string()),
        Err(_) => Ok("conflict".to_string()),
    }
}
