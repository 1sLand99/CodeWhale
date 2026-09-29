//! Bounded package content digest shared by audit and mutation.
//!
//! Kept separate so `install` can write metadata v2 without depending on the
//! audit module (which itself depends on install marker constants).

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use super::install::{DEFAULT_MAX_SIZE_BYTES, is_reserved_root_metadata};

pub const PACKAGE_DIGEST_MAX_BYTES: u64 = DEFAULT_MAX_SIZE_BYTES;
pub const PACKAGE_DIGEST_MAX_FILES: usize = 256;
pub const PACKAGE_DIGEST_MAX_DEPTH: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackageDigestError {
    Unreadable,
    SymlinkPresent,
    EscapedRoot,
    Cycle,
    Oversized,
    TooManyFiles,
    TooDeep,
}

impl std::fmt::Display for PackageDigestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Unreadable => "unreadable package file",
            Self::SymlinkPresent => "symlink present in package",
            Self::EscapedRoot => "path escaped package root",
            Self::Cycle => "symlink/directory cycle",
            Self::Oversized => "package exceeded size limit",
            Self::TooManyFiles => "package exceeded file limit",
            Self::TooDeep => "package exceeded depth limit",
        })
    }
}

impl std::error::Error for PackageDigestError {}

/// SHA-256 hex of the normalized package manifest (relative path + len + bytes).
pub fn compute_package_digest(package_dir: &Path) -> Result<String, PackageDigestError> {
    let canonical_package =
        fs::canonicalize(package_dir).map_err(|_| PackageDigestError::Unreadable)?;

    let mut files: Vec<(String, Vec<u8>)> = Vec::new();
    let mut total_bytes: u64 = 0;
    let mut visited = HashSet::new();

    walk(
        package_dir,
        &canonical_package,
        0,
        &mut visited,
        &mut files,
        &mut total_bytes,
    )?;

    files.sort_by(|a, b| a.0.cmp(&b.0));
    let mut hasher = Sha256::new();
    for (rel, bytes) in &files {
        hasher.update(rel.as_bytes());
        hasher.update(b"\0");
        hasher.update((bytes.len() as u64).to_le_bytes());
        hasher.update(bytes);
    }
    Ok(hex_digest(hasher.finalize()))
}

fn walk(
    dir: &Path,
    package_root: &Path,
    depth: usize,
    visited: &mut HashSet<PathBuf>,
    files: &mut Vec<(String, Vec<u8>)>,
    total_bytes: &mut u64,
) -> Result<(), PackageDigestError> {
    if depth > PACKAGE_DIGEST_MAX_DEPTH {
        return Err(PackageDigestError::TooDeep);
    }
    let meta = fs::symlink_metadata(dir).map_err(|_| PackageDigestError::Unreadable)?;
    if meta.file_type().is_symlink() {
        return Err(PackageDigestError::SymlinkPresent);
    }
    let canonical = fs::canonicalize(dir).map_err(|_| PackageDigestError::Unreadable)?;
    if !canonical.starts_with(package_root) {
        return Err(PackageDigestError::EscapedRoot);
    }
    if !visited.insert(canonical) {
        return Err(PackageDigestError::Cycle);
    }

    let entries = fs::read_dir(dir).map_err(|_| PackageDigestError::Unreadable)?;
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };

        let meta = fs::symlink_metadata(&path).map_err(|_| PackageDigestError::Unreadable)?;
        if meta.file_type().is_symlink() {
            return Err(PackageDigestError::SymlinkPresent);
        }

        // Only local bookkeeping at the package root is outside the receipt.
        // Hidden files and backup/temp names can contain executable payloads.
        if depth == 0 && is_reserved_root_metadata(Path::new(name)) {
            continue;
        }

        if meta.is_dir() {
            walk(&path, package_root, depth + 1, visited, files, total_bytes)?;
            continue;
        }
        if !meta.is_file() {
            continue;
        }
        if files.len() >= PACKAGE_DIGEST_MAX_FILES {
            return Err(PackageDigestError::TooManyFiles);
        }
        let len = meta.len();
        if *total_bytes + len > PACKAGE_DIGEST_MAX_BYTES {
            return Err(PackageDigestError::Oversized);
        }
        let bytes = fs::read(&path).map_err(|_| PackageDigestError::Unreadable)?;
        *total_bytes += bytes.len() as u64;
        let rel = path
            .strip_prefix(package_root)
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_else(|_| name.to_string());
        files.push((rel, bytes));
    }
    Ok(())
}

fn hex_digest(bytes: impl AsRef<[u8]>) -> String {
    let bytes = bytes.as_ref();
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(&mut out, "{byte:02x}");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skills::audit::{self, SkillAuditMode, TrustState};
    use crate::skills::install::{INSTALLED_FROM_MARKER, TRUSTED_MARKER, write_trust_v2};

    #[test]
    fn hidden_and_backup_payload_changes_stale_the_trust_receipt() {
        let tmp = tempfile::tempdir().unwrap();
        let package = tmp.path().join(".codewhale/skills/demo");
        fs::create_dir_all(&package).unwrap();
        fs::write(
            package.join("SKILL.md"),
            "---\nname: demo\ndescription: test\n---\nbody",
        )
        .unwrap();
        let payloads = [
            ".hidden",
            ".hidden-dir/run.sh",
            "script.bak",
            "script.tmp",
            "nested/.trusted",
        ];
        for relative in payloads {
            let path = package.join(relative);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, "safe").unwrap();
        }
        let digest = compute_package_digest(&package).unwrap();
        write_trust_v2(&package, &digest).unwrap();
        let initial = audit::scan(tmp.path(), None, SkillAuditMode::OwnedOnly, None);
        assert_eq!(initial.skills.len(), 1);
        assert_eq!(
            initial.skills[0].trust,
            TrustState::TrustedForDigest(digest.clone())
        );
        for relative in payloads {
            fs::write(package.join(relative), "evil").unwrap();
            assert_ne!(
                compute_package_digest(&package).unwrap(),
                digest,
                "{relative}"
            );
            let changed = audit::scan(tmp.path(), None, SkillAuditMode::OwnedOnly, None);
            assert_eq!(
                changed.skills[0].trust,
                TrustState::TrustStale,
                "{relative}"
            );
            fs::write(package.join(relative), "safe").unwrap();
        }
        // These root files are local bookkeeping, not executable payload.
        for marker in [
            INSTALLED_FROM_MARKER,
            TRUSTED_MARKER,
            ".system-installed-version",
        ] {
            fs::write(package.join(marker), "local metadata").unwrap();
            assert_eq!(compute_package_digest(&package).unwrap(), digest);
        }
    }
}
