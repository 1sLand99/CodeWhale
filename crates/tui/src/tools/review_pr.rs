//! One immutable PR input for CLI, interactive and model-tool reviews.
//! Replaces their separate `gh pr diff` readers; large PRs use local pinned
//! Git objects without fetching, checking out or executing pull-request code.

use std::io::Read;
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use wait_timeout::ChildExt;

use crate::dependencies::{ExternalTool, Gh, Git};

const MAX_OUTPUT_BYTES: usize = 8 * 1024 * 1024;
const VIEW_FIELDS: &str =
    "title,body,baseRefName,headRefName,url,headRefOid,baseRefOid,changedFiles,additions,deletions";

#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct GhPullRequest {
    pub title: String,
    pub body: String,
    #[serde(rename = "baseRefName")]
    pub base: String,
    #[serde(rename = "headRefName")]
    pub head: String,
    pub url: String,
    #[serde(rename = "headRefOid")]
    pub head_sha: String,
    #[serde(rename = "baseRefOid")]
    pub base_sha: String,
    #[serde(rename = "changedFiles")]
    pub changed_files: usize,
    pub additions: usize,
    pub deletions: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Program {
    Gh,
    Git,
}

fn pr_args(action: &str, number: u32, repo: Option<&str>) -> Vec<String> {
    let mut args = vec!["pr".into(), action.into(), number.to_string()];
    if let Some(repo) = repo {
        args.extend(["--repo".into(), repo.into()]);
    }
    args
}

