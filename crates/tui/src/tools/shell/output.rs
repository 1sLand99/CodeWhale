use std::sync::{Arc, Mutex};

pub(super) fn take_delta_from_buffer(
    buffer: &Arc<Mutex<Vec<u8>>>,
    cursor: &mut usize,
) -> (Vec<u8>, usize) {
    let guard = buffer.lock().unwrap_or_else(|e| e.into_inner());
    let total = guard.len();
    let start = (*cursor).min(total);
    // Clone only the unread portion (the delta), not the entire accumulated buffer.
    // Long-running processes can produce megabytes of output; cloning the full
    // buffer on every poll held the ShellManager mutex for O(total_bytes) time.
    let unread = &guard[start..];
    // A poll can land mid-character: the caller decodes this delta as UTF-8, so
    // handing back a truncated multibyte sequence renders it as replacement
    // glyphs and corrupts the next delta's leading byte too (the streaming-client
    // bug from #1675, in the shell preview path). Leave an incomplete trailing
    // sequence in the buffer for the next poll. Bytes that are genuinely invalid
    // rather than merely unfinished still pass through, so binary output cannot
    // stall the cursor, and the final result is read from the whole buffer.
    let consumed = match std::str::from_utf8(unread) {
        Ok(_) => unread.len(),
        Err(error) if error.error_len().is_none() => error.valid_up_to(),
        Err(_) => unread.len(),
    };
    let delta = unread[..consumed].to_vec();
    *cursor = start + consumed;
    (delta, total)
}

/// Read only the tail of a byte buffer and return (total_len, tail_string).
///
/// Avoids cloning the full buffer when only a trailing excerpt is needed
/// (e.g. for the job-panel display). `max_tail_chars` is in Unicode scalar
/// values; we read at most `max_tail_chars * 4` bytes from the end to account
/// for multi-byte UTF-8 sequences.
pub(super) fn tail_from_buffer(
    buffer: &Arc<Mutex<Vec<u8>>>,
    max_tail_chars: usize,
) -> (usize, String) {
    let guard = buffer.lock().unwrap_or_else(|e| e.into_inner());
    let total = guard.len();
    // Over-estimate byte count (4 bytes per char worst case for UTF-8).
    let mut tail_start = total.saturating_sub(max_tail_chars.saturating_mul(4));
    // Snap forward to the next valid UTF-8 codepoint boundary so we don't
    // pass a slice beginning with continuation bytes (0x80-0xBF) to
    // from_utf8_lossy, which would emit a leading U+FFFD replacement char.
    while tail_start < total && (guard[tail_start] & 0xC0) == 0x80 {
        tail_start += 1;
    }
    let tail_str = String::from_utf8_lossy(&guard[tail_start..]).into_owned();
    (total, tail_text(&tail_str, max_tail_chars))
}

fn tail_text(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let tail = text
        .chars()
        .rev()
        .take(max_chars)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<String>();
    format!("...{tail}")
}

#[cfg(test)]
mod tests {
    use super::take_delta_from_buffer;
    use std::sync::{Arc, Mutex};

    #[test]
    fn delta_holds_back_an_incomplete_trailing_utf8_sequence() {
        // "宽" is three bytes; deliver two of them, then the rest.
        let wide = "宽".as_bytes();
        let buffer = Arc::new(Mutex::new(b"ok ".to_vec()));
        buffer.lock().unwrap().extend_from_slice(&wide[..2]);
        let mut cursor = 0usize;

        let (delta, total) = take_delta_from_buffer(&buffer, &mut cursor);
        assert_eq!(
            String::from_utf8(delta).expect("delta must be whole characters"),
            "ok "
        );
        assert_eq!(total, 5, "total still reports every buffered byte");
        assert_eq!(cursor, 3, "the split character stays unread");

        buffer.lock().unwrap().extend_from_slice(&wide[2..]);
        let (delta, _) = take_delta_from_buffer(&buffer, &mut cursor);
        assert_eq!(
            String::from_utf8(delta).expect("delta must be whole characters"),
            "宽"
        );
    }

    #[test]
    fn delta_does_not_stall_on_genuinely_invalid_bytes() {
        // A lone 0xFF is never a valid start byte: passing it through keeps
        // binary output flowing instead of parking the cursor forever.
        let buffer = Arc::new(Mutex::new(vec![b'a', 0xFF, b'b']));
        let mut cursor = 0usize;
        let (delta, total) = take_delta_from_buffer(&buffer, &mut cursor);
        assert_eq!(delta, vec![b'a', 0xFF, b'b']);
        assert_eq!(cursor, total);
    }
}
