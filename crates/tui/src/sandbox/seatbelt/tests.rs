use super::*;

/// #4085: entering Codewhale's Seatbelt profile must retain filesystem
/// access already granted by macOS. A read-write extension remains readable
/// under ReadOnly, but only a write-capable policy may honor its write half.
#[test]
fn file_provider_extensions_preserve_read_without_bypassing_read_only() {
    let cwd = Path::new("/tmp/test");
    let workspace_write = generate_policy(&SandboxPolicy::default(), cwd);
    let read_only = generate_policy(&SandboxPolicy::ReadOnly, cwd);

    for policy in [&workspace_write, &read_only] {
        assert!(policy.contains(r#"(allow file-read* (extension "com.apple.app-sandbox.read"))"#));
        assert!(
            policy.contains(r#"(allow file-read* (extension "com.apple.app-sandbox.read-write"))"#)
        );
    }
    assert!(
        workspace_write
            .contains(r#"(allow file-write* (extension "com.apple.app-sandbox.read-write"))"#)
    );
    assert!(
        !read_only
            .contains(r#"(allow file-write* (extension "com.apple.app-sandbox.read-write"))"#)
    );
}

/// Hermetic command/policy-shape coverage for the six operations reported in
/// #4085. Each operation gets a fresh fixture so an early failure cannot hide
/// later results. This is not physical File Provider acceptance.
#[test]
fn file_provider_synthetic_operations_are_independent_when_seatbelt_available() {
    if !is_available() {
        eprintln!("SKIP: sandbox-exec unavailable; no synthetic operation evidence collected");
        return;
    }

    #[derive(Clone, Copy, Debug)]
    enum Operation {
        Mkdir,
        Write,
        Read,
        Grep,
        DeleteFile,
        DeleteDirectory,
    }

    let mut failures = Vec::new();
    for operation in [
        Operation::Mkdir,
        Operation::Write,
        Operation::Read,
        Operation::Grep,
        Operation::DeleteFile,
        Operation::DeleteDirectory,
    ] {
        let fixture = tempfile::tempdir().expect("create independent operation fixture");
        let workspace = fixture
            .path()
            .join("Library/CloudStorage/TestProvider/Workspace");
        std::fs::create_dir_all(&workspace).expect("create synthetic CloudStorage workspace");
        let target = workspace.join("target");
        let source = workspace.join("source");

        let command = match operation {
            Operation::Mkdir => vec![
                "/bin/mkdir".to_string(),
                target.to_string_lossy().into_owned(),
            ],
            Operation::Write => {
                std::fs::write(&source, b"file-provider\n").expect("seed copy source");
                vec![
                    "/bin/cp".to_string(),
                    source.to_string_lossy().into_owned(),
                    target.to_string_lossy().into_owned(),
                ]
            }
            Operation::Read => {
                std::fs::write(&target, b"file-provider\n").expect("seed read target");
                vec![
                    "/bin/cat".to_string(),
                    target.to_string_lossy().into_owned(),
                ]
            }
            Operation::Grep => {
                std::fs::write(&target, b"file-provider\n").expect("seed grep target");
                vec![
                    "/usr/bin/grep".to_string(),
                    "-q".to_string(),
                    "file-provider".to_string(),
                    target.to_string_lossy().into_owned(),
                ]
            }
            Operation::DeleteFile => {
                std::fs::write(&target, b"file-provider\n").expect("seed deletion target");
                vec!["/bin/rm".to_string(), target.to_string_lossy().into_owned()]
            }
            Operation::DeleteDirectory => {
                std::fs::create_dir(&target).expect("seed directory deletion target");
                vec![
                    "/bin/rmdir".to_string(),
                    target.to_string_lossy().into_owned(),
                ]
            }
        };

        let args = create_seatbelt_args(command, &SandboxPolicy::default(), &workspace);
        let output = Command::new(SANDBOX_EXEC_PATH)
            .args(args)
            .current_dir(&workspace)
            .output()
            .expect("execute sandboxed operation");

        let effect_matches = match operation {
            Operation::Mkdir => target.is_dir(),
            Operation::Write => {
                matches!(std::fs::read(&target), Ok(bytes) if bytes == b"file-provider\n")
            }
            Operation::Read => output.stdout == b"file-provider\n",
            Operation::Grep => true,
            Operation::DeleteFile | Operation::DeleteDirectory => !target.exists(),
        };
        if !output.status.success() || !effect_matches {
            failures.push(format!(
                "{operation:?}: status={:?}, stderr={}",
                output.status.code(),
                String::from_utf8_lossy(&output.stderr)
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "independent sandbox operations failed:\n{}",
        failures.join("\n")
    );
}