fn commit_id(value: &str) -> bool {
    matches!(value.len(), 40 | 64) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn view_with(
    number: u32,
    repo: Option<&str>,
    run: &mut impl FnMut(Program, &[String]) -> Result<String>,
) -> Result<GhPullRequest> {
    if number == 0 {
        bail!("A positive pull request number is required");
    }
    let mut args = pr_args("view", number, repo);
    args.extend(["--json".into(), VIEW_FIELDS.into()]);
    let view: GhPullRequest = serde_json::from_str(&run(Program::Gh, &args)?)
        .context("gh pr view returned incomplete PR metadata")?;
    if !commit_id(&view.base_sha) || !commit_id(&view.head_sha) {
        bail!("gh pr view did not return exact base and head commit IDs");
    }
    Ok(view)
}

pub(crate) fn fetch_view(
    number: u32,
    repo: Option<&str>,
    workspace: &Path,
) -> Result<GhPullRequest> {
    view_with(number, repo, &mut |program, args| {
        run_command(workspace, program, args)
    })
}

fn same_revision(expected: &GhPullRequest, current: &GhPullRequest) -> Result<()> {
    if expected.head_sha != current.head_sha
        || expected.base_sha != current.base_sha
        || expected.changed_files != current.changed_files
        || expected.additions != current.additions
        || expected.deletions != current.deletions
        || expected.url != current.url
    {
        bail!(
            "Pull request changed during review; no current-PR review or receipt can be accepted. Run the review again."
        );
    }
    Ok(())
}

pub(crate) fn ensure_current(
    number: u32,
    repo: Option<&str>,
    workspace: &Path,
    expected: &GhPullRequest,
) -> Result<()> {
    same_revision(expected, &fetch_view(number, repo, workspace)?)
}

/// A PR review must never turn its configured input budget into a partial
/// review or a receipt that appears to cover the whole PR.
pub(crate) fn ensure_input_fits(diff: &str, max_chars: usize) -> Result<()> {
    let chars = diff.chars().count();
    if chars > max_chars {
        bail!(
            "Complete PR diff requires {chars} characters, exceeding the review limit of {max_chars}. No review was run or posted. Increase max_chars/--max-chars only if the selected model can accept the complete input."
        );
    }
    Ok(())
}

fn complete_file_set(diff: &str, view: &GhPullRequest) -> Result<()> {
    let mut files = 0;
    let (mut additions, mut deletions) = (0, 0);
    let mut remaining = (0_u32, 0_u32);
    let mut has_patch = false;
    for line in diff.lines() {
        if remaining != (0, 0) {
            match line.as_bytes().first() {
                Some(b'+') if remaining.1 > 0 => {
                    remaining.1 -= 1;
                    additions += 1;
                }
                Some(b'-') if remaining.0 > 0 => {
                    remaining.0 -= 1;
                    deletions += 1;
                }
                Some(b' ') if remaining.0 > 0 && remaining.1 > 0 => {
                    remaining.0 -= 1;
                    remaining.1 -= 1;
                }
                Some(b'\\') => {}
                _ => bail!("Incomplete PR diff: a text hunk is truncated or malformed"),
            }
            continue;
        }
        if line.starts_with("diff --git ") {
            if files > 0 && !has_patch {
                bail!("Incomplete PR diff: a file patch is missing");
            }
            files += 1;
            has_patch = false;
        } else if let Some((_, old, new)) = super::review_hunks::parse_hunk_header(line) {
            remaining = (old, new);
            has_patch = true;
        } else if line.starts_with("@@") {
            bail!("Incomplete PR diff: malformed hunk header");
        } else if [
            "new file mode ",
            "deleted file mode ",
            "old mode ",
            "new mode ",
            "rename from ",
            "rename to ",
            "GIT binary patch",
        ]
        .iter()
        .any(|prefix| line.starts_with(prefix))
        {
            has_patch = true;
        } else if line.starts_with("Binary files ") {
            bail!(
                "PR diff contains a missing binary patch; complete local Git objects are required"
            );
        }
    }
    if remaining != (0, 0)
        || !has_patch
        || files == 0
        || files != view.changed_files
        || additions != view.additions
        || deletions != view.deletions
    {
        bail!(
            "Incomplete PR diff: received {files} file patches, {additions} additions and {deletions} deletions; expected {}, {} and {}. No partial review is accepted.",
            view.changed_files,
            view.additions,
            view.deletions
        );
    }
    Ok(())
}

fn diff_with(
    number: u32,
    repo: Option<&str>,
    view: &GhPullRequest,
    run: &mut impl FnMut(Program, &[String]) -> Result<String>,
) -> Result<String> {
    // GitHub's diff representation refuses PRs with more than 300 files.
    // Preserve remote-only small-PR usage, but never rely on that limit for
    // completeness: also check the metadata's changed-file count.
    let remote = if view.changed_files <= 300 {
        run(Program::Gh, &pr_args("diff", number, repo)).and_then(|diff| {
            complete_file_set(&diff, view)?;
            Ok(diff)
        })
    } else {
        Err(anyhow::anyhow!("GitHub diff exceeds its 300-file limit"))
    };
    let diff = match remote {
        Ok(diff) => diff,
        Err(remote_error) => {
            let local: Result<String> = (|| {
                let shallow = run(
                    Program::Git,
                    &["rev-parse".into(), "--is-shallow-repository".into()],
                )?;
                if shallow.trim() != "false" {
                    bail!("A full Git history is required to establish the PR merge base");
                }
                let base = run(
                    Program::Git,
                    &[
                        "merge-base".into(),
                        "--all".into(),
                        view.base_sha.clone(),
                        view.head_sha.clone(),
                    ],
                )?;
                let base = base.trim();
                if !commit_id(base) {
                    bail!("The pinned PR commits do not have one available merge base");
                }
                let diff = run(
                    Program::Git,
                    &[
                        "diff".into(),
                        "--no-ext-diff".into(),
                        "--no-textconv".into(),
                        "--no-color".into(),
                        "--no-relative".into(),
                        "--binary".into(),
                        "--full-index".into(),
                        "--find-renames=50%".into(),
                        "--src-prefix=a/".into(),
                        "--dst-prefix=b/".into(),
                        "--ignore-submodules=none".into(),
                        "--submodule=short".into(),
                        base.into(),
                        view.head_sha.clone(),
                        "--".into(),
                    ],
                )?;
                complete_file_set(&diff, view)?;
                Ok(diff)
            })();
            local.with_context(|| format!("Cannot obtain the complete PR diff ({remote_error}). Make the exact base {} and head {} commits and their full history available in this repository (CI: fetch-depth: 0 and fetch the PR head ref). No fetch or checkout was performed.", view.base_sha, view.head_sha))?
        }
    };
    same_revision(view, &view_with(number, repo, run)?)?;
    Ok(diff)
}

pub(crate) fn fetch_diff(
    number: u32,
    repo: Option<&str>,
    workspace: &Path,
    view: &GhPullRequest,
) -> Result<String> {
    diff_with(number, repo, view, &mut |program, args| {
        run_command(workspace, program, args)
    })
}

fn read_bounded(reader: impl Read, limit: usize) -> std::io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    reader.take(limit as u64 + 1).read_to_end(&mut bytes)?;
    Ok(bytes)
}

