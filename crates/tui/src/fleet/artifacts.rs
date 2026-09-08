//! Fleet artifact I/O. Both verifier publication and HTTP evidence reads use
//! the same workspace-relative, opened-directory authority. This replaces the
//! ordinary path-based writes in task_spec and reads in runtime_api.

use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Component, Path};

use anyhow::{Context, Result, ensure};
use codewhale_protocol::fleet::FleetArtifactRef;
use sha2::{Digest, Sha256};

// The HTTP preview stays small; verifying a larger artifact streams its digest
// without retaining all bytes. The writer shares the ceiling so it cannot
// publish an artifact the evidence reader is unable to verify.
const MAX_ARTIFACT_BYTES: u64 = 16 * 1024 * 1024;

pub(crate) fn path_is_confined(path: &Path) -> bool {
    !path.as_os_str().is_empty()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

pub(crate) fn write(workspace: &Path, relative: &Path, bytes: &[u8]) -> Result<()> {
    ensure!(
        bytes.len() as u64 <= MAX_ARTIFACT_BYTES,
        "Fleet artifact exceeds the 16 MiB limit"
    );
    let parent = ArtifactParent::open(workspace, relative, true)?;
    match parent.publish(bytes) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            let existing = parent.open_file()?;
            let mut saved = Vec::new();
            existing
                .take(bytes.len() as u64 + 1)
                .read_to_end(&mut saved)?;
            ensure!(
                saved == bytes,
                "An existing Fleet artifact contains different bytes"
            );
            Ok(())
        }
        Err(error) => Err(error).context("Publishing Fleet artifact"),
    }
}

pub(crate) fn read_verified(
    workspace: &Path,
    artifact: &FleetArtifactRef,
    preview_limit: u64,
) -> Result<(Vec<u8>, u64)> {
    let parent = ArtifactParent::open(workspace, &artifact.path, false)?;
    let file = parent.open_file()?;
    let size = file.metadata()?.len();
    ensure!(
        size <= MAX_ARTIFACT_BYTES,
        "Fleet artifact exceeds the 16 MiB verification limit"
    );
    ensure!(
        artifact.size_bytes.is_none_or(|expected| expected == size),
        "Fleet artifact size changed"
    );
    let checksum = artifact
        .checksum
        .as_deref()
        .context("Fleet artifact has no recorded checksum")?;
    let mut hasher = Sha256::new();
    let mut preview = Vec::new();
    let mut buffer = [0_u8; 8192];
    let mut total = 0_u64;
    // The digest and returned preview consume exactly the same bytes from the
    // same opened file. A changed/replaced pathname is never reopened for data.
    let mut reader = (&file).take(size + 1);
    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        total += count as u64;
        ensure!(total <= size, "Fleet artifact grew while being read");
        hasher.update(&buffer[..count]);
        let remaining = preview_limit.saturating_sub(preview.len() as u64) as usize;
        preview.extend_from_slice(&buffer[..count.min(remaining)]);
    }
    ensure!(
        total == size && file.metadata()?.len() == size,
        "Fleet artifact size changed while being read"
    );
    ensure!(
        format!("sha256:{}", crate::hashing::hex_bytes(hasher.finalize())) == checksum,
        "Fleet artifact checksum does not match the recorded receipt"
    );
    Ok((preview, size))
}

fn invalid_path() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        "Fleet artifact path must stay within the workspace",
    )
}

#[cfg(unix)]
struct ArtifactParent {
    directory: File,
    filename: std::ffi::CString,
}

