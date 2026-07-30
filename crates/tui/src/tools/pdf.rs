//! Shared PDF-to-text adapter.
//!
//! PDF parsing is intentionally delegated to the optional `pdftotext`
//! executable. Keeping the adapter here gives file and web tools one error
//! contract without carrying a second parser and font stack in Codewhale.

use std::ffi::OsStr;
use std::fmt;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum PdfTextError {
    BinaryUnavailable,
    Execution(String),
}

impl fmt::Display for PdfTextError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BinaryUnavailable => formatter.write_str(
                "PDF text extraction requires the optional `pdftotext` executable (Poppler)",
            ),
            Self::Execution(message) => formatter.write_str(message),
        }
    }
}

pub(super) fn extract_path(
    path: &Path,
    page_range: Option<(u32, u32)>,
) -> Result<String, PdfTextError> {
    extract_path_with_binary(OsStr::new("pdftotext"), path, page_range)
}

pub(super) fn extract_bytes(bytes: &[u8]) -> Result<String, PdfTextError> {
    extract_bytes_with_binary(OsStr::new("pdftotext"), bytes)
}

fn extract_bytes_with_binary(binary: &OsStr, bytes: &[u8]) -> Result<String, PdfTextError> {
    let mut input = tempfile::NamedTempFile::new().map_err(|error| {
        PdfTextError::Execution(format!("failed to stage fetched PDF: {error}"))
    })?;
    input.write_all(bytes).map_err(|error| {
        PdfTextError::Execution(format!("failed to stage fetched PDF: {error}"))
    })?;
    input.flush().map_err(|error| {
        PdfTextError::Execution(format!("failed to stage fetched PDF: {error}"))
    })?;
    extract_path_with_binary(binary, input.path(), None)
}

fn extract_path_with_binary(
    binary: &OsStr,
    path: &Path,
    page_range: Option<(u32, u32)>,
) -> Result<String, PdfTextError> {
    let mut command = Command::new(binary);
    command.arg("-layout");
    if let Some((start, end)) = page_range {
        command.arg("-f").arg(start.to_string());
        command.arg("-l").arg(end.to_string());
    }
    command
        .arg(path)
        .arg("-")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let output = command.output().map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            PdfTextError::BinaryUnavailable
        } else {
            PdfTextError::Execution(format!("failed to launch pdftotext: {error}"))
        }
    })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(PdfTextError::Execution(format!(
            "pdftotext failed (exit {:?}): {stderr}",
            output.status.code()
        )));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_binary_is_typed_without_inspecting_error_text() {
        let temporary = tempfile::tempdir().expect("tempdir");
        let missing = temporary.path().join("definitely-not-pdftotext");
        let input = temporary.path().join("input.pdf");
        std::fs::write(&input, b"%PDF-1.7\n%%EOF").expect("fixture");
        assert_eq!(
            extract_path_with_binary(missing.as_os_str(), &input, None),
            Err(PdfTextError::BinaryUnavailable)
        );
    }

    #[cfg(unix)]
    #[test]
    fn shared_adapter_forwards_page_window_and_returns_stdout() {
        use std::os::unix::fs::PermissionsExt;

        let temporary = tempfile::tempdir().expect("tempdir");
        let binary = temporary.path().join("fake-pdftotext");
        std::fs::write(
            &binary,
            "#!/bin/sh\nprintf 'args:%s\\n' \"$*\"\nprintf 'page one\\fpage two\\n'\n",
        )
        .expect("fake binary");
        let mut permissions = std::fs::metadata(&binary).expect("metadata").permissions();
        permissions.set_mode(0o700);
        std::fs::set_permissions(&binary, permissions).expect("executable");

        let input = temporary.path().join("input.pdf");
        std::fs::write(&input, b"fixture bytes").expect("fixture");
        let text = extract_path_with_binary(binary.as_os_str(), &input, Some((2, 4)))
            .expect("fake extraction");
        assert!(text.contains("-layout -f 2 -l 4"), "{text}");
        assert!(text.contains(input.to_string_lossy().as_ref()), "{text}");
        assert!(text.ends_with("page one\u{c}page two\n"), "{text:?}");

        let staged = extract_bytes_with_binary(binary.as_os_str(), b"fetched fixture bytes")
            .expect("fake fetched extraction");
        assert!(staged.contains("-layout"), "{staged}");
        assert!(staged.ends_with("page one\u{c}page two\n"), "{staged:?}");
    }
}