fn run_command(workspace: &Path, program: Program, args: &[String]) -> Result<String> {
    let mut command = match program {
        Program::Gh => Gh::command(),
        Program::Git => Git::command(),
    }
    .context("PR review requires Git and GitHub CLI on PATH")?;
    command
        .args(args)
        .current_dir(workspace)
        .env("GIT_NO_REPLACE_OBJECTS", "1")
        .env("GIT_NO_LAZY_FETCH", "1")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GH_PROMPT_DISABLED", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .context("Failed to start PR input command")?;
    let stdout = child
        .stdout
        .take()
        .context("PR command stdout unavailable")?;
    let stderr = child
        .stderr
        .take()
        .context("PR command stderr unavailable")?;
    let stdout = std::thread::spawn(move || read_bounded(stdout, MAX_OUTPUT_BYTES));
    let stderr = std::thread::spawn(move || read_bounded(stderr, 64 * 1024));
    let status = match child.wait_timeout(Duration::from_secs(60))? {
        Some(status) => status,
        None => {
            let _ = child.kill();
            let _ = child.wait();
            bail!("PR input command timed out; no partial output was accepted");
        }
    };
    let stdout = stdout
        .join()
        .map_err(|_| anyhow::anyhow!("PR stdout reader failed"))??;
    let stderr = stderr
        .join()
        .map_err(|_| anyhow::anyhow!("PR stderr reader failed"))??;
    if stdout.len() > MAX_OUTPUT_BYTES || stderr.len() > 64 * 1024 {
        bail!(
            "PR input exceeds the bounded capture limit (8 MiB diff); no partial output was accepted"
        );
    }
    if !status.success() {
        bail!(
            "PR input command failed: {}",
            String::from_utf8_lossy(&stderr).trim()
        );
    }
    String::from_utf8(stdout).context("PR diff is not valid UTF-8; no lossy review is accepted")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view(files: usize) -> GhPullRequest {
        GhPullRequest {
            title: "Fixture".into(),
            body: String::new(),
            base: "main".into(),
            head: "feature".into(),
            url: "https://github.com/example/repo/pull/6002".into(),
            base_sha: "a".repeat(40),
            head_sha: "b".repeat(40),
            changed_files: files,
            additions: files,
            deletions: 0,
        }
    }

    fn metadata(view: &GhPullRequest) -> String {
        serde_json::json!({
            "title": view.title, "body": view.body, "baseRefName": view.base,
            "headRefName": view.head, "url": view.url, "headRefOid": view.head_sha,
            "baseRefOid": view.base_sha, "changedFiles": view.changed_files,
            "additions": view.additions, "deletions": view.deletions,
        })
        .to_string()
    }

    fn patch(name: &str) -> String {
        format!(
            "diff --git a/{name} b/{name}\nnew file mode 100644\n--- /dev/null\n+++ b/{name}\n@@ -0,0 +1 @@\n+complete\n"
        )
    }

    fn git(workspace: &Path, args: &[&str]) -> String {
        run_command(
            workspace,
            Program::Git,
            &args.iter().map(|arg| (*arg).into()).collect::<Vec<_>>(),
        )
        .expect("local Git fixture")
    }

    fn repository() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        git(dir.path(), &["init", "--template="]);
        git(dir.path(), &["config", "user.name", "Review Fixture"]);
        git(
            dir.path(),
            &["config", "user.email", "review@example.invalid"],
        );
        git(dir.path(), &["config", "commit.gpgsign", "false"]);
        let hooks = dir.path().join("empty-hooks");
        std::fs::create_dir(&hooks).unwrap();
        git(
            dir.path(),
            &["config", "core.hooksPath", hooks.to_str().unwrap()],
        );
        dir
    }

    #[test]
    fn large_pr_uses_all_pinned_git_patches_including_binary_not_the_checkout() {
        let dir = repository();
        git(dir.path(), &["commit", "--allow-empty", "-m", "base"]);
        let base = git(dir.path(), &["rev-parse", "HEAD"]).trim().to_string();
        for i in 0..301 {
            std::fs::write(
                dir.path().join(format!("file-{i}.txt")),
                format!("file {i}\n"),
            )
            .unwrap();
        }
        std::fs::write(dir.path().join("binary.dat"), b"\0\x01\x02\xff").unwrap();
        git(dir.path(), &["add", "*.txt", "binary.dat"]);
        git(dir.path(), &["commit", "-m", "PR head"]);
        let head = git(dir.path(), &["rev-parse", "HEAD"]).trim().to_string();
        std::fs::write(
            dir.path().join("file-300.txt"),
            "unreviewed checkout content\n",
        )
        .unwrap();
        git(dir.path(), &["add", "file-300.txt"]);
        git(dir.path(), &["commit", "-m", "unrelated local head"]);
        std::fs::write(dir.path().join("file-0.txt"), "dirty worktree content\n").unwrap();
        let view = GhPullRequest {
            base_sha: base,
            head_sha: head,
            additions: 301,
            ..view(302)
        };
        let diff = diff_with(6002, Some("example/repo"), &view, &mut |program, args| {
            if program == Program::Gh {
                assert_eq!(
                    args[1], "view",
                    "large PR must not call the 300-file endpoint"
                );
                Ok(metadata(&view))
            } else {
                run_command(dir.path(), program, args)
            }
        })
        .unwrap();
        assert_eq!(
            diff.lines()
                .filter(|line| line.starts_with("diff --git "))
                .count(),
            302
        );
        assert!(diff.contains("+file 300\n"));
        assert!(diff.contains("GIT binary patch\nliteral 4\n"));
        assert!(!diff.contains("unreviewed checkout content"));
        assert!(!diff.contains("dirty worktree content"));
    }

    #[test]
    fn limit_error_missing_files_and_missing_binary_patch_use_the_same_fallback() {
        let view = view(2);
        let complete = patch("a.txt") + &patch("b.txt");
        for remote in [
            None,
            Some(patch("a.txt")),
            Some(
                patch("a.txt")
                    + "diff --git a/b.txt b/b.txt\nBinary files a/b.txt and b/b.txt differ\n",
            ),
        ] {
            let mut used_local = false;
            let result = diff_with(6002, None, &view, &mut |program, args| match (
                program,
                args[0].as_str(),
            ) {
                (Program::Gh, _) if args[1] == "diff" => remote
                    .clone()
                    .ok_or_else(|| anyhow::anyhow!("HTTP 406: diff exceeds 300 files")),
                (Program::Gh, _) => Ok(metadata(&view)),
                (Program::Git, "rev-parse") => Ok("false\n".into()),
                (Program::Git, "merge-base") => {
                    assert_eq!(&args[2..], &[view.base_sha.clone(), view.head_sha.clone()]);
                    Ok(view.base_sha.clone())
                }
                (Program::Git, "diff") => {
                    used_local = true;
                    assert!(args.contains(&"--no-ext-diff".into()));
                    assert!(args.contains(&"--no-textconv".into()));
                    assert!(args.contains(&"--binary".into()));
                    assert_eq!(
                        &args[args.len() - 3..],
                        &[view.base_sha.clone(), view.head_sha.clone(), "--".into()]
                    );
                    Ok(complete.clone())
                }
                _ => panic!("unexpected command"),
            })
            .unwrap();
            assert!(used_local);
            assert_eq!(result, complete);
        }
    }

    #[test]
    fn missing_shallow_or_ambiguous_history_never_returns_a_partial_diff() {
        let view = view(301);
        for failure in ["missing", "shallow", "multiple"] {
            let error = diff_with(6002, None, &view, &mut |program, args| {
                assert_eq!(program, Program::Git);
                match args[0].as_str() {
                    "rev-parse" => Ok(if failure == "shallow" {
                        "true"
                    } else {
                        "false"
                    }
                    .into()),
                    "merge-base" if failure == "multiple" => {
                        Ok(format!("{}\n{}\n", "c".repeat(40), "d".repeat(40)))
                    }
                    "merge-base" => bail!("missing pinned commit object"),
                    _ => panic!("must not generate a diff without exact history"),
                }
            })
            .unwrap_err();
            let error = format!("{error:#}");
            assert!(error.contains("Cannot obtain the complete PR diff"));
            assert!(error.contains("fetch-depth: 0"));
            assert!(error.contains(&view.head_sha));
        }
    }

    #[test]
    fn head_base_and_file_count_changes_after_collection_invalidate_the_review() {
        let view = view(1);
        for field in ["head", "base", "files"] {
            let mut current = view.clone();
            match field {
                "head" => current.head_sha = "c".repeat(40),
                "base" => current.base_sha = "d".repeat(40),
                _ => current.changed_files = 2,
            }
            let error = diff_with(6002, None, &view, &mut |program, args| {
                assert_eq!(program, Program::Gh);
                Ok(if args[1] == "diff" {
                    patch("a.txt")
                } else {
                    metadata(&current)
                })
            })
            .unwrap_err();
            assert!(
                error
                    .to_string()
                    .contains("Pull request changed during review")
            );
            assert!(
                same_revision(&view, &current).is_err(),
                "publication uses the same revision check"
            );
        }
    }

    #[test]
    fn review_budget_preserves_unicode_and_rejects_the_whole_oversized_input() {
        let diff = patch("unicode.txt") + "+\u{1f433}\n";
        ensure_input_fits(&diff, diff.chars().count()).unwrap();
        let error = ensure_input_fits(&diff, diff.chars().count() - 1).unwrap_err();
        assert!(error.to_string().contains("No review was run or posted"));
        assert!(diff.ends_with("+\u{1f433}\n"));
    }

    #[test]
    fn malformed_commit_metadata_cannot_become_git_arguments() {
        let mut invalid = view(1);
        invalid.head_sha = "--output=/tmp/unsafe".into();
        assert!(view_with(6002, None, &mut |_, _| Ok(metadata(&invalid))).is_err());
    }

    #[test]
    fn missing_or_truncated_text_patches_cannot_pass_with_a_matching_file_count() {
        let view = view(1);
        for diff in [
            "diff --git a/a b/a\nindex 123..456 100644\n",
            "diff --git a/a b/a\nnew file mode 100644\n",
            "diff --git a/a b/a\n--- a/a\n+++ b/a\n@@ -0,0 +1,2 @@\n+first\n",
            "diff --git a/a b/a\n--- a/a\n+++ b/a\n@@ malformed @@\n+first\n",
        ] {
            assert!(complete_file_set(diff, &view).is_err(), "{diff}");
        }
        complete_file_set(&patch("a"), &view).unwrap();
    }

    #[test]
    fn oversized_command_output_is_refused_without_unbounded_capture() {
        let dir = repository();
        std::fs::write(
            dir.path().join("large.txt"),
            vec![b'x'; MAX_OUTPUT_BYTES + 1],
        )
        .unwrap();
        git(dir.path(), &["add", "large.txt"]);
        git(dir.path(), &["commit", "-m", "large fixture"]);
        let error = run_command(
            dir.path(),
            Program::Git,
            &["show".into(), "HEAD:large.txt".into()],
        )
        .unwrap_err();
        assert!(error.to_string().contains("no partial output was accepted"));
    }
}
