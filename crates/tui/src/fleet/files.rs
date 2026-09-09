//! Workspace-confined file operations shared by Fleet artifacts and its ledger.

use std::fs::File;
use std::io::{self, Write};
use std::path::{Component, Path};

pub(crate) fn path_is_confined(path: &Path) -> bool {
    !path.as_os_str().is_empty()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

fn invalid_path() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        "Fleet artifact path must stay within the workspace",
    )
}

#[cfg(unix)]
#[derive(Debug)]
pub(super) struct WorkspaceFile {
    directory: File,
    filename: std::ffi::CString,
}

#[cfg(unix)]
impl WorkspaceFile {
    pub(super) fn open(workspace: &Path, relative: &Path, create: bool) -> io::Result<Self> {
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

    pub(super) fn sibling(&self, name: &str) -> io::Result<Self> {
        if !path_is_confined(Path::new(name)) || Path::new(name).components().count() != 1 {
            return Err(invalid_path());
        }
        Ok(Self {
            directory: self.directory.try_clone()?,
            filename: std::ffi::CString::new(name)?,
        })
    }

    pub(super) fn open_update(&self, create: bool, append: bool) -> io::Result<File> {
        self.open_with_flags(
            libc::O_RDWR
                | if create { libc::O_CREAT } else { 0 }
                | if append { libc::O_APPEND } else { 0 },
        )
    }

    pub(super) fn open_file(&self) -> io::Result<File> {
        self.open_with_flags(libc::O_RDONLY)
    }

    fn open_with_flags(&self, flags: libc::c_int) -> io::Result<File> {
        use std::os::fd::{AsRawFd, FromRawFd};
        use std::os::unix::fs::MetadataExt;
        // SAFETY: a pinned parent and validated basename; never follows links.
        let fd = unsafe {
            libc::openat(
                self.directory.as_raw_fd(),
                self.filename.as_ptr(),
                flags | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
                0o600,
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
                "Fleet file must be a regular, non-hard-linked file",
            ));
        }
        Ok(file)
    }

    pub(super) fn publish(&self, bytes: &[u8]) -> io::Result<()> {
        self.atomic_write(bytes, false)
    }

    pub(super) fn replace(&self, bytes: &[u8]) -> io::Result<()> {
        self.atomic_write(bytes, true)
    }

    fn atomic_write(&self, bytes: &[u8], replace: bool) -> io::Result<()> {
        use std::os::fd::{AsRawFd, FromRawFd};
        let temporary =
            std::ffi::CString::new(format!(".fleet-write-{}.tmp", uuid::Uuid::new_v4()))
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
            // Replacement changes the directory entry, never a symlink target.
            let published = unsafe {
                if replace {
                    libc::renameat(
                        self.directory.as_raw_fd(),
                        temporary.as_ptr(),
                        self.directory.as_raw_fd(),
                        self.filename.as_ptr(),
                    )
                } else {
                    libc::linkat(
                        self.directory.as_raw_fd(),
                        temporary.as_ptr(),
                        self.directory.as_raw_fd(),
                        self.filename.as_ptr(),
                        0,
                    )
                }
            };
            if published != 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        })();
        // Successful rename already consumed this temporary entry. Never
        // unlink the vacant old name, which another writer could now reuse.
        if !replace || result.is_err() {
            // SAFETY: unlink this call's exclusive temporary basename.
            if unsafe { libc::unlinkat(self.directory.as_raw_fd(), temporary.as_ptr(), 0) } != 0 {
                return Err(io::Error::last_os_error());
            }
        }
        result?;
        self.directory.sync_all()
    }
}

#[cfg(windows)]
#[derive(Debug)]
pub(super) struct WorkspaceFile {
    // Retaining every ancestor without delete/write sharing prevents a path
    // swap or junction replacement while path-based Windows calls are running.
    _ancestors: Vec<File>,
    directory: std::path::PathBuf,
    filename: std::ffi::OsString,
}

