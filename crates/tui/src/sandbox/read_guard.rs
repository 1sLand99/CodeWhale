//! Read deny-list (S1, #5568 follow-up).
//!
//! # What this is
//!
//! Every sandbox posture Codewhale ships — including `read-only` — grants the
//! sandboxed process read access to the entire filesystem
//! (`policy.rs::has_full_disk_read_access`). #5568 added the *plumbing* for an
//! opt-in deny-list (Seatbelt last-match-wins `deny file-read*` rules;
//! bubblewrap masks), but the list shipped empty, so in practice nothing was
//! denied. This module supplies (a) a curated default set covering the obvious
//! credential stores and (b) the in-process matcher that Codewhale's own
//! file-reading tools consult — those tools call `std::fs` directly inside the
//! harness process and are never wrapped by `sandbox-exec` or `bwrap` at all,
//! so the OS-level rules alone left the largest hole wide open.
//!
//! # What this is NOT
//!
//! **This is defense-in-depth, not a security boundary.** It raises the cost of
//! a confused or prompt-injected agent stumbling into `~/.ssh/id_ed25519`; it
//! does not contain a deliberate attacker. Specifically it does NOT stop:
//!
//! - **Hardlinks.** A hardlink is a second *name* for the same inode with no
//!   trace of the first. `foo` hardlinked to `~/.ssh/id_rsa` canonicalizes to
//!   `foo`, matches nothing, and is read. No path-based deny-list can fix this;
//!   only an inode-level or MAC-label check could.
//! - **Content already elsewhere.** A key copied into the workspace before the
//!   agent ran, or pasted into the conversation, is readable.
//! - **Indirect reads.** `ssh-agent`, `security find-generic-password`,
//!   `aws sts get-session-token`, a helper the user installed — a process that
//!   *hands over* a secret without the agent reading the file. On macOS
//!   `~/Library/Keychains` is denied but the keychain *daemon* is not.
//! - **`danger-full-access`.** That posture bypasses the OS wrapper entirely
//!   (`should_sandbox() == false`). The in-process tool checks still apply, but
//!   a shell command does not.
//! - **Anything on the network side.** A denied read does not stop exfiltration
//!   of what *was* read.
//! - **Reads by MCP servers and other child processes** that Codewhale did not
//!   itself wrap.
//!
//! Treat it as a seatbelt, and keep the real controls (least-privilege
//! credentials, short-lived tokens, approval prompts) doing the real work.
//!
//! # Matching rules
//!
//! - **Deny wins.** A path matching any deny rule is refused; there is no allow
//!   rule that can override one. Exemptions (`sandbox_read_denylist_exempt`)
//!   subtract from the *built-in defaults* only, before matching — a path the
//!   user explicitly listed in `sandbox_denied_read_paths` can never be
//!   exempted back open.
//! - **Symlinks.** Both the literal path and its `canonicalize()`d target are
//!   tested, so a symlink pointing into `~/.ssh` is denied by its target even
//!   though its own name is innocuous.
//! - **`..` and relative paths.** Paths are lexically normalized (`.`/`..`
//!   folded without touching the disk) and, when the file does not exist, the
//!   deepest existing ancestor is canonicalized and the remainder re-appended.
//! - **Case.** On macOS and Windows — where the default filesystem is
//!   case-insensitive — comparison is case-folded, so `~/.SSH/ID_RSA` is denied.
//!   On Linux comparison is exact, matching the filesystem's own semantics.
//! - **Boundaries are component-wise.** `~/.awsome/notes.md` is not under
//!   `~/.aws`; a plain string `starts_with` would have said it was.

use std::ffi::OsString;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, OnceLock, RwLock};

/// Why a read was refused, so callers can render one clear message.
///
/// A denial is always an explicit error. It is never rendered as an empty file,
/// a zero-length result, or a "not found" — a silent empty read teaches an agent
/// that the file is empty and invites it to try a dozen sibling paths.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadDenial {
    /// The path the caller asked for, as written.
    pub requested: PathBuf,
    /// The deny rule that matched.
    pub rule: DenyRule,
    /// True when the match was on the symlink target rather than the literal
    /// path — worth saying out loud, or the refusal looks arbitrary.
    pub via_symlink: bool,
}