#[cfg(unix)]
impl ArtifactParent {
    fn open(workspace: &Path, relative: &Path, create: bool) -> io::Result<Self> {
        use std::os::fd::{AsRawFd, FromRawFd};
        use std::os::unix::ffi::OsStrExt;
        if !path_is_confined(relative) {
            return Err(invalid_path());
        }
        let workspace = workspace.canonicalize()?;
        // Use the established credential/artifact openat pattern, without
        // touching credentials or creating a second filesystem store.
        // SAFETY: static path and immediate ownership of a successful fd.
        let fd = unsafe {
            libc::open(
                c"/".as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            )
        };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: this fd was just created and has no other owner.
        let mut directory = unsafe { File::from_raw_fd(fd) };
        let parents = relative.parent().ok_or_else(invalid_path)?;
        for (path, may_create) in [(workspace.as_path(), false), (parents, create)] {
            for component in path.components() {
                let Component::Normal(name) = component else {
                    if component == Component::RootDir {
                        continue;
                    }
                    return Err(invalid_path());
                };
                let name = std::ffi::CString::new(name.as_bytes())?;
                let flags = libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC;
                // SAFETY: directory pins the parent; name is one component.
                let mut fd = unsafe { libc::openat(directory.as_raw_fd(), name.as_ptr(), flags) };
                if fd < 0
                    && may_create
                    && io::Error::last_os_error().kind() == io::ErrorKind::NotFound
                {
                    // SAFETY: directory and relative basename remain valid.
                    if unsafe { libc::mkdirat(directory.as_raw_fd(), name.as_ptr(), 0o700) } != 0
                        && io::Error::last_os_error().kind() != io::ErrorKind::AlreadyExists
                    {
                        return Err(io::Error::last_os_error());
                    }
                    // SAFETY: reject a symlink inserted after mkdirat.
                    fd = unsafe { libc::openat(directory.as_raw_fd(), name.as_ptr(), flags) };
                }
                if fd < 0 {
                    return Err(io::Error::last_os_error());
                }
                // SAFETY: fd is freshly owned.
                directory = unsafe { File::from_raw_fd(fd) };
            }
        }
        Ok(Self {
            directory,
            filename: std::ffi::CString::new(
                relative.file_name().ok_or_else(invalid_path)?.as_bytes(),
            )?,
        })
    }

    fn open_file(&self) -> io::Result<File> {
        use std::os::fd::{AsRawFd, FromRawFd};
        use std::os::unix::fs::MetadataExt;
        // SAFETY: a pinned parent and validated basename; never follows links.
        let fd = unsafe {
            libc::openat(
                self.directory.as_raw_fd(),
                self.filename.as_ptr(),
                libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
            )
        };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: fd is freshly owned.
        let file = unsafe { File::from_raw_fd(fd) };
        let metadata = file.metadata()?;
        if !metadata.is_file() || metadata.nlink() != 1 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Fleet artifact must be a regular, non-hard-linked file",
            ));
        }
        Ok(file)
    }

    fn publish(&self, bytes: &[u8]) -> io::Result<()> {
        use std::os::fd::{AsRawFd, FromRawFd};
        let temporary =
            std::ffi::CString::new(format!(".fleet-artifact-{}.tmp", uuid::Uuid::new_v4()))
                .expect("generated basename");
        // SAFETY: parent is pinned; exclusive creation cannot follow a link.
        let fd = unsafe {
            libc::openat(
                self.directory.as_raw_fd(),
                temporary.as_ptr(),
                libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                0o600,
            )
        };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: fd is freshly owned.
        let mut file = unsafe { File::from_raw_fd(fd) };
        let result = (|| {
            file.write_all(bytes)?;
            file.sync_all()?;
            // SAFETY: both basenames are anchored to the same open parent.
            // linkat publishes all bytes atomically without clobbering a winner.
            if unsafe {
                libc::linkat(
                    self.directory.as_raw_fd(),
                    temporary.as_ptr(),
                    self.directory.as_raw_fd(),
                    self.filename.as_ptr(),
                    0,
                )
            } != 0
            {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        })();
        // SAFETY: unlink only this call's exclusive temporary basename.
        let cleanup = unsafe { libc::unlinkat(self.directory.as_raw_fd(), temporary.as_ptr(), 0) };
        if cleanup != 0 {
            return Err(io::Error::last_os_error());
        }
        result?;
        self.directory.sync_all()
    }
}

#[cfg(windows)]
struct ArtifactParent {
    // Retaining every ancestor without delete/write sharing prevents a path
    // swap or junction replacement while path-based Windows calls are running.
    _ancestors: Vec<File>,
    directory: std::path::PathBuf,
    filename: std::ffi::OsString,
}