#[cfg(windows)]
impl WorkspaceFile {
    pub(super) fn open(workspace: &Path, relative: &Path, create: bool) -> io::Result<Self> {
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

    pub(super) fn sibling(&self, name: &str) -> io::Result<Self> {
        if !path_is_confined(Path::new(name)) || Path::new(name).components().count() != 1 {
            return Err(invalid_path());
        }
        Ok(Self {
            _ancestors: self
                ._ancestors
                .iter()
                .map(File::try_clone)
                .collect::<io::Result<_>>()?,
            directory: self.directory.clone(),
            filename: name.into(),
        })
    }

    pub(super) fn open_update(&self, create: bool, append: bool) -> io::Result<File> {
        use std::os::windows::fs::OpenOptionsExt;
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .append(append)
            .create(create)
            .truncate(false)
            .share_mode(0x0000_0007)
            .custom_flags(0x0020_0000)
            .open(self.directory.join(&self.filename))?;
        let metadata = file.metadata()?;
        if !metadata.is_file()
            || crate::plugins::metadata_is_link_or_reparse(&metadata)
            || crate::plugins::windows_file_identity(&file)?.links != 1
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Fleet file must be regular and not linked",
            ));
        }
        Ok(file)
    }

    pub(super) fn open_file(&self) -> io::Result<File> {
        // Existing protected reader rejects reparse points, hard links and
        // non-regular files, and denies concurrent writes/replacement.
        crate::plugins::manifest::open_bundle_file(&self.directory.join(&self.filename))
    }

    pub(super) fn publish(&self, bytes: &[u8]) -> io::Result<()> {
        self.atomic_write(bytes, false)
    }

    pub(super) fn replace(&self, bytes: &[u8]) -> io::Result<()> {
        self.atomic_write(bytes, true)
    }

    fn atomic_write(&self, bytes: &[u8], replace: bool) -> io::Result<()> {
        let mut temporary = tempfile::NamedTempFile::new_in(&self.directory)?;
        temporary.write_all(bytes)?;
        temporary.as_file().sync_all()?;
        let path = self.directory.join(&self.filename);
        if replace {
            temporary.persist(path)
        } else {
            temporary.persist_noclobber(path)
        }
        .map_err(|error| error.error)?;
        Ok(())
    }
}

#[cfg(all(not(unix), not(windows)))]
#[derive(Debug)]
pub(super) struct WorkspaceFile;
#[cfg(all(not(unix), not(windows)))]
impl WorkspaceFile {
    pub(super) fn open(_: &Path, _: &Path, _: bool) -> io::Result<Self> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Confined Fleet artifact I/O is unavailable on this platform",
        ))
    }
    pub(super) fn sibling(&self, _: &str) -> io::Result<Self> {
        unreachable!()
    }
    pub(super) fn open_update(&self, _: bool, _: bool) -> io::Result<File> {
        unreachable!()
    }
    pub(super) fn replace(&self, _: &[u8]) -> io::Result<()> {
        unreachable!()
    }
    pub(super) fn open_file(&self) -> io::Result<File> {
        unreachable!()
    }
    pub(super) fn publish(&self, _: &[u8]) -> io::Result<()> {
        unreachable!()
    }
}

/// Compare already opened lock handles; a replaced lock must never create two
/// independent critical sections for the same live ledger.
pub(super) fn same_file(left: &File, right: &File) -> io::Result<bool> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let a = left.metadata()?;
        let b = right.metadata()?;
        Ok(a.dev() == b.dev() && a.ino() == b.ino())
    }
    #[cfg(windows)]
    {
        let a = crate::plugins::windows_file_identity(left)?;
        let b = crate::plugins::windows_file_identity(right)?;
        Ok(a.volume == b.volume && a.index == b.index)
    }
    #[cfg(all(not(unix), not(windows)))]
    {
        let _ = (left, right);
        unreachable!()
    }
}