impl ReadDenial {
    /// One-line, non-leaky refusal message.
    ///
    /// Names the *rule*, not the resolved secret path: telling the model that
    /// `notes.txt` really points at `/Users/x/.ssh/id_ed25519` hands it the
    /// location it was looking for.
    #[must_use]
    pub fn message(&self, tool: &str) -> String {
        let via = if self.via_symlink {
            " (reached through a symlink)"
        } else {
            ""
        };
        format!(
            "{tool} refused to read {}{via}: the sandbox read deny-list blocks {}. \
             This path is treated as a credential store. If it is genuinely needed, \
             add it to `sandbox_read_denylist_exempt` in your Codewhale config.",
            self.requested.display(),
            self.rule.describe(),
        )
    }
}

/// A single deny rule. Kept as an enum rather than a bare path so the refusal
/// message can name the rule ("SSH keys") instead of echoing a secret path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DenyRule {
    /// Everything at or below a directory (or a single file at that path).
    Subtree {
        /// Normalized absolute path.
        path: PathBuf,
        /// Human label, e.g. "SSH keys (~/.ssh)".
        label: &'static str,
    },
    /// Any file whose *name* matches, anywhere on disk. Used for `.env`, which
    /// has no fixed location.
    FileName {
        /// Human label.
        label: &'static str,
    },
}

impl DenyRule {
    #[must_use]
    fn describe(&self) -> String {
        match self {
            DenyRule::Subtree { label, .. } => (*label).to_string(),
            DenyRule::FileName { label } => (*label).to_string(),
        }
    }
}

/// The compiled deny-list.
#[derive(Debug, Clone, Default)]
pub struct ReadDenylist {
    subtrees: Vec<DenyRule>,
    deny_env_files: bool,
}

impl ReadDenylist {
    /// An empty deny-list: denies nothing. Used when the user turns defaults
    /// off and configures no paths of their own.
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    /// Build the effective deny-list.
    ///
    /// * `include_defaults` — apply the built-in credential-store set.
    /// * `extra` — user-configured `sandbox_denied_read_paths`; these are
    ///   absolute (or `~`-prefixed) paths and are never exemptable.
    /// * `exempt` — user-configured `sandbox_read_denylist_exempt`; subtracts
    ///   from the built-in defaults only.
    #[must_use]
    pub fn build(include_defaults: bool, extra: &[PathBuf], exempt: &[PathBuf]) -> Self {
        let exempt_normalized: Vec<PathBuf> = exempt
            .iter()
            .cloned()
            .map(expand_home_prefix)
            .map(|p| normalize_lexically(&p))
            .collect();

        let mut subtrees = Vec::new();
        let mut deny_env_files = false;

        if include_defaults {
            deny_env_files = !exempt_normalized.iter().any(|p| p.as_os_str() == ".env");
            for (raw, label) in default_denied_subtrees() {
                let path = normalize_lexically(&raw);
                if exempt_normalized.iter().any(|e| path_is_within(&path, e)) {
                    continue;
                }
                subtrees.push(DenyRule::Subtree { path, label });
            }
        }

        // User-listed denies are appended last and are NOT filtered by the
        // exempt list: deny wins over allow, without exception.
        for raw in extra {
            let path = normalize_lexically(&expand_home_prefix(raw.clone()));
            if path.as_os_str().is_empty() {
                continue;
            }
            subtrees.push(DenyRule::Subtree {
                path,
                label: "a path in `sandbox_denied_read_paths`",
            });
        }

        Self {
            subtrees,
            deny_env_files,
        }
    }