#[cfg(windows)]
impl ArtifactParent {
    fn open(workspace: &Path, relative: &Path, create: bool) -> io::Result<Self> {
        use std::os::windows::fs::OpenOptionsExt;
        if !path_is_confined(relative) {
            return Err(invalid_path());
        }
        let workspace = workspace.canonicalize()?;
        let mut ancestors = Vec::new();
        let mut directory = std::path::PathBuf::new();
        for (path, may_create) in [
            (workspace.as_path(), false),
            (relative.parent().ok_or_else(invalid_path)?, create),
        ] {
            for component in path.components() {
                directory.push(component.as_os_str());
                if matches!(component, Component::Prefix(_)) {
                    continue;
                }
                let open = || {
                    std::fs::OpenOptions::new()
                        .read(true)
                        .share_mode(0x0000_0001)
                        .custom_flags(0x0220_0000)
                        .open(&directory)
                }; // BACKUP_SEMANTICS | OPEN_REPARSE_POINT
                let file = match open() {
                    Err(error) if may_create && error.kind() == io::ErrorKind::NotFound => {
                        match std::fs::create_dir(&directory) {
                            Ok(()) => {}
                            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
                            Err(error) => return Err(error),
                        }
                        open()?
                    }
                    result => result?,
                };
                let metadata = file.metadata()?;
                if !metadata.is_dir() || crate::plugins::metadata_is_link_or_reparse(&metadata) {
                    return Err(invalid_path());
                }
                ancestors.push(file);
            }
        }
        Ok(Self {
            _ancestors: ancestors,
            directory,
            filename: relative.file_name().ok_or_else(invalid_path)?.to_owned(),
        })
    }

    fn open_file(&self) -> io::Result<File> {
        // Existing protected reader rejects reparse points, hard links and
        // non-regular files, and denies concurrent writes/replacement.
        crate::plugins::manifest::open_bundle_file(&self.directory.join(&self.filename))
    }

    fn publish(&self, bytes: &[u8]) -> io::Result<()> {
        let mut temporary = tempfile::NamedTempFile::new_in(&self.directory)?;
        temporary.write_all(bytes)?;
        temporary.as_file().sync_all()?;
        temporary
            .persist_noclobber(self.directory.join(&self.filename))
            .map_err(|error| error.error)?;
        Ok(())
    }
}

