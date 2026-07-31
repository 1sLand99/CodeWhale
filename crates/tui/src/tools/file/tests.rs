use super::*;
use std::time::Duration;

#[tokio::test]
async fn missing_pdf_path_precedes_unavailable_helper() {
    let temporary = tempfile::tempdir().expect("tempdir");
    let input = temporary.path().join("missing.pdf");
    let missing = temporary.path().join("definitely-not-pdftotext");
    let error = read_pdf_if_detected(
        &input,
        None,
        super::super::pdf::PdfTextCommand::test(missing.as_os_str(), Duration::from_secs(1), None),
    )
    .await
    .expect_err("missing path must fail before the missing helper is launched");

    match error {
        ToolError::ExecutionFailed { message } => {
            assert!(message.contains("Failed to read"), "{message}");
            assert!(message.contains("missing.pdf"), "{message}");
        }
        other => panic!("expected ordinary read failure, got {other:?}"),
    }
}

#[tokio::test]
async fn read_file_missing_pdftotext_is_a_failed_typed_outcome() {
    let temporary = tempfile::tempdir().expect("tempdir");
    let missing = temporary.path().join("definitely-not-pdftotext");
    let input = temporary.path().join("input.pdf");
    std::fs::write(&input, b"%PDF-1.7\n%%EOF").expect("fixture");

    let error = read_pdf_with_command(
        &input,
        None,
        super::super::pdf::PdfTextCommand::test(missing.as_os_str(), Duration::from_secs(1), None),
    )
    .await
    .expect_err("missing helper must fail the tool call");
    let payload = match &error {
        ToolError::NotAvailable { message } => {
            serde_json::from_str::<Value>(message).expect("structured unavailable payload")
        }
        other => panic!("unexpected error: {other:?}"),
    };
    assert_eq!(payload["type"], "binary_unavailable");
    assert_eq!(
        crate::tools::spec::ToolExecutionOutcome::from_legacy(Err(error)).status,
        crate::tools::spec::ToolTerminalStatus::Failed
    );
}