    /// True when nothing is denied — i.e. the posture really does grant read of
    /// every file on disk.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.subtrees.is_empty() && !self.deny_env_files
    }

    /// Every literal subtree path, for handing to the OS wrappers
    /// (`SandboxManager::set_denied_read_subpaths`). The filename rule (`.env`)
    /// has no fixed path and therefore cannot be expressed to Seatbelt or
    /// bubblewrap as a subpath — it is enforced in-process only, which is a
    /// real gap for shell commands and is documented as such.
    #[must_use]
    pub fn subtree_paths(&self) -> Vec<PathBuf> {
        self.subtrees
            .iter()
            .filter_map(|rule| match rule {
                DenyRule::Subtree { path, .. } => Some(path.clone()),
                DenyRule::FileName { .. } => None,
            })
            .collect()
    }

    /// Check a path a tool is about to read.
    ///
    /// `requested` may be relative, may contain `..`, may be a symlink, and may
    /// not exist. Both the lexically normalized path and the canonicalized
    /// target are tested; either matching is a denial.
    pub fn check(&self, requested: &Path) -> Result<(), ReadDenial> {
        if self.is_empty() {
            return Ok(());
        }

        let literal = absolutize(requested);
        let resolved = canonicalize_best_effort(requested);
        let via_symlink = resolved != literal;

        for candidate in [&literal, &resolved] {
            if self.deny_env_files && is_env_file(candidate) {
                return Err(ReadDenial {
                    requested: requested.to_path_buf(),
                    rule: DenyRule::FileName {
                        label: "environment files (`.env`, `.env.<name>`)",
                    },
                    via_symlink: via_symlink && candidate == &resolved,
                });
            }
            for rule in &self.subtrees {
                let DenyRule::Subtree { path, .. } = rule else {
                    continue;
                };
                if path_is_within(candidate, path) {
                    return Err(ReadDenial {
                        requested: requested.to_path_buf(),
                        rule: rule.clone(),
                        via_symlink: via_symlink && candidate == &resolved,
                    });
                }
            }
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Process-wide active deny-list
//
// Codewhale's file-reading tools (`read_file`, `read`, `read_media`, …) run
// in-process and are never wrapped by `sandbox-exec` or `bwrap`, so they have
// to consult the deny-list themselves. Threading config through every tool
// signature would touch dozens of call sites for one read; a process-global
// set once at startup keeps the blast radius to the tools that actually read
// files.
// ---------------------------------------------------------------------------

static ACTIVE: RwLock<Option<Arc<ReadDenylist>>> = RwLock::new(None);
static FALLBACK: OnceLock<Arc<ReadDenylist>> = OnceLock::new();

/// Install the deny-list resolved from user config. Called once during startup.
pub fn set_active(list: ReadDenylist) {
    if let Ok(mut slot) = ACTIVE.write() {
        *slot = Some(Arc::new(list));
    }
}

/// The deny-list in force for this process.
///
/// Falls back to the built-in defaults when startup has not installed one, so
/// a code path that runs before config load is protected rather than open.
#[must_use]
pub fn active() -> Arc<ReadDenylist> {
    if let Ok(slot) = ACTIVE.read()
        && let Some(list) = slot.as_ref()
    {
        return Arc::clone(list);
    }
    Arc::clone(FALLBACK.get_or_init(|| Arc::new(ReadDenylist::build(true, &[], &[]))))
}

/// `.env`, `.env.local`, `.env.production` — but deliberately NOT
/// `.env.example`, `.env.sample`, `.env.template`, `.env.defaults`, or
/// `.env.dist`. Those are committed placeholders that a coding agent has a
/// legitimate, routine reason to read (they document which variables a project
/// needs), and denying them would break ordinary development for no security
/// gain — they contain no secrets by construction.
fn is_env_file(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    let name = fold_case_str(name);
    if name == ".env" {
        return true;
    }
    let Some(suffix) = name.strip_prefix(".env.") else {
        return false;
    };
    const PLACEHOLDER_SUFFIXES: &[&str] = &[
        "example", "sample", "template", "defaults", "dist", "schema",
    ];
    !PLACEHOLDER_SUFFIXES.contains(&suffix)
}

/// The built-in default deny set.
///
/// Chosen against one test: **would denying this break ordinary development?**
/// Everything here is a credential store that build tools, language servers,
/// test runners, and source reading never need. Deliberately excluded, despite
/// containing or neighbouring secrets:
///
/// - `~/.gitconfig` — read constantly by tooling; holds config, not secrets.
///   (`~/.git-credentials`, which holds the secrets, IS denied.)
/// - `~/.cargo`, `~/.npm`, `~/.m2` as a whole — the seatbelt profile already
///   grants these read+write because `cargo build` and `npx` fail without
///   them. Only the credential *files* inside them are denied.
/// - `~/.docker` as a whole — `docker build` reads it. Only
///   `~/.docker/config.json` (registry auth) is denied.
/// - `~/.config` as a whole — far too broad; individual credential dirs inside
///   it are listed instead.
/// - The user's source tree, `~/Documents`, `~/Downloads` — a coding agent must
///   still be able to read the user's code, which is the entire point.
fn default_denied_subtrees() -> Vec<(PathBuf, &'static str)> {
    let Some(home) = dirs::home_dir() else {
        // No home directory: only the machine-wide entries are meaningful.
        return machine_wide_denied_subtrees();
    };
    let h = |rel: &str| home.join(rel);

    let mut out = vec![
        // --- SSH / GPG ---
        (h(".ssh"), "SSH keys and known-hosts (~/.ssh)"),
        (h(".gnupg"), "GnuPG keyring (~/.gnupg)"),
        // --- Cloud provider credentials ---
        (h(".aws"), "AWS credentials (~/.aws)"),
        (
            h(".config/gcloud"),
            "Google Cloud credentials (~/.config/gcloud)",
        ),
        (h(".azure"), "Azure credentials (~/.azure)"),
        (h(".kube"), "Kubernetes credentials (~/.kube)"),
        (h(".oci"), "Oracle Cloud credentials (~/.oci)"),
        (
            h(".config/doctl"),
            "DigitalOcean credentials (~/.config/doctl)",
        ),
        (h(".config/fly"), "Fly.io credentials (~/.config/fly)"),
        (h(".vercel"), "Vercel credentials (~/.vercel)"),
        (
            h(".wrangler/config"),
            "Cloudflare credentials (~/.wrangler/config)",
        ),
        (
            h(".config/gh/hosts.yml"),
            "GitHub CLI tokens (~/.config/gh/hosts.yml)",
        ),
        (
            h(".config/glab-cli"),
            "GitLab CLI tokens (~/.config/glab-cli)",
        ),
        // --- Package-registry and network credential files ---
        (h(".netrc"), "netrc credentials (~/.netrc)"),
        (h("_netrc"), "netrc credentials (~/_netrc)"),
        (h(".pgpass"), "PostgreSQL password file (~/.pgpass)"),
        (h(".my.cnf"), "MySQL credentials (~/.my.cnf)"),
        (h(".npmrc"), "npm auth tokens (~/.npmrc)"),
        (h(".pypirc"), "PyPI auth tokens (~/.pypirc)"),
        (
            h(".git-credentials"),
            "stored git credentials (~/.git-credentials)",
        ),
        (
            h(".cargo/credentials"),
            "crates.io token (~/.cargo/credentials)",
        ),
        (
            h(".cargo/credentials.toml"),
            "crates.io token (~/.cargo/credentials.toml)",
        ),
        (
            h(".docker/config.json"),
            "Docker registry auth (~/.docker/config.json)",
        ),
        (
            h(".m2/settings-security.xml"),
            "Maven master password (~/.m2)",
        ),
        (
            h(".gradle/gradle.properties"),
            "Gradle credentials (~/.gradle/gradle.properties)",
        ),
        // --- Codewhale's own credential stores ---
        // Duplicated with `tools::file::is_codewhale_credential_path` on
        // purpose: that guard is scoped to the *active* config, this one is
        // unconditional, and neither should depend on the other still existing.
        (
            h(".codewhale/secrets"),
            "Codewhale secret store (~/.codewhale/secrets)",
        ),
        (
            h(".deepseek/secrets"),
            "Codewhale secret store (~/.deepseek/secrets)",
        ),
        // --- Browser profiles (cookies, saved passwords, session tokens) ---
        (h(".mozilla"), "Firefox profile (~/.mozilla)"),
        (
            h(".config/google-chrome"),
            "Chrome profile (~/.config/google-chrome)",
        ),
        (
            h(".config/chromium"),
            "Chromium profile (~/.config/chromium)",
        ),
        (
            h(".config/BraveSoftware"),
            "Brave profile (~/.config/BraveSoftware)",
        ),
    ];

    if cfg!(target_os = "macos") {
        out.extend([
            (
                h("Library/Keychains"),
                "macOS keychain (~/Library/Keychains)",
            ),
            (
                h("Library/Application Support/Google/Chrome"),
                "Chrome profile (~/Library/Application Support/Google/Chrome)",
            ),
            (
                h("Library/Application Support/Firefox"),
                "Firefox profile (~/Library/Application Support/Firefox)",
            ),
            (
                h("Library/Application Support/BraveSoftware"),
                "Brave profile (~/Library/Application Support/BraveSoftware)",
            ),
            (h("Library/Safari"), "Safari profile (~/Library/Safari)"),
            (
                h("Library/Cookies"),
                "macOS cookie store (~/Library/Cookies)",
            ),
        ]);
    }

    out.extend(machine_wide_denied_subtrees());
    out
}

fn machine_wide_denied_subtrees() -> Vec<(PathBuf, &'static str)> {
    let mut out: Vec<(PathBuf, &'static str)> = vec![
        (
            PathBuf::from("/etc/shadow"),
            "system password hashes (/etc/shadow)",
        ),
        (
            PathBuf::from("/etc/sudoers"),
            "sudoers policy (/etc/sudoers)",
        ),
        (PathBuf::from("/etc/ssh"), "system SSH host keys (/etc/ssh)"),
    ];
    if cfg!(target_os = "macos") {
        out.push((
            PathBuf::from("/Library/Keychains"),
            "system keychain (/Library/Keychains)",
        ));
    }
    out
}

// ---------------------------------------------------------------------------
// Path handling
//
// The evasion cases this has to survive are the whole reason the module exists;
// a deny-list a symlink walks around is theater.
// ---------------------------------------------------------------------------

/// Expand a leading `~` to the user's home directory.
fn expand_home_prefix(path: PathBuf) -> PathBuf {
    let Some(text) = path.to_str() else {
        return path;
    };
    if text == "~" {
        return dirs::home_dir().unwrap_or(path);
    }
    if let Some(rest) = text.strip_prefix("~/")
        && let Some(home) = dirs::home_dir()
    {
        return home.join(rest);
    }
    path
}

/// Fold `.` and `..` without touching the disk, and make the path absolute
/// against the current directory when it is relative.
///
/// Purely lexical on purpose: this is the check that catches
/// `workspace/../../../.ssh/id_rsa` even when nothing on that path exists yet.
/// It is paired with — never a substitute for — `canonicalize_best_effort`,
/// which is what catches symlinks.
fn normalize_lexically(path: &Path) -> PathBuf {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("/"))
            .join(path)
    };

    let mut out = PathBuf::new();
    for component in absolute.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                // Never pop past the root: `/..` is `/`.
                if out
                    .components()
                    .next_back()
                    .is_some_and(|c| !matches!(c, Component::RootDir | Component::Prefix(_)))
                {
                    out.pop();
                }
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

fn absolutize(path: &Path) -> PathBuf {
    normalize_lexically(path)
}

/// Resolve symlinks as far as the filesystem allows.
///
/// `canonicalize` fails on a path that does not exist, which is exactly the
/// case for a read of a file that is about to be created — and also the case an
/// evader would reach for. So on failure we walk up to the deepest ancestor
/// that *does* exist, canonicalize that (resolving any symlink in the
/// directory chain), and re-append the remainder.
fn canonicalize_best_effort(path: &Path) -> PathBuf {
    let absolute = normalize_lexically(path);
    if let Ok(resolved) = std::fs::canonicalize(&absolute) {
        return resolved;
    }

    let mut suffix: Vec<OsString> = Vec::new();
    let mut cursor = absolute.as_path();
    loop {
        let Some(parent) = cursor.parent() else {
            return absolute;
        };
        if let Some(name) = cursor.file_name() {
            suffix.push(name.to_os_string());
        }
        if let Ok(resolved) = std::fs::canonicalize(parent) {
            let mut out = resolved;
            for name in suffix.iter().rev() {
                out.push(name);
            }
            return out;
        }
        cursor = parent;
    }
}

/// Case-fold when — and only when — the platform's default filesystem is
/// case-insensitive. Folding on Linux would deny `~/.SSH` on a system where
/// that is a genuinely different directory.
fn fold_case_str(text: &str) -> String {
    if cfg!(any(target_os = "macos", target_os = "windows")) {
        text.to_lowercase()
    } else {
        text.to_string()
    }
}

fn fold_component(component: &std::ffi::OsStr) -> OsString {
    match component.to_str() {
        Some(text) => OsString::from(fold_case_str(text)),
        None => component.to_os_string(),
    }
}

/// True when `candidate` is `root` itself or lives beneath it.
///
/// Compared component by component, not by string prefix: `~/.awsome` must not
/// match the `~/.aws` rule, and `starts_with` on the raw strings says it does.
/// (`Path::starts_with` is already component-wise; the case folding is what
/// forces the manual walk.)
fn path_is_within(candidate: &Path, root: &Path) -> bool {
    let mut root_components = root.components().map(|c| fold_component(c.as_os_str()));
    let mut candidate_components = candidate
        .components()
        .map(|c| fold_component(c.as_os_str()));

    loop {
        match (root_components.next(), candidate_components.next()) {
            (None, _) => return true,
            (Some(_), None) => return false,
            (Some(r), Some(c)) if r == c => {}
            (Some(_), Some(_)) => return false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn denylist_for(paths: &[PathBuf]) -> ReadDenylist {
        ReadDenylist::build(false, paths, &[])
    }

    #[test]
    fn empty_denylist_denies_nothing_and_reports_full_disk_read() {
        let list = ReadDenylist::empty();
        assert!(list.is_empty());
        assert!(list.check(Path::new("/etc/hosts")).is_ok());
    }

    #[test]
    fn direct_path_under_a_denied_root_is_refused() {
        let secret = tempfile::tempdir().expect("tempdir");
        let file = secret.path().join("id_ed25519");
        std::fs::write(&file, "KEY").expect("write");

        let list = denylist_for(&[secret.path().to_path_buf()]);
        let denial = list.check(&file).expect_err("must deny");
        assert_eq!(denial.requested, file);
        assert!(!denial.via_symlink);
    }

    #[test]
    fn sibling_with_a_shared_string_prefix_is_not_denied() {
        // `~/.awsome` must not be caught by the `~/.aws` rule. This is the bug
        // a naive string `starts_with` ships with.
        let tmp = tempfile::tempdir().expect("tempdir");
        let denied = tmp.path().join("aws");
        let innocent = tmp.path().join("awsome");
        std::fs::create_dir_all(&denied).expect("mkdir");
        std::fs::create_dir_all(&innocent).expect("mkdir");
        let note = innocent.join("notes.md");
        std::fs::write(&note, "notes").expect("write");

        let list = denylist_for(std::slice::from_ref(&denied));
        assert!(
            list.check(&note).is_ok(),
            "sibling prefix must stay readable"
        );
    }

    #[test]
    fn dot_dot_traversal_out_of_the_workspace_is_refused() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let secret_dir = tmp.path().join("secrets");
        let workspace = tmp.path().join("workspace");
        std::fs::create_dir_all(&secret_dir).expect("mkdir");
        std::fs::create_dir_all(&workspace).expect("mkdir");
        let secret = secret_dir.join("token");
        std::fs::write(&secret, "TOKEN").expect("write");

        let list = denylist_for(std::slice::from_ref(&secret_dir));
        let sneaky = workspace.join("..").join("secrets").join("token");
        list.check(&sneaky)
            .expect_err("`..` must not walk around the deny-list");
    }

    #[test]
    fn symlink_pointing_into_a_denied_tree_is_refused_by_its_target() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let secret_dir = tmp.path().join("secrets");
        let workspace = tmp.path().join("workspace");
        std::fs::create_dir_all(&secret_dir).expect("mkdir");
        std::fs::create_dir_all(&workspace).expect("mkdir");
        let secret = secret_dir.join("id_rsa");
        std::fs::write(&secret, "KEY").expect("write");

        let link = workspace.join("harmless.txt");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&secret, &link).expect("symlink");
        #[cfg(not(unix))]
        return;

        let list = denylist_for(std::slice::from_ref(&secret_dir));
        let denial = list.check(&link).expect_err("symlink must not walk around");
        assert!(denial.via_symlink, "denial should report the symlink hop");
        assert!(denial.message("read_file").contains("symlink"));
    }

    #[test]
    fn symlinked_parent_directory_is_refused() {
        // The link is on a *directory* in the middle of the path, not the leaf.
        let tmp = tempfile::tempdir().expect("tempdir");
        let secret_dir = tmp.path().join("secrets");
        let workspace = tmp.path().join("workspace");
        std::fs::create_dir_all(&secret_dir).expect("mkdir");
        std::fs::create_dir_all(&workspace).expect("mkdir");
        std::fs::write(secret_dir.join("token"), "TOKEN").expect("write");

        let link_dir = workspace.join("data");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&secret_dir, &link_dir).expect("symlink");
        #[cfg(not(unix))]
        return;

        let list = denylist_for(std::slice::from_ref(&secret_dir));
        list.check(&link_dir.join("token"))
            .expect_err("symlinked parent must not walk around");
    }

    #[cfg(any(target_os = "macos", target_os = "windows"))]
    #[test]
    fn case_variation_is_refused_on_case_insensitive_filesystems() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let secret_dir = tmp.path().join("Secrets");
        std::fs::create_dir_all(&secret_dir).expect("mkdir");
        std::fs::write(secret_dir.join("id_rsa"), "KEY").expect("write");

        let list = denylist_for(std::slice::from_ref(&secret_dir));
        let shouted = tmp.path().join("SECRETS").join("ID_RSA");
        list.check(&shouted)
            .expect_err("case variation must not walk around on a case-insensitive FS");
    }

    #[test]
    fn nonexistent_path_under_a_denied_root_is_still_refused() {
        // canonicalize() fails here; the deepest-existing-ancestor walk is what
        // has to carry the check.
        let tmp = tempfile::tempdir().expect("tempdir");
        let secret_dir = tmp.path().join("secrets");
        std::fs::create_dir_all(&secret_dir).expect("mkdir");

        let list = denylist_for(std::slice::from_ref(&secret_dir));
        list.check(&secret_dir.join("not-created-yet").join("key"))
            .expect_err("a not-yet-existing path under a denied root must still be denied");
    }

    #[test]
    fn env_files_are_denied_but_committed_placeholders_are_not() {
        let list = ReadDenylist::build(true, &[], &[]);
        let tmp = tempfile::tempdir().expect("tempdir");

        for denied in [".env", ".env.local", ".env.production"] {
            let path = tmp.path().join(denied);
            list.check(&path)
                .unwrap_err_or_panic(&format!("{denied} should be denied"));
        }
        for allowed in [".env.example", ".env.sample", ".env.template", ".env.dist"] {
            let path = tmp.path().join(allowed);
            assert!(
                list.check(&path).is_ok(),
                "{allowed} is a committed placeholder and must stay readable"
            );
        }
    }

    #[test]
    fn ordinary_source_files_stay_readable_under_the_defaults() {
        let list = ReadDenylist::build(true, &[], &[]);
        let tmp = tempfile::tempdir().expect("tempdir");
        for ordinary in [
            "main.rs",
            "Cargo.toml",
            "README.md",
            ".gitignore",
            ".env.example",
        ] {
            let path = tmp.path().join(ordinary);
            assert!(
                list.check(&path).is_ok(),
                "{ordinary} must stay readable — a coding agent has to read the source tree"
            );
        }
    }

    #[test]
    fn defaults_cover_ssh_and_cloud_credential_stores() {
        let Some(home) = dirs::home_dir() else {
            return;
        };
        let list = ReadDenylist::build(true, &[], &[]);
        for rel in [
            ".ssh/id_ed25519",
            ".aws/credentials",
            ".config/gcloud/x",
            ".netrc",
        ] {
            list.check(&home.join(rel))
                .unwrap_err_or_panic(&format!("~/{rel} should be denied by default"));
        }
    }

    #[test]
    fn exempt_narrows_the_defaults_but_never_an_explicit_deny() {
        let Some(home) = dirs::home_dir() else {
            return;
        };
        let ssh = home.join(".ssh");

        // Exempting the default rule reopens it.
        let exempted = ReadDenylist::build(true, &[], std::slice::from_ref(&ssh));
        assert!(
            exempted.check(&ssh.join("id_rsa")).is_ok(),
            "an exempted default must be readable again"
        );

        // The same exemption must NOT reopen a path the user explicitly denied.
        let both =
            ReadDenylist::build(true, std::slice::from_ref(&ssh), std::slice::from_ref(&ssh));
        both.check(&ssh.join("id_rsa"))
            .unwrap_err_or_panic("deny must win over allow");
    }

    #[test]
    fn defaults_can_be_turned_off_entirely() {
        let Some(home) = dirs::home_dir() else {
            return;
        };
        let list = ReadDenylist::build(false, &[], &[]);
        assert!(list.is_empty());
        assert!(list.check(&home.join(".ssh/id_rsa")).is_ok());
    }

    #[test]
    fn subtree_paths_feed_the_os_wrappers_and_omit_the_filename_rule() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let list = ReadDenylist::build(false, &[tmp.path().to_path_buf()], &[]);
        let paths = list.subtree_paths();
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0], normalize_lexically(tmp.path()));
    }

    #[test]
    fn denial_message_names_the_rule_without_echoing_the_resolved_secret_path() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let secret_dir = tmp.path().join("secrets");
        let workspace = tmp.path().join("workspace");
        std::fs::create_dir_all(&secret_dir).expect("mkdir");
        std::fs::create_dir_all(&workspace).expect("mkdir");
        std::fs::write(secret_dir.join("id_rsa"), "KEY").expect("write");

        let link = workspace.join("notes.txt");
        #[cfg(unix)]
        std::os::unix::fs::symlink(secret_dir.join("id_rsa"), &link).expect("symlink");
        #[cfg(not(unix))]
        return;

        let list = denylist_for(std::slice::from_ref(&secret_dir));
        let message = list.check(&link).expect_err("deny").message("read_file");
        assert!(message.contains("notes.txt"), "{message}");
        assert!(
            !message.contains("id_rsa"),
            "the refusal must not hand back the secret's real location: {message}"
        );
        assert!(
            message.contains("sandbox_read_denylist_exempt"),
            "{message}"
        );
    }

    #[test]
    fn root_parent_traversal_does_not_escape_above_root() {
        assert_eq!(
            normalize_lexically(Path::new("/../../etc")),
            PathBuf::from("/etc")
        );
    }

    // Small helper so the intent of a "must be denied" assertion reads clearly.
    trait ExpectDenied {
        fn unwrap_err_or_panic(self, message: &str);
    }
    impl ExpectDenied for Result<(), ReadDenial> {
        fn unwrap_err_or_panic(self, message: &str) {
            assert!(self.is_err(), "{message}");
        }
    }
}
