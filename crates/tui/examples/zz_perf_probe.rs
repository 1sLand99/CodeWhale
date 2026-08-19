use codewhale_tui::session_manager::{SavedSession, create_saved_session_with_id_and_mode};
use std::time::Instant;

fn main() {
    let path = std::env::args().nth(1).expect("path");
    let raw = std::fs::read_to_string(&path).unwrap();
    println!("file bytes: {}", raw.len());

    let t = Instant::now();
    let session: SavedSession = serde_json::from_str(&raw).unwrap();
    println!("typed parse: {:?}", t.elapsed());
    println!(
        "messages: {} journal entries: {}",
        session.messages.len(),
        session
            .journal
            .as_ref()
            .map(|j| j.entries.len())
            .unwrap_or(0)
    );

    // 1. to_string_pretty of whole session (what serialize_saved_session does)
    for _ in 0..3 {
        let t = Instant::now();
        let s = serde_json::to_string_pretty(&session).unwrap();
        println!(
            "to_string_pretty full: {:?} ({} bytes)",
            t.elapsed(),
            s.len()
        );
    }
    // 2. to_string_pretty with messages dropped (the proposed dedup)
    let mut deduped = session.clone();
    deduped.messages = Vec::new();
    for _ in 0..3 {
        let t = Instant::now();
        let s = serde_json::to_string_pretty(&deduped).unwrap();
        println!(
            "to_string_pretty journal-only: {:?} ({} bytes)",
            t.elapsed(),
            s.len()
        );
    }
    // 3. full deep clone of session
    for _ in 0..3 {
        let t = Instant::now();
        let c = session.clone();
        println!(
            "SavedSession::clone: {:?} (msgs {})",
            t.elapsed(),
            c.messages.len()
        );
    }
    // 4. journal.to_messages()
    let j = session.journal.as_ref().unwrap();
    for _ in 0..3 {
        let t = Instant::now();
        let m = j.to_messages();
        println!("journal.to_messages: {:?} ({} msgs)", t.elapsed(), m.len());
    }
    // 5. UI-thread cost: create_saved_session_with_id_and_mode over api_messages
    for _ in 0..3 {
        let t = Instant::now();
        let s = create_saved_session_with_id_and_mode(
            "id".to_string(),
            &session.messages,
            "m",
            std::path::Path::new("/tmp"),
            0,
            None,
            None,
        );
        println!(
            "create_saved_session_with_id_and_mode: {:?} ({} msgs)",
            t.elapsed(),
            s.messages.len()
        );
    }
    // 6. atomic write timing
    let content = serde_json::to_string_pretty(&session).unwrap();
    let dir = std::env::temp_dir();
    for _ in 0..3 {
        let t = Instant::now();
        let mut f = tempfile::NamedTempFile::new_in(&dir).unwrap();
        use std::io::Write;
        f.write_all(content.as_bytes()).unwrap();
        f.as_file().sync_all().unwrap();
        let p = dir.join("zz_perf_probe_out.json");
        f.persist(&p).unwrap();
        println!("atomic write+fsync: {:?}", t.elapsed());
    }
}