#[cfg(all(not(unix), not(windows)))]
struct ArtifactParent;
#[cfg(all(not(unix), not(windows)))]
impl ArtifactParent {
    fn open(_: &Path, _: &Path, _: bool) -> io::Result<Self> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Confined Fleet artifact I/O is unavailable on this platform",
        ))
    }
    fn open_file(&self) -> io::Result<File> {
        unreachable!()
    }
    fn publish(&self, _: &[u8]) -> io::Result<()> {
        unreachable!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use codewhale_protocol::fleet::FleetArtifactKind;

    fn reference(path: &str, bytes: &[u8]) -> FleetArtifactRef {
        FleetArtifactRef {
            kind: FleetArtifactKind::Receipt,
            path: path.into(),
            checksum: Some(format!("sha256:{}", crate::hashing::sha256_hex(bytes))),
            mime_type: None,
            size_bytes: Some(bytes.len() as u64),
        }
    }

    #[test]
    fn publication_is_immutable_and_verifies_beyond_the_preview() {
        let workspace = tempfile::tempdir().unwrap();
        let bytes = vec![b'a'; 128 * 1024];
        let artifact = reference(".codewhale/fleet/receipt.json", &bytes);
        write(workspace.path(), &artifact.path, &bytes).unwrap();
        write(workspace.path(), &artifact.path, &bytes).unwrap();
        assert!(write(workspace.path(), &artifact.path, b"replacement").is_err());
        let (preview, size) = read_verified(workspace.path(), &artifact, 65_536).unwrap();
        assert_eq!(preview, bytes[..65_536]);
        assert_eq!(size, bytes.len() as u64);

        // Changing bytes outside the returned preview must still fail the
        // complete digest check, even when size/metadata remain unchanged.
        let mut changed = bytes;
        *changed.last_mut().unwrap() = b'b';
        std::fs::write(workspace.path().join(&artifact.path), changed).unwrap();
        let error = read_verified(workspace.path(), &artifact, 65_536).unwrap_err();
        assert!(error.to_string().contains("checksum"));
    }

    #[test]
    fn missing_digest_size_mismatch_and_oversized_files_fail_closed() {
        let workspace = tempfile::tempdir().unwrap();
        let mut artifact = reference("receipt.json", b"receipt");
        write(workspace.path(), &artifact.path, b"receipt").unwrap();
        artifact.checksum = None;
        assert!(read_verified(workspace.path(), &artifact, 64).is_err());
        artifact = reference("receipt.json", b"receipt");
        artifact.size_bytes = Some(999);
        assert!(read_verified(workspace.path(), &artifact, 64).is_err());
        File::options()
            .write(true)
            .open(workspace.path().join(&artifact.path))
            .unwrap()
            .set_len(MAX_ARTIFACT_BYTES + 1)
            .unwrap();
        assert!(
            read_verified(workspace.path(), &artifact, 64)
                .unwrap_err()
                .to_string()
                .contains("limit")
        );
        assert!(
            write(
                workspace.path(),
                Path::new("huge.json"),
                &vec![0; MAX_ARTIFACT_BYTES as usize + 1]
            )
            .is_err()
        );
        assert!(!workspace.path().join("huge.json").exists());
    }

    #[cfg(unix)]
    #[test]
    fn parent_and_final_symlinks_and_hard_links_never_escape() {
        use std::os::unix::fs::symlink;
        let workspace = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let secret = b"OUTSIDE_SYNTHETIC_RECEIPT_CANARY";
        let outside_file = outside.path().join("private.txt");
        std::fs::write(&outside_file, secret).unwrap();
        std::fs::create_dir_all(workspace.path().join(".codewhale/fleet")).unwrap();

        let final_link = reference(".codewhale/fleet/final.json", secret);
        symlink(&outside_file, workspace.path().join(&final_link.path)).unwrap();
        assert!(read_verified(workspace.path(), &final_link, 64).is_err());
        assert!(write(workspace.path(), &final_link.path, b"overwrite").is_err());

        symlink(
            outside.path(),
            workspace.path().join(".codewhale/fleet/parent"),
        )
        .unwrap();
        let parent_link = reference(".codewhale/fleet/parent/private.txt", secret);
        assert!(read_verified(workspace.path(), &parent_link, 64).is_err());
        assert!(write(workspace.path(), &parent_link.path, b"overwrite").is_err());
        assert!(
            write(
                workspace.path(),
                Path::new(".codewhale/fleet/parent/new.json"),
                b"new"
            )
            .is_err()
        );
        assert!(!outside.path().join("new.json").exists());

        let hard_link = reference(".codewhale/fleet/hard.json", secret);
        std::fs::hard_link(&outside_file, workspace.path().join(&hard_link.path)).unwrap();
        assert!(read_verified(workspace.path(), &hard_link, 64).is_err());
        assert!(write(workspace.path(), &hard_link.path, b"overwrite").is_err());
        assert_eq!(std::fs::read(outside_file).unwrap(), secret);
    }

    #[cfg(unix)]
    #[test]
    fn publication_uses_the_open_parent_after_a_path_swap() {
        use std::os::unix::fs::symlink;
        let workspace = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let parent =
            ArtifactParent::open(workspace.path(), Path::new("receipts/item.json"), true).unwrap();
        std::fs::rename(
            workspace.path().join("receipts"),
            workspace.path().join("pinned"),
        )
        .unwrap();
        symlink(outside.path(), workspace.path().join("receipts")).unwrap();
        parent.publish(b"complete receipt").unwrap();
        assert_eq!(
            std::fs::read(workspace.path().join("pinned/item.json")).unwrap(),
            b"complete receipt"
        );
        assert!(!outside.path().join("item.json").exists());
        assert!(write(workspace.path(), Path::new("receipts/next.json"), b"next").is_err());
    }
}
