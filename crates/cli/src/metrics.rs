//! `codewhale metrics` — reads the audit log and session/task stores and prints
//! a human-readable usage rollup.
//!
//! Data sources, all resolved through the shared Codewhale state resolvers so
//! the reader lands on the same files the writers use:
//! - `~/.codewhale/audit.log`   — one JSON line per event (approvals, credentials)
//! - `~/.codewhale/sessions/`   — saved session JSON files (tool call history)
//! - `~/.codewhale/tasks/runtime/events/` — runtime thread JSONL event streams
//!
//! Default-root audit history includes retained rotations and legacy receipts,
//! excluding records copied across roots. An explicit `CODEWHALE_HOME` never
//! reads outside that root.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use serde_json::Value;
use sha2::{Digest, Sha256};

// ──────────────────────────────────────────────────────────────────────────────
// Public entry-point
// ──────────────────────────────────────────────────────────────────────────────

/// Arguments accepted by `codewhale metrics`.
#[derive(Debug, Default)]
pub struct MetricsArgs {
    /// Emit machine-readable JSON instead of human text.
    pub json: bool,
    /// Restrict to events newer than this cutoff (inclusive).
    pub since: Option<DateTime<Utc>>,
}

pub fn run(args: MetricsArgs) -> Result<()> {
    // `resolve_state_dir` is the shared read-path resolver already used by
    // `doctor` and the session store; the runtime thread store hangs its event
    // streams off `<tasks>/runtime`. Resolving the home is fallible, and a
    // rollup of zeros is indistinguishable from real emptiness, so a home we
    // cannot resolve is an error rather than a silent all-zero report.
    let audit_roots = resolve_audit_roots()?;
    let sessions = codewhale_config::resolve_state_dir("sessions")?;
    let runtime_events = codewhale_config::resolve_state_dir("tasks")?
        .join("runtime")
        .join("events");

    // Collect data from every source; treat missing files as empty.
    let mut rollup = Rollup::default();
    read_audit_history(&audit_roots, args.since, &mut rollup);
    read_session_files(&sessions, args.since, &mut rollup);
    read_runtime_events(&runtime_events, args.since, &mut rollup);

    if args.json {
        print_json(&rollup)?;
    } else {
        print_human(&rollup);
    }

    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// Duration-string parser  ("7d", "24h", "30m", "2h", "now-2h", "2h30m")
// ──────────────────────────────────────────────────────────────────────────────

/// Parse a loose humantime-ish duration string into an absolute `DateTime<Utc>`
/// cutoff (i.e. `Utc::now() - duration`).
///
/// Accepted forms:
/// - `7d` / `24h` / `30m` / `90s`
/// - `2h30m`, `1d12h`
/// - `now-2h` (leading `now-` is stripped before parsing)
pub fn parse_since(s: &str) -> Result<DateTime<Utc>> {
    let s = s.trim().to_ascii_lowercase();
    let s = s.strip_prefix("now-").unwrap_or(&s);
    let secs = parse_duration_secs(s)?;
    Ok(Utc::now() - Duration::seconds(secs))
}

fn parse_duration_secs(s: &str) -> Result<i64> {
    // Walk through the string accumulating numbers and consuming unit suffixes.
    let mut total: i64 = 0;
    let mut num_buf = String::new();

    for ch in s.chars() {
        match ch {
            '0'..='9' => num_buf.push(ch),
            'd' | 'h' | 'm' | 's' => {
                let n: i64 = num_buf
                    .parse()
                    .map_err(|_| anyhow::anyhow!("invalid duration component: {num_buf:?}"))?;
                num_buf.clear();
                let factor = match ch {
                    'd' => 86_400,
                    'h' => 3_600,
                    'm' => 60,
                    's' => 1,
                    _ => unreachable!(),
                };
                total += n * factor;
            }
            _ => anyhow::bail!("unrecognised character {ch:?} in duration {s:?}"),
        }
    }

    if !num_buf.is_empty() {
        // Trailing bare number — treat as seconds.
        let n: i64 = num_buf.parse()?;
        total += n;
    }

    if total == 0 {
        anyhow::bail!("duration {s:?} resolved to zero seconds");
    }

    Ok(total)
}

// ──────────────────────────────────────────────────────────────────────────────
// Rollup data model
// ──────────────────────────────────────────────────────────────────────────────

/// Per-tool aggregated counters.
#[derive(Debug, Default, serde::Serialize)]
pub struct ToolStats {
    pub calls: u64,
    /// Calls that were auto-approved (no prompt required).
    pub auto_approved: u64,
    /// Calls that required a manual prompt.
    pub prompted: u64,
    /// Total elapsed ms (from events that carry this field).
    pub total_elapsed_ms: u64,
    /// Number of elapsed_ms samples included in `total_elapsed_ms`.
    pub elapsed_samples: u64,
    /// Successful calls (where we have result data).
    pub successes: u64,
    /// Failed calls.
    pub failures: u64,
}

impl ToolStats {
    fn success_rate_pct(&self) -> Option<f64> {
        let judged = self.successes + self.failures;
        if judged == 0 {
            None
        } else {
            Some(self.successes as f64 / judged as f64 * 100.0)
        }
    }

    fn avg_elapsed_ms(&self) -> Option<u64> {
        self.total_elapsed_ms.checked_div(self.elapsed_samples)
    }
}

/// Compaction event stats.
#[derive(Debug, Default, serde::Serialize)]
pub struct CompactionStats {
    pub events: u64,
    /// Sum of `reduction_ratio` from events that carry it (0.0–1.0 each).
    pub ratio_sum: f64,
    pub ratio_samples: u64,
}

impl CompactionStats {
    fn avg_reduction_pct(&self) -> Option<f64> {
        if self.ratio_samples == 0 {
            None
        } else {
            Some(self.ratio_sum / self.ratio_samples as f64 * 100.0)
        }
    }
}

/// Sub-agent lifecycle receipt counts; these are not unique worker totals.
#[derive(Debug, Default, serde::Serialize)]
pub struct AgentStats {
    pub spawns: u64,
    pub successes: u64,
    pub failures: u64,
    pub cancelled: u64,
    pub interrupted: u64,
    pub budget_exhausted: u64,
    /// Terminal receipts with missing, malformed, or unrecognized outcomes.
    pub unknown_outcomes: u64,
}

impl AgentStats {
    fn record_completion(&mut self, event: &Value) {
        // Runtime's worker_status owns the outcome. A completed status item
        // means its receipt settled, not that the worker succeeded. Preserve
        // explicit unknown values instead of falling back to a legacy boolean.
        let status = event
            .pointer("/details/worker_status")
            .or_else(|| event.pointer("/payload/worker_status"))
            .or_else(|| event.pointer("/details/status"))
            .or_else(|| event.pointer("/payload/status"));
        let count = if let Some(status) = status {
            match status.as_str() {
                Some("completed") => &mut self.successes,
                Some("failed") => &mut self.failures,
                Some("cancelled") => &mut self.cancelled,
                Some("interrupted") => &mut self.interrupted,
                Some("budget_exhausted") => &mut self.budget_exhausted,
                _ => &mut self.unknown_outcomes,
            }
        } else {
            match event
                .pointer("/details/success")
                .or_else(|| event.pointer("/payload/success"))
                .and_then(Value::as_bool)
            {
                Some(true) => &mut self.successes,
                Some(false) => &mut self.failures,
                None => &mut self.unknown_outcomes,
            }
        };
        *count = count.saturating_add(1);
    }

    fn summary(&self) -> String {
        let outcomes = [
            (self.successes, "completed"),
            (self.failures, "failed"),
            (self.cancelled, "cancelled"),
            (self.interrupted, "interrupted"),
            (self.budget_exhausted, "budget exhausted"),
            (self.unknown_outcomes, "outcome unconfirmed"),
        ]
        .into_iter()
        .filter(|(count, _)| *count > 0)
        .map(|(count, label)| format!("{} {label}", fmt_num(count)))
        .collect::<Vec<_>>();
        if self.spawns == 0 && outcomes.is_empty() {
            return "Sub-agents: (no data)".to_string();
        }
        let mut summary = format!("Sub-agents: {} spawn receipts", fmt_num(self.spawns));
        if !outcomes.is_empty() {
            summary.push_str(&format!("; outcomes: {}", outcomes.join(", ")));
        }
        summary
    }
}

/// Capacity-controller / rate-limit intervention stats.
#[derive(Debug, Default, serde::Serialize)]
pub struct CapacityStats {
    pub total: u64,
    pub by_category: HashMap<String, u64>,
}

/// Credential / session event stats (from audit log).
#[derive(Debug, Default, serde::Serialize)]
pub struct CredentialStats {
    pub saves: u64,
    pub clears: u64,
}

/// Runtime receipts for model-client dispatch and provider-reported usage.
///
/// These are deliberately not billing records: the terminal diagnostics count
/// parent model-client calls, while `turn.usage` exists only when a provider
/// supplied usage for one call. Client-internal HTTP retries and invoices are
/// outside both receipts.
#[derive(Debug, Default, serde::Serialize)]
pub struct RuntimeRequestStats {
    /// Distinct durable `turn.completed` receipts with a usable `(thread, turn)` identity.
    pub terminal_turn_receipts: u64,
    /// Terminal receipts carrying the optional request diagnostics projection.
    pub diagnostics_turn_receipts: u64,
    /// Terminal receipts from older or partial logs with no diagnostics projection.
    pub diagnostics_unavailable_turn_receipts: u64,
    /// Present-but-incomplete diagnostics are unknown rather than zero.
    pub diagnostics_incomplete_turn_receipts: u64,
    /// Terminal receipts omitted because their identity could not be verified.
    pub terminal_receipts_without_identity: u64,
    /// Repeated terminal snapshots for one `(thread, turn)` omitted from the rollup.
    pub duplicate_terminal_receipts_skipped: u64,
    /// Parent model-client calls recorded by terminal diagnostics, not HTTP retries or invoices.
    pub model_requests_started: u64,
    pub transparent_stream_retries: u64,
    pub stream_resumes: u64,
    /// Distinct `turn.usage` receipts with a verified runtime event identity.
    pub provider_usage_receipts: u64,
    /// `turn.usage` records that could not be identified, so their values are unknown.
    pub provider_usage_receipts_without_identity: u64,
    /// Repeated runtime event identities omitted from provider usage totals.
    pub duplicate_provider_usage_receipts_skipped: u64,
    /// `turn.usage` records missing either required token total are not treated as zero.
    pub provider_usage_receipts_incomplete: u64,
    /// Provider-reported per-request input tokens only; terminal cumulative snapshots are excluded.
    pub provider_reported_input_tokens: u64,
    /// Provider-reported per-request output tokens only; terminal cumulative snapshots are excluded.
    pub provider_reported_output_tokens: u64,
}

/// Top-level rollup.
#[derive(Debug, Default, serde::Serialize)]
pub struct Rollup {
    /// UTC timestamp of the earliest event we've seen.
    pub earliest_ts: Option<DateTime<Utc>>,
    /// UTC timestamp of the latest event we've seen.
    pub latest_ts: Option<DateTime<Utc>>,
    /// Per-tool stats keyed by tool name.
    pub tools: HashMap<String, ToolStats>,
    pub compaction: CompactionStats,
    pub agents: AgentStats,
    pub capacity: CapacityStats,
    pub credentials: CredentialStats,
    pub runtime_requests: RuntimeRequestStats,
    /// Total lines read across all sources.
    pub total_lines: u64,
    /// Lines successfully parsed.
    pub parsed_lines: u64,
}

#[derive(Default)]
struct RuntimeEventDedup {
    terminal_turns: HashSet<(String, String)>,
    event_records: HashSet<(String, u64)>,
}

impl Rollup {
    fn touch_ts(&mut self, ts: &DateTime<Utc>) {
        match self.earliest_ts {
            None => self.earliest_ts = Some(*ts),
            Some(ref cur) if ts < cur => self.earliest_ts = Some(*ts),
            _ => {}
        }
        match self.latest_ts {
            None => self.latest_ts = Some(*ts),
            Some(ref cur) if ts > cur => self.latest_ts = Some(*ts),
            _ => {}
        }
    }

    fn tool_mut(&mut self, name: &str) -> &mut ToolStats {
        self.tools.entry(name.to_string()).or_default()
    }

    fn total_tool_calls(&self) -> u64 {
        self.tools.values().map(|t| t.calls).sum()
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Source readers
// ──────────────────────────────────────────────────────────────────────────────

/// Read both retained generations from each root. A copied legacy record is
/// counted once across roots, while repeated records within one root retain
/// their multiplicity. No source log is rewritten or removed.
fn read_audit_history(roots: &[PathBuf], since: Option<DateTime<Utc>>, rollup: &mut Rollup) {
    let mut earlier_roots = HashMap::new();
    for root in roots {
        let mut root_counts = HashMap::new();
        for name in ["audit.log.1", "audit.log"] {
            read_audit_log(
                &root.join(name),
                since,
                rollup,
                &earlier_roots,
                &mut root_counts,
            );
        }
        for (record, count) in root_counts {
            let prior = earlier_roots.entry(record).or_insert(0);
            *prior = (*prior).max(count);
        }
    }
}

/// Read one JSON event per line, excluding copies already seen in other roots.
fn read_audit_log(
    path: &Path,
    since: Option<DateTime<Utc>>,
    rollup: &mut Rollup,
    earlier_roots: &HashMap<[u8; 32], u64>,
    root_counts: &mut HashMap<[u8; 32], u64>,
) {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return,
        Err(e) => {
            tracing::trace!(
                "metrics: could not read audit log {}: {}",
                path.display(),
                e
            );
            return;
        }
    };

    for raw_line in content.lines() {
        rollup.total_lines += 1;
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }

        let v: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(e) => {
                tracing::trace!("metrics: skipping malformed audit line: {e}");
                continue;
            }
        };

        // Copy migration preserves the complete event, including its timestamp.
        // Count occurrences so two identical legitimate records in one source
        // are not collapsed into one merely because another root also exists.
        let fingerprint: [u8; 32] = Sha256::digest(v.to_string().as_bytes()).into();
        let count = root_counts.entry(fingerprint).or_insert(0);
        *count += 1;
        if *count <= earlier_roots.get(&fingerprint).copied().unwrap_or(0) {
            continue;
        }

        // Parse timestamp — field is "ts" in audit log.
        let ts = parse_ts_field(&v, "ts");

        if let Some(cutoff) = since {
            match ts {
                Some(t) if t < cutoff => continue,
                _ => {}
            }
        }

        rollup.parsed_lines += 1;
        if let Some(t) = &ts {
            rollup.touch_ts(t);
        }

        let event = v.get("event").and_then(|e| e.as_str()).unwrap_or("");

        match event {
            "tool.approval.auto_approve" => {
                let tool_name = v
                    .pointer("/details/tool_name")
                    .and_then(|t| t.as_str())
                    .unwrap_or("unknown");
                let stats = rollup.tool_mut(tool_name);
                stats.calls += 1;
                stats.auto_approved += 1;
            }
            "tool.approval.prompted" => {
                let tool_name = v
                    .pointer("/details/tool_name")
                    .and_then(|t| t.as_str())
                    .unwrap_or("unknown");
                let stats = rollup.tool_mut(tool_name);
                stats.calls += 1;
                stats.prompted += 1;
            }
            "tool.completed" | "tool.result" => {
                let tool_name = v
                    .pointer("/details/tool_name")
                    .or_else(|| v.pointer("/payload/tool_name"))
                    .and_then(|t| t.as_str())
                    .unwrap_or("unknown");
                let stats = rollup.tool_mut(tool_name);
                stats.calls += 1;

                // Optional elapsed_ms
                if let Some(ms) = v
                    .pointer("/details/elapsed_ms")
                    .or_else(|| v.pointer("/payload/elapsed_ms"))
                    .and_then(|v| v.as_u64())
                {
                    stats.total_elapsed_ms += ms;
                    stats.elapsed_samples += 1;
                }

                // Success / failure
                let success = v
                    .pointer("/details/success")
                    .or_else(|| v.pointer("/payload/success"))
                    .and_then(|b| b.as_bool())
                    .unwrap_or(true);
                if success {
                    stats.successes += 1;
                } else {
                    stats.failures += 1;
                }
            }
            "compaction.completed" | "context.compaction" => {
                rollup.compaction.events += 1;
                if let Some(ratio) = v
                    .pointer("/details/reduction_ratio")
                    .or_else(|| v.pointer("/payload/reduction_ratio"))
                    .and_then(|r| r.as_f64())
                {
                    rollup.compaction.ratio_sum += ratio;
                    rollup.compaction.ratio_samples += 1;
                }
            }
            "agent.spawn" | "subagent.spawned" => {
                rollup.agents.spawns += 1;
            }
            "agent.completed" | "subagent.completed" => {
                rollup.agents.record_completion(&v);
            }
            e if e.starts_with("capacity.") => {
                rollup.capacity.total += 1;
                let category = v
                    .pointer("/details/category")
                    .or_else(|| v.pointer("/payload/category"))
                    .and_then(|c| c.as_str())
                    .unwrap_or(e.trim_start_matches("capacity."));
                *rollup
                    .capacity
                    .by_category
                    .entry(category.to_string())
                    .or_insert(0) += 1;
            }
            "credential.save" => {
                rollup.credentials.saves += 1;
            }
            "credential.clear" => {
                rollup.credentials.clears += 1;
            }
            _ => {
                // Unknown event — tracked in parsed_lines but otherwise ignored.
            }
        }
    }
}

/// Read session JSON files under `sessions/` (one per session).
/// These carry tool call history with optional elapsed_ms and result data.
fn read_session_files(sessions_dir: &Path, since: Option<DateTime<Utc>>, rollup: &mut Rollup) {
    let rd = match std::fs::read_dir(sessions_dir) {
        Ok(rd) => rd,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return,
        Err(e) => {
            tracing::trace!(
                "metrics: could not list sessions dir {}: {}",
                sessions_dir.display(),
                e
            );
            return;
        }
    };

    for entry in rd.flatten() {
        let path = entry.path();
        // Only look at .json files directly in sessions/; skip sub-dirs.
        if path.is_dir() || path.extension().map(|e| e != "json").unwrap_or(true) {
            continue;
        }
        read_session_file(&path, since, rollup);
    }
}

fn read_session_file(path: &Path, since: Option<DateTime<Utc>>, rollup: &mut Rollup) {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            tracing::trace!(
                "metrics: could not read session file {}: {}",
                path.display(),
                e
            );
            return;
        }
    };

    rollup.total_lines += 1;

    let v: Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(e) => {
            tracing::trace!(
                "metrics: skipping malformed session file {}: {}",
                path.display(),
                e
            );
            return;
        }
    };

    rollup.parsed_lines += 1;

    // Session-level timestamp filter (check metadata.created_at or updated_at).
    let session_ts = v
        .pointer("/metadata/updated_at")
        .or_else(|| v.pointer("/metadata/created_at"))
        .and_then(|t| t.as_str())
        .and_then(|s| s.parse::<DateTime<Utc>>().ok());

    if let Some(cutoff) = since
        && let Some(ts) = &session_ts
        && *ts < cutoff
    {
        return;
    }

    if let Some(ts) = session_ts {
        rollup.touch_ts(&ts);
    }

    // Walk messages looking for tool_use calls with associated results.
    let messages = match v.get("messages").and_then(|m| m.as_array()) {
        Some(m) => m,
        None => return,
    };

    // Build a map from tool_use_id → (tool_name, elapsed_ms_option, started_at_option).
    let mut pending: HashMap<String, (String, Option<u64>)> = HashMap::new();

    for msg in messages {
        let role = msg.get("role").and_then(|r| r.as_str()).unwrap_or("");
        let content_arr = match msg.get("content").and_then(|c| c.as_array()) {
            Some(c) => c,
            None => continue,
        };

        for block in content_arr {
            let block_type = block.get("type").and_then(|t| t.as_str()).unwrap_or("");
            match (role, block_type) {
                ("assistant", "tool_use") => {
                    let id = block.get("id").and_then(|i| i.as_str()).unwrap_or("");
                    let name = block
                        .get("name")
                        .and_then(|n| n.as_str())
                        .unwrap_or("unknown");
                    let elapsed_ms = block.get("elapsed_ms").and_then(|e| e.as_u64());
                    if !id.is_empty() {
                        pending.insert(id.to_string(), (name.to_string(), elapsed_ms));
                    }
                }
                ("user", "tool_result") => {
                    let id = block
                        .get("tool_use_id")
                        .and_then(|i| i.as_str())
                        .unwrap_or("");
                    if let Some((name, elapsed_ms)) = pending.remove(id) {
                        let stats = rollup.tool_mut(&name);
                        // Only count if not already counted via audit log (we don't de-dup, so
                        // session files may double-count approvals; that's acceptable — users who
                        // want precise counts should use --json and cross-reference).
                        stats.calls += 1;
                        if let Some(ms) = elapsed_ms {
                            stats.total_elapsed_ms += ms;
                            stats.elapsed_samples += 1;
                        }
                        // Tool result success: absence of "is_error": true
                        let is_error = block
                            .get("is_error")
                            .and_then(|e| e.as_bool())
                            .unwrap_or(false);
                        if is_error {
                            stats.failures += 1;
                        } else {
                            stats.successes += 1;
                        }
                    }
                }
                _ => {}
            }
        }
    }

    // Walk messages for compaction events embedded as special user messages.
    for msg in messages {
        if let Some(compaction) = msg
            .get("compaction")
            .or_else(|| msg.pointer("/metadata/compaction"))
        {
            rollup.compaction.events += 1;
            if let Some(ratio) = compaction.get("reduction_ratio").and_then(|r| r.as_f64()) {
                rollup.compaction.ratio_sum += ratio;
                rollup.compaction.ratio_samples += 1;
            }
        }
    }
}

/// Read JSONL event streams from the tasks runtime events directory.
fn read_runtime_events(events_dir: &Path, since: Option<DateTime<Utc>>, rollup: &mut Rollup) {
    let rd = match std::fs::read_dir(events_dir) {
        Ok(rd) => rd,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return,
        Err(e) => {
            tracing::trace!(
                "metrics: could not list events dir {}: {}",
                events_dir.display(),
                e
            );
            return;
        }
    };

    let mut dedup = RuntimeEventDedup::default();
    for entry in rd.flatten() {
        let path = entry.path();
        if path.extension().map(|e| e != "jsonl").unwrap_or(true) {
            continue;
        }
        read_events_jsonl(&path, since, rollup, &mut dedup);
    }
}

fn read_events_jsonl(
    path: &Path,
    since: Option<DateTime<Utc>>,
    rollup: &mut Rollup,
    dedup: &mut RuntimeEventDedup,
) {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            tracing::trace!(
                "metrics: could not read events file {}: {}",
                path.display(),
                e
            );
            return;
        }
    };

    for raw_line in content.lines() {
        rollup.total_lines += 1;
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }

        let v: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(e) => {
                tracing::trace!("metrics: skipping malformed event line: {e}");
                continue;
            }
        };

        let ts = parse_ts_field(&v, "timestamp");

        if let Some(cutoff) = since {
            match ts {
                Some(t) if t < cutoff => continue,
                _ => {}
            }
        }

        rollup.parsed_lines += 1;
        if let Some(t) = &ts {
            rollup.touch_ts(t);
        }

        let event = v.get("event").and_then(|e| e.as_str()).unwrap_or("");

        match event {
            "turn.completed" => record_terminal_request_diagnostics(&v, rollup, dedup),
            "turn.usage" => record_provider_usage_receipt(&v, rollup, dedup),
            "tool.started" | "tool.completed" | "tool.failed" => {
                let tool_name = v
                    .pointer("/payload/tool_name")
                    .or_else(|| v.pointer("/payload/name"))
                    .and_then(|t| t.as_str())
                    .unwrap_or("unknown");
                let stats = rollup.tool_mut(tool_name);

                if event == "tool.started" {
                    stats.calls += 1;
                } else if event == "tool.completed" {
                    stats.successes += 1;
                    if let Some(ms) = v.pointer("/payload/elapsed_ms").and_then(|v| v.as_u64()) {
                        stats.total_elapsed_ms += ms;
                        stats.elapsed_samples += 1;
                    }
                } else {
                    // tool.failed
                    stats.failures += 1;
                }
            }
            "compaction.completed" => {
                rollup.compaction.events += 1;
                if let Some(ratio) = v
                    .pointer("/payload/reduction_ratio")
                    .and_then(|r| r.as_f64())
                {
                    rollup.compaction.ratio_sum += ratio;
                    rollup.compaction.ratio_samples += 1;
                }
            }
            "agent.spawned" | "subagent.spawned" => {
                rollup.agents.spawns += 1;
            }
            "agent.completed" | "subagent.completed" => {
                rollup.agents.record_completion(&v);
            }
            e if e.starts_with("capacity.") => {
                rollup.capacity.total += 1;
                let category = v
                    .pointer("/payload/category")
                    .and_then(|c| c.as_str())
                    .unwrap_or(e.trim_start_matches("capacity."));
                *rollup
                    .capacity
                    .by_category
                    .entry(category.to_string())
                    .or_insert(0) += 1;
            }
            _ => {}
        }
    }
}

fn runtime_event_identity(v: &Value) -> Option<(String, u64)> {
    Some((
        v.get("thread_id")?.as_str()?.to_string(),
        v.get("seq")?.as_u64()?,
    ))
}

fn terminal_turn_identity(v: &Value) -> Option<(String, String)> {
    Some((
        v.get("thread_id")?.as_str()?.to_string(),
        v.get("turn_id")?.as_str()?.to_string(),
    ))
}

fn record_terminal_request_diagnostics(
    v: &Value,
    rollup: &mut Rollup,
    dedup: &mut RuntimeEventDedup,
) {
    let Some(identity) = terminal_turn_identity(v) else {
        rollup.runtime_requests.terminal_receipts_without_identity = rollup
            .runtime_requests
            .terminal_receipts_without_identity
            .saturating_add(1);
        return;
    };
    if !dedup.terminal_turns.insert(identity) {
        rollup.runtime_requests.duplicate_terminal_receipts_skipped = rollup
            .runtime_requests
            .duplicate_terminal_receipts_skipped
            .saturating_add(1);
        return;
    }

    let stats = &mut rollup.runtime_requests;
    stats.terminal_turn_receipts = stats.terminal_turn_receipts.saturating_add(1);
    let Some(diagnostics) = v.pointer("/payload/turn/modelRequestDiagnostics") else {
        stats.diagnostics_unavailable_turn_receipts = stats
            .diagnostics_unavailable_turn_receipts
            .saturating_add(1);
        return;
    };
    let Some(model_requests_started) = diagnostics
        .get("modelRequestsStarted")
        .and_then(Value::as_u64)
    else {
        stats.diagnostics_incomplete_turn_receipts =
            stats.diagnostics_incomplete_turn_receipts.saturating_add(1);
        return;
    };
    let Some(transparent_stream_retries) = diagnostics
        .get("transparentStreamRetries")
        .and_then(Value::as_u64)
    else {
        stats.diagnostics_incomplete_turn_receipts =
            stats.diagnostics_incomplete_turn_receipts.saturating_add(1);
        return;
    };
    let Some(stream_resumes) = diagnostics.get("streamResumes").and_then(Value::as_u64) else {
        stats.diagnostics_incomplete_turn_receipts =
            stats.diagnostics_incomplete_turn_receipts.saturating_add(1);
        return;
    };

    stats.diagnostics_turn_receipts = stats.diagnostics_turn_receipts.saturating_add(1);
    stats.model_requests_started = stats
        .model_requests_started
        .saturating_add(model_requests_started);
    stats.transparent_stream_retries = stats
        .transparent_stream_retries
        .saturating_add(transparent_stream_retries);
    stats.stream_resumes = stats.stream_resumes.saturating_add(stream_resumes);
}

fn record_provider_usage_receipt(v: &Value, rollup: &mut Rollup, dedup: &mut RuntimeEventDedup) {
    let Some(identity) = runtime_event_identity(v) else {
        rollup
            .runtime_requests
            .provider_usage_receipts_without_identity = rollup
            .runtime_requests
            .provider_usage_receipts_without_identity
            .saturating_add(1);
        return;
    };
    if !dedup.event_records.insert(identity) {
        rollup
            .runtime_requests
            .duplicate_provider_usage_receipts_skipped = rollup
            .runtime_requests
            .duplicate_provider_usage_receipts_skipped
            .saturating_add(1);
        return;
    }

    let stats = &mut rollup.runtime_requests;
    let Some(input_tokens) = v
        .pointer("/payload/usage/input_tokens")
        .and_then(Value::as_u64)
    else {
        stats.provider_usage_receipts_incomplete =
            stats.provider_usage_receipts_incomplete.saturating_add(1);
        return;
    };
    let Some(output_tokens) = v
        .pointer("/payload/usage/output_tokens")
        .and_then(Value::as_u64)
    else {
        stats.provider_usage_receipts_incomplete =
            stats.provider_usage_receipts_incomplete.saturating_add(1);
        return;
    };
    stats.provider_usage_receipts = stats.provider_usage_receipts.saturating_add(1);
    stats.provider_reported_input_tokens = stats
        .provider_reported_input_tokens
        .saturating_add(input_tokens);
    stats.provider_reported_output_tokens = stats
        .provider_reported_output_tokens
        .saturating_add(output_tokens);
}

// ──────────────────────────────────────────────────────────────────────────────
// Output formatters
// ──────────────────────────────────────────────────────────────────────────────

fn print_json(rollup: &Rollup) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(rollup)?);
    Ok(())
}

fn print_human(rollup: &Rollup) {
    // Period header
    match (rollup.earliest_ts, rollup.latest_ts) {
        (Some(start), Some(end)) => {
            let days = (end - start).num_days();
            println!(
                "Period: {} → {} ({} days)",
                start.format("%Y-%m-%d"),
                end.format("%Y-%m-%d"),
                days
            );
        }
        (Some(start), None) | (None, Some(start)) => {
            println!("Period: {} → (unknown)", start.format("%Y-%m-%d"));
        }
        (None, None) => {
            println!("Period: (no data)");
        }
    }

    // ── Tools ──────────────────────────────────────────────────────────────
    let total_calls = rollup.total_tool_calls();
    if total_calls > 0 {
        // Overall success rate from session-file data (where we have result info).
        let total_ok: u64 = rollup.tools.values().map(|t| t.successes).sum();
        let total_judged: u64 = rollup
            .tools
            .values()
            .map(|t| t.successes + t.failures)
            .sum();
        let overall_rate = if total_judged > 0 {
            format!(
                "{:.1}% success",
                total_ok as f64 / total_judged as f64 * 100.0
            )
        } else {
            // Only approval events — show prompt breakdown.
            let auto: u64 = rollup.tools.values().map(|t| t.auto_approved).sum();
            let prompted: u64 = rollup.tools.values().map(|t| t.prompted).sum();
            format!("{auto} auto-approved, {prompted} prompted")
        };

        println!(
            "Tools: {:>6} calls ({})",
            fmt_num(total_calls),
            overall_rate
        );

        // Sort tools by call count descending, top 15.
        let mut tools: Vec<(&String, &ToolStats)> = rollup.tools.iter().collect();
        tools.sort_by_key(|b| std::cmp::Reverse(b.1.calls));
        for (name, stats) in tools.iter().take(15) {
            let rate_str = match stats.success_rate_pct() {
                Some(pct) => format!("{pct:5.1}%"),
                None => {
                    // Only approval data available — show auto/prompted breakdown.
                    let a = stats.auto_approved;
                    let p = stats.prompted;
                    if p == 0 {
                        format!("auto×{a}  ")
                    } else {
                        format!("auto×{a}/prompted×{p}")
                    }
                }
            };
            let avg_str = match stats.avg_elapsed_ms() {
                Some(ms) => format!("  avg {ms}ms"),
                None => String::new(),
            };
            println!(
                "  {name:<22} {:>6}  {rate_str}{avg_str}",
                fmt_num(stats.calls)
            );
        }
        if tools.len() > 15 {
            println!("  … and {} more tools", tools.len() - 15);
        }
    } else {
        println!("Tools: (no data)");
    }

    // ── Compaction ─────────────────────────────────────────────────────────
    if rollup.compaction.events > 0 {
        let avg_str = match rollup.compaction.avg_reduction_pct() {
            Some(pct) => format!(", avg {pct:.0}% size reduction"),
            None => String::new(),
        };
        println!(
            "Compaction: {} events{}",
            fmt_num(rollup.compaction.events),
            avg_str
        );
    } else {
        println!("Compaction: (no data)");
    }

    // ── Sub-agents ─────────────────────────────────────────────────────────
    println!("{}", rollup.agents.summary());

    // ── Capacity interventions ─────────────────────────────────────────────
    if rollup.capacity.total > 0 {
        let cat_str: String = {
            let mut cats: Vec<(&String, &u64)> = rollup.capacity.by_category.iter().collect();
            cats.sort_by(|a, b| b.1.cmp(a.1));
            cats.iter()
                .map(|(k, v)| format!("{v} {k}"))
                .collect::<Vec<_>>()
                .join(", ")
        };
        println!(
            "Capacity interventions: {} ({})",
            fmt_num(rollup.capacity.total),
            cat_str
        );
    } else {
        println!("Capacity interventions: (no data)");
    }

    // ── Runtime request and provider-usage receipts ───────────────────────
    let runtime = &rollup.runtime_requests;
    if runtime.terminal_turn_receipts == 0 {
        println!("Runtime requests: (no terminal receipts; model-client counts unknown)");
    } else if runtime.diagnostics_turn_receipts == 0 {
        println!(
            "Runtime requests: (diagnostics unavailable for all {} terminal receipts)",
            fmt_num(runtime.terminal_turn_receipts)
        );
    } else {
        println!(
            "Runtime requests: {} model-client calls, {} stream resumes, {} transparent retries (diagnostics for {}/{} terminal receipts; status events excluded)",
            fmt_num(runtime.model_requests_started),
            fmt_num(runtime.stream_resumes),
            fmt_num(runtime.transparent_stream_retries),
            fmt_num(runtime.diagnostics_turn_receipts),
            fmt_num(runtime.terminal_turn_receipts),
        );
    }
    if runtime.provider_usage_receipts == 0 {
        println!("Provider usage receipts: (none recorded; this is not zero usage)");
    } else {
        println!(
            "Provider usage receipts: {} records, {} input tokens, {} output tokens",
            fmt_num(runtime.provider_usage_receipts),
            fmt_num(runtime.provider_reported_input_tokens),
            fmt_num(runtime.provider_reported_output_tokens),
        );
    }
    if runtime.diagnostics_unavailable_turn_receipts > 0
        || runtime.diagnostics_incomplete_turn_receipts > 0
        || runtime.terminal_receipts_without_identity > 0
        || runtime.provider_usage_receipts_without_identity > 0
        || runtime.provider_usage_receipts_incomplete > 0
        || runtime.duplicate_terminal_receipts_skipped > 0
        || runtime.duplicate_provider_usage_receipts_skipped > 0
    {
        println!(
            "Runtime receipt coverage: {} diagnostics unavailable, {} diagnostics incomplete, {} terminal receipts without identity, {} usage receipts without identity, {} usage receipts incomplete, {} duplicate terminal receipts skipped, {} duplicate usage receipts skipped",
            fmt_num(runtime.diagnostics_unavailable_turn_receipts),
            fmt_num(runtime.diagnostics_incomplete_turn_receipts),
            fmt_num(runtime.terminal_receipts_without_identity),
            fmt_num(runtime.provider_usage_receipts_without_identity),
            fmt_num(runtime.provider_usage_receipts_incomplete),
            fmt_num(runtime.duplicate_terminal_receipts_skipped),
            fmt_num(runtime.duplicate_provider_usage_receipts_skipped),
        );
    }

    // ── Credentials ────────────────────────────────────────────────────────
    if rollup.credentials.saves > 0 || rollup.credentials.clears > 0 {
        println!(
            "Credentials: {} saves, {} clears",
            rollup.credentials.saves, rollup.credentials.clears
        );
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Helpers
// ──────────────────────────────────────────────────────────────────────────────

/// An explicit home is an isolation boundary. Default installs can have
/// distinct audit histories in both roots, even after a copied migration.
fn resolve_audit_roots() -> Result<Vec<PathBuf>> {
    let primary = codewhale_config::codewhale_home()?;
    let mut roots = vec![primary];
    if !codewhale_config::codewhale_home_is_explicit() {
        let legacy = codewhale_config::legacy_deepseek_home()?;
        if !roots.contains(&legacy) {
            roots.push(legacy);
        }
    }
    Ok(roots)
}

/// Parse a timestamp from a JSON value field (tries RFC3339).
fn parse_ts_field(v: &Value, field: &str) -> Option<DateTime<Utc>> {
    v.get(field)?.as_str()?.parse::<DateTime<Utc>>().ok()
}

/// Format a number with thousands separators.
fn fmt_num(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::with_capacity(s.len() + s.len() / 3);
    for (i, ch) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(ch);
    }
    result.chars().rev().collect()
}

// ──────────────────────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn read_audit_test_log(path: &Path, since: Option<DateTime<Utc>>, rollup: &mut Rollup) {
        super::read_audit_log(path, since, rollup, &HashMap::new(), &mut HashMap::new());
    }

    fn read_runtime_test_log(path: &Path, since: Option<DateTime<Utc>>, rollup: &mut Rollup) {
        super::read_events_jsonl(path, since, rollup, &mut RuntimeEventDedup::default());
    }

    fn runtime_event(
        seq: u64,
        timestamp: &str,
        thread_id: &str,
        turn_id: Option<&str>,
        event: &str,
        payload: Value,
    ) -> Value {
        serde_json::json!({
            "schema_version": 4,
            "seq": seq,
            "timestamp": timestamp,
            "thread_id": thread_id,
            "turn_id": turn_id,
            "event": event,
            "payload": payload,
        })
    }

    fn write_runtime_events(events: &[Value]) -> tempfile::NamedTempFile {
        use std::io::Write;

        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        for event in events {
            writeln!(tmp, "{event}").unwrap();
        }
        tmp
    }

    #[test]
    fn runtime_worker_completion_uses_owner_outcome_not_completed_receipt_status() {
        let statuses = [
            serde_json::json!("completed"),
            serde_json::json!("failed"),
            serde_json::json!("cancelled"),
            serde_json::json!("interrupted"),
            serde_json::json!("budget_exhausted"),
            Value::Null,
        ];
        let events: Vec<_> = statuses
            .into_iter()
            .enumerate()
            .map(|(seq, worker_status)| {
                runtime_event(
                    seq as u64,
                    "2026-09-08T10:00:00Z",
                    "thread-a",
                    Some("turn-a"),
                    "agent.completed",
                    serde_json::json!({
                        "item": { "kind": "status", "status": "completed" },
                        "agent_id": format!("worker-{seq}"),
                        "worker_status": worker_status,
                        "parent_run_id": "run-a",
                        "spawn_depth": 1,
                        "continuable": false,
                    }),
                )
            })
            .collect();
        let tmp = write_runtime_events(&events);
        let mut rollup = Rollup::default();
        read_runtime_test_log(tmp.path(), None, &mut rollup);
        let agents = &rollup.agents;
        assert_eq!(agents.successes, 1, "a settled item is not worker success");
        assert_eq!(agents.failures, 1);
        assert_eq!(agents.cancelled, 1);
        assert_eq!(agents.interrupted, 1);
        assert_eq!(agents.budget_exhausted, 1);
        assert_eq!(agents.unknown_outcomes, 1);
        assert_eq!(agents.spawns, 0);
        let summary = agents.summary();
        assert!(summary.contains("1 failed"));
        assert!(summary.contains("1 outcome unconfirmed"));
        assert!(
            !summary.contains("no data"),
            "terminal-only windows have data"
        );
        assert!(
            !summary.contains("%"),
            "partial receipts are not a success rate"
        );
    }

    #[test]
    fn runtime_legacy_worker_receipts_require_explicit_success_evidence() {
        let payloads = [
            serde_json::json!({ "success": true }),
            serde_json::json!({ "success": false }),
            serde_json::json!({}),
            serde_json::json!({ "success": "true" }),
            serde_json::json!({ "worker_status": "failed", "success": true }),
            serde_json::json!({ "worker_status": null, "success": true }),
            serde_json::json!({ "worker_status": "running", "success": true }),
            serde_json::json!({ "worker_status": { "completed": true }, "success": true }),
            serde_json::json!({ "worker_status": "future_outcome", "success": true }),
        ];
        let events: Vec<_> = payloads
            .into_iter()
            .enumerate()
            .map(|(seq, payload)| {
                runtime_event(
                    seq as u64,
                    "2026-09-08T10:00:00Z",
                    "thread-a",
                    Some("turn-a"),
                    "agent.completed",
                    payload,
                )
            })
            .collect();
        let tmp = write_runtime_events(&events);
        let mut rollup = Rollup::default();
        read_runtime_test_log(tmp.path(), None, &mut rollup);
        assert_eq!(rollup.agents.successes, 1);
        assert_eq!(rollup.agents.failures, 2);
        assert_eq!(rollup.agents.unknown_outcomes, 6);
    }

    #[test]
    fn audit_worker_receipts_share_typed_and_legacy_outcome_rules() {
        let events = [
            serde_json::json!({ "event": "agent.completed", "details": { "worker_status": "failed", "success": true } }),
            serde_json::json!({ "event": "subagent.completed", "payload": { "status": "cancelled", "success": true } }),
            serde_json::json!({ "event": "subagent.completed", "details": { "status": "completed" } }),
            serde_json::json!({ "event": "agent.completed", "details": { "success": false } }),
            serde_json::json!({ "event": "agent.completed", "payload": { "success": true } }),
            serde_json::json!({ "event": "agent.completed", "details": { "worker_status": null, "success": true } }),
            serde_json::json!({ "event": "agent.completed", "details": {} }),
        ];
        let tmp = write_runtime_events(&events);
        let mut rollup = Rollup::default();
        read_audit_test_log(tmp.path(), None, &mut rollup);
        assert_eq!(rollup.agents.successes, 2);
        assert_eq!(rollup.agents.failures, 2);
        assert_eq!(rollup.agents.cancelled, 1);
        assert_eq!(rollup.agents.unknown_outcomes, 2);
        let json = serde_json::to_value(&rollup).unwrap();
        assert_eq!(json["agents"]["unknown_outcomes"], 2);
        assert_eq!(json["agents"]["cancelled"], 1);
    }

    // ── Duration parser ──

    #[test]
    fn parse_since_7d() {
        let cutoff = parse_since("7d").unwrap();
        let expected = Utc::now() - Duration::days(7);
        // Allow ±2s for test execution time.
        assert!((cutoff - expected).num_seconds().abs() < 2);
    }

    #[test]
    fn parse_since_24h() {
        let cutoff = parse_since("24h").unwrap();
        let expected = Utc::now() - Duration::hours(24);
        assert!((cutoff - expected).num_seconds().abs() < 2);
    }

    #[test]
    fn parse_since_30m() {
        let cutoff = parse_since("30m").unwrap();
        let expected = Utc::now() - Duration::minutes(30);
        assert!((cutoff - expected).num_seconds().abs() < 2);
    }

    #[test]
    fn parse_since_now_prefix() {
        // "now-2h" should strip "now-" and parse "2h".
        let cutoff = parse_since("now-2h").unwrap();
        let expected = Utc::now() - Duration::hours(2);
        assert!((cutoff - expected).num_seconds().abs() < 2);
    }

    #[test]
    fn parse_since_compound() {
        let cutoff = parse_since("2h30m").unwrap();
        let expected = Utc::now() - Duration::seconds(2 * 3600 + 30 * 60);
        assert!((cutoff - expected).num_seconds().abs() < 2);
    }

    #[test]
    fn parse_since_compound_days_hours() {
        let cutoff = parse_since("1d12h").unwrap();
        let expected = Utc::now() - Duration::seconds(36 * 3600);
        assert!((cutoff - expected).num_seconds().abs() < 2);
    }

    #[test]
    fn parse_since_error_on_invalid() {
        assert!(parse_since("xyz").is_err());
        assert!(parse_since("").is_err());
    }

    // ── fmt_num ──

    #[test]
    fn fmt_num_zero() {
        assert_eq!(fmt_num(0), "0");
    }

    #[test]
    fn fmt_num_thousands() {
        assert_eq!(fmt_num(1_000), "1,000");
        assert_eq!(fmt_num(12_453), "12,453");
        assert_eq!(fmt_num(1_000_000), "1,000,000");
    }

    // ── Rollup from audit log ──

    fn make_audit_line(event: &str, tool: &str, ts: &str) -> String {
        format!(
            r#"{{"details":{{"mode":"YOLO","session_id":null,"tool_name":"{tool}"}},"event":"{event}","ts":"{ts}"}}"#
        )
    }

    #[test]
    fn audit_log_empty_file() {
        let mut rollup = Rollup::default();
        // Non-existent path — should not panic, rollup stays empty.
        read_audit_test_log(Path::new("/nonexistent/audit.log"), None, &mut rollup);
        assert_eq!(rollup.total_lines, 0);
    }

    #[test]
    fn audit_log_parses_auto_approve() {
        use std::io::Write;
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        let line1 = make_audit_line(
            "tool.approval.auto_approve",
            "exec_shell",
            "2026-04-01T10:00:00+00:00",
        );
        let line2 = make_audit_line(
            "tool.approval.auto_approve",
            "read_file",
            "2026-04-02T10:00:00+00:00",
        );
        writeln!(tmp, "{line1}").unwrap();
        writeln!(tmp, "{line2}").unwrap();

        let mut rollup = Rollup::default();
        read_audit_test_log(tmp.path(), None, &mut rollup);

        assert_eq!(rollup.parsed_lines, 2);
        assert_eq!(rollup.tools["exec_shell"].calls, 1);
        assert_eq!(rollup.tools["exec_shell"].auto_approved, 1);
        assert_eq!(rollup.tools["read_file"].calls, 1);
    }

    #[test]
    fn audit_log_skips_malformed_lines() {
        use std::io::Write;
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        writeln!(tmp, "not json at all").unwrap();
        writeln!(
            tmp,
            r#"{{"event":"credential.save","ts":"2026-04-01T10:00:00+00:00"}}"#
        )
        .unwrap();

        let mut rollup = Rollup::default();
        read_audit_test_log(tmp.path(), None, &mut rollup);

        // 2 lines total, 1 malformed skipped, 1 parsed.
        assert_eq!(rollup.total_lines, 2);
        assert_eq!(rollup.parsed_lines, 1);
        assert_eq!(rollup.credentials.saves, 1);
    }

    #[test]
    fn audit_log_since_filter() {
        use std::io::Write;
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        let line_old = make_audit_line(
            "tool.approval.auto_approve",
            "exec_shell",
            "2025-01-01T00:00:00+00:00",
        );
        let line_new = make_audit_line(
            "tool.approval.auto_approve",
            "read_file",
            "2026-04-01T00:00:00+00:00",
        );
        writeln!(tmp, "{line_old}").unwrap();
        writeln!(tmp, "{line_new}").unwrap();

        let cutoff: DateTime<Utc> = "2026-01-01T00:00:00Z".parse().unwrap();
        let mut rollup = Rollup::default();
        read_audit_test_log(tmp.path(), Some(cutoff), &mut rollup);

        // Only the newer line should be counted.
        assert_eq!(rollup.parsed_lines, 1);
        assert!(!rollup.tools.contains_key("exec_shell"));
        assert_eq!(rollup.tools["read_file"].calls, 1);
    }

    #[test]
    fn total_tool_calls_sums_across_tools() {
        let mut rollup = Rollup::default();
        rollup.tool_mut("read_file").calls = 4_012;
        rollup.tool_mut("exec_shell").calls = 1_118;
        assert_eq!(rollup.total_tool_calls(), 5_130);
    }

    // ── Runtime request and provider-usage receipts ──

    #[test]
    fn runtime_receipts_separate_terminal_requests_from_per_request_usage() {
        let timestamp = "2026-09-08T10:00:00Z";
        let terminal = runtime_event(
            2,
            timestamp,
            "thread-a",
            Some("turn-a"),
            "turn.completed",
            serde_json::json!({
                "turn": {
                    "usage": { "input_tokens": 10_000, "output_tokens": 9_000 },
                    "modelRequestDiagnostics": {
                        "modelRequestsStarted": 2,
                        "transparentStreamRetries": 1,
                        "streamResumes": 1,
                    },
                },
            }),
        );
        let usage_one = runtime_event(
            3,
            timestamp,
            "thread-a",
            Some("turn-a"),
            "turn.usage",
            serde_json::json!({ "usage": { "input_tokens": 7, "output_tokens": 2 } }),
        );
        let usage_two = runtime_event(
            4,
            timestamp,
            "thread-a",
            Some("turn-a"),
            "turn.usage",
            serde_json::json!({ "usage": { "input_tokens": 11, "output_tokens": 3 } }),
        );
        let duplicate_terminal = runtime_event(
            5,
            timestamp,
            "thread-a",
            Some("turn-a"),
            "turn.completed",
            serde_json::json!({
                "turn": {
                    "modelRequestDiagnostics": {
                        "modelRequestsStarted": 99,
                        "transparentStreamRetries": 99,
                        "streamResumes": 99,
                    },
                },
            }),
        );
        let legacy_terminal = runtime_event(
            6,
            timestamp,
            "thread-a",
            Some("turn-b"),
            "turn.completed",
            serde_json::json!({ "turn": { "usage": { "input_tokens": 50, "output_tokens": 5 } } }),
        );
        let status = runtime_event(
            7,
            timestamp,
            "thread-a",
            Some("turn-a"),
            "item.completed",
            serde_json::json!({ "item": { "kind": "status" } }),
        );
        let tmp = write_runtime_events(&[
            terminal,
            usage_one,
            usage_two,
            duplicate_terminal,
            legacy_terminal,
            status,
        ]);
        let mut rollup = Rollup::default();
        read_runtime_test_log(tmp.path(), None, &mut rollup);

        let runtime = &rollup.runtime_requests;
        assert_eq!(runtime.terminal_turn_receipts, 2);
        assert_eq!(runtime.diagnostics_turn_receipts, 1);
        assert_eq!(runtime.diagnostics_unavailable_turn_receipts, 1);
        assert_eq!(runtime.duplicate_terminal_receipts_skipped, 1);
        assert_eq!(runtime.model_requests_started, 2);
        assert_eq!(runtime.transparent_stream_retries, 1);
        assert_eq!(runtime.stream_resumes, 1);
        assert_eq!(runtime.provider_usage_receipts, 2);
        assert_eq!(runtime.provider_reported_input_tokens, 18);
        assert_eq!(runtime.provider_reported_output_tokens, 5);
        assert_ne!(runtime.provider_reported_input_tokens, 10_018);
        assert_eq!(runtime.model_requests_started, 2, "status is not a request");
    }

    #[test]
    fn runtime_usage_receipts_deduplicate_by_runtime_event_identity() {
        let usage = runtime_event(
            20,
            "2026-09-08T10:00:00Z",
            "thread-a",
            Some("turn-a"),
            "turn.usage",
            serde_json::json!({ "usage": { "input_tokens": 7, "output_tokens": 2 } }),
        );
        let tmp = write_runtime_events(&[usage.clone(), usage]);
        let mut rollup = Rollup::default();
        read_runtime_test_log(tmp.path(), None, &mut rollup);

        let runtime = &rollup.runtime_requests;
        assert_eq!(runtime.provider_usage_receipts, 1);
        assert_eq!(runtime.provider_reported_input_tokens, 7);
        assert_eq!(runtime.provider_reported_output_tokens, 2);
        assert_eq!(runtime.duplicate_provider_usage_receipts_skipped, 1);
    }

    #[test]
    fn runtime_receipt_coverage_marks_unidentified_or_incomplete_old_records_unknown() {
        let terminal_without_identity = serde_json::json!({
            "timestamp": "2026-09-08T10:00:00Z",
            "event": "turn.completed",
            "payload": {
                "turn": {
                    "modelRequestDiagnostics": {
                        "modelRequestsStarted": 3,
                        "transparentStreamRetries": 1,
                        "streamResumes": 2,
                    },
                },
            },
        });
        let usage_without_identity = serde_json::json!({
            "timestamp": "2026-09-08T10:00:00Z",
            "event": "turn.usage",
            "payload": { "usage": { "input_tokens": 9, "output_tokens": 4 } },
        });
        let incomplete_diagnostics = runtime_event(
            30,
            "2026-09-08T10:00:00Z",
            "thread-a",
            Some("turn-b"),
            "turn.completed",
            serde_json::json!({
                "turn": { "modelRequestDiagnostics": { "modelRequestsStarted": 3 } },
            }),
        );
        let incomplete_usage = runtime_event(
            31,
            "2026-09-08T10:00:00Z",
            "thread-a",
            Some("turn-b"),
            "turn.usage",
            serde_json::json!({ "usage": { "input_tokens": 9 } }),
        );
        let tmp = write_runtime_events(&[
            terminal_without_identity,
            usage_without_identity,
            incomplete_diagnostics,
            incomplete_usage,
        ]);
        let mut rollup = Rollup::default();
        read_runtime_test_log(tmp.path(), None, &mut rollup);

        let runtime = &rollup.runtime_requests;
        assert_eq!(runtime.terminal_receipts_without_identity, 1);
        assert_eq!(runtime.terminal_turn_receipts, 1);
        assert_eq!(runtime.diagnostics_incomplete_turn_receipts, 1);
        assert_eq!(runtime.model_requests_started, 0);
        assert_eq!(runtime.provider_usage_receipts_without_identity, 1);
        assert_eq!(runtime.provider_usage_receipts_incomplete, 1);
        assert_eq!(runtime.provider_usage_receipts, 0);
        assert_eq!(runtime.provider_reported_input_tokens, 0);
    }

    #[test]
    fn runtime_receipts_respect_since_cutoff_without_crossing_snapshot_boundaries() {
        let old_terminal = runtime_event(
            40,
            "2026-09-01T10:00:00Z",
            "thread-a",
            Some("turn-old"),
            "turn.completed",
            serde_json::json!({
                "turn": { "modelRequestDiagnostics": {
                    "modelRequestsStarted": 4,
                    "transparentStreamRetries": 1,
                    "streamResumes": 2,
                } },
            }),
        );
        let old_usage = runtime_event(
            41,
            "2026-09-01T10:00:00Z",
            "thread-a",
            Some("turn-old"),
            "turn.usage",
            serde_json::json!({ "usage": { "input_tokens": 40, "output_tokens": 4 } }),
        );
        let new_terminal = runtime_event(
            42,
            "2026-09-08T10:00:00Z",
            "thread-a",
            Some("turn-new"),
            "turn.completed",
            serde_json::json!({
                "turn": { "modelRequestDiagnostics": {
                    "modelRequestsStarted": 1,
                    "transparentStreamRetries": 0,
                    "streamResumes": 0,
                } },
            }),
        );
        let new_usage = runtime_event(
            43,
            "2026-09-08T10:00:00Z",
            "thread-a",
            Some("turn-new"),
            "turn.usage",
            serde_json::json!({ "usage": { "input_tokens": 10, "output_tokens": 1 } }),
        );
        let tmp = write_runtime_events(&[old_terminal, old_usage, new_terminal, new_usage]);
        let mut rollup = Rollup::default();
        read_runtime_test_log(
            tmp.path(),
            Some("2026-09-08T00:00:00Z".parse().unwrap()),
            &mut rollup,
        );

        let runtime = &rollup.runtime_requests;
        assert_eq!(runtime.terminal_turn_receipts, 1);
        assert_eq!(runtime.model_requests_started, 1);
        assert_eq!(runtime.provider_usage_receipts, 1);
        assert_eq!(runtime.provider_reported_input_tokens, 10);
        assert_eq!(runtime.provider_reported_output_tokens, 1);
    }

    // ── State-root resolution ──
    //
    // These pin *which* files the rollup reads. Before the fix the reader
    // resolved `$HOME/.deepseek`, which nothing has written since the v0.8.44
    // rename, so `codewhale metrics` printed an all-zero rollup as truth.

    /// Isolate the ambient home so the resolver sees a clean, empty install.
    ///
    /// The returned guards must stay bound for the life of the test: dropping
    /// them restores the previous environment. Destructure the tuple so the
    /// bindings drop in reverse order — the environment is restored *before*
    /// the lock is released, or a concurrent env-mutating test sees a torn HOME.
    fn isolated_home() -> (
        tempfile::TempDir,
        std::sync::MutexGuard<'static, ()>,
        Vec<crate::tests::ScopedEnvVar>,
    ) {
        let guard = crate::tests::env_lock();
        let home = tempfile::TempDir::new().expect("tempdir");
        let vars = vec![
            crate::tests::ScopedEnvVar::set("HOME", &home.path().to_string_lossy()),
            crate::tests::ScopedEnvVar::set("USERPROFILE", &home.path().to_string_lossy()),
            crate::tests::ScopedEnvVar::remove("CODEWHALE_HOME"),
            crate::tests::ScopedEnvVar::remove("DEEPSEEK_HOME"),
        ];
        (home, guard, vars)
    }

    #[test]
    fn default_audit_history_includes_both_roots_without_requiring_existing_files() {
        let (home, _lock, _env) = isolated_home();
        assert_eq!(
            resolve_audit_roots().expect("resolves"),
            vec![
                home.path().join(".codewhale"),
                home.path().join(".deepseek")
            ],
        );
    }

    #[test]
    fn copied_audit_history_keeps_unique_legacy_and_rotated_records() {
        let dir = tempfile::TempDir::new().expect("tempdir");
        let primary = dir.path().join("primary");
        let legacy = dir.path().join("legacy");
        std::fs::create_dir_all(&primary).unwrap();
        std::fs::create_dir_all(&legacy).unwrap();
        let shared = r#"{"ts":"2026-09-01T00:00:00Z","event":"credential.save","details":{}}"#;
        let old = r#"{"ts":"2026-08-01T00:00:00Z","event":"credential.clear","details":{}}"#;
        let new = r#"{"ts":"2026-09-02T00:00:00Z","event":"credential.save","details":{}}"#;
        std::fs::write(primary.join("audit.log.1"), format!("{shared}\n{shared}\n")).unwrap();
        std::fs::write(primary.join("audit.log"), format!("{new}\nmalformed\n")).unwrap();
        std::fs::write(
            legacy.join("audit.log"),
            format!("{shared}\n{shared}\n{shared}\n"),
        )
        .unwrap();
        std::fs::write(legacy.join("audit.log.1"), format!("{old}\n")).unwrap();
        let roots = [primary, legacy];
        let before: Vec<_> = roots
            .iter()
            .flat_map(|root| {
                ["audit.log.1", "audit.log"].map(|name| {
                    let path = root.join(name);
                    (path.clone(), std::fs::read(path).unwrap())
                })
            })
            .collect();
        let mut rollup = Rollup::default();
        read_audit_history(&roots, None, &mut rollup);
        assert_eq!(
            rollup.credentials.saves, 4,
            "maximum occurrence count across copied roots"
        );
        assert_eq!(
            rollup.credentials.clears, 1,
            "unique old rotation is retained"
        );
        assert_eq!(rollup.parsed_lines, 5);
        for (path, bytes) in before {
            assert_eq!(
                std::fs::read(path).unwrap(),
                bytes,
                "source history is read-only"
            );
        }
        let mut recent = Rollup::default();
        read_audit_history(
            &roots,
            Some("2026-09-01T00:00:00Z".parse().unwrap()),
            &mut recent,
        );
        assert_eq!(recent.credentials.saves, 4);
        assert_eq!(recent.credentials.clears, 0);
    }

    #[test]
    fn an_explicit_codewhale_home_is_an_audit_isolation_boundary() {
        let (home, _lock, _env) = isolated_home();
        let legacy = home.path().join(".deepseek");
        std::fs::create_dir_all(&legacy).unwrap();
        std::fs::write(legacy.join("audit.log"), r#"{"event":"credential.save"}"#).unwrap();
        let explicit = tempfile::TempDir::new().unwrap();
        let _pin =
            crate::tests::ScopedEnvVar::set("CODEWHALE_HOME", &explicit.path().to_string_lossy());
        let roots = resolve_audit_roots().unwrap();
        assert_eq!(roots, vec![explicit.path().to_path_buf()]);
        let mut rollup = Rollup::default();
        read_audit_history(&roots, None, &mut rollup);
        assert_eq!(rollup.parsed_lines, 0);
    }

    #[test]
    fn the_legacy_deepseek_home_variable_is_no_longer_honoured() {
        let (home, _lock, _env) = isolated_home();
        let stale = tempfile::TempDir::new().unwrap();
        let _stale =
            crate::tests::ScopedEnvVar::set("DEEPSEEK_HOME", &stale.path().to_string_lossy());
        assert_eq!(
            resolve_audit_roots().unwrap(),
            vec![
                home.path().join(".codewhale"),
                home.path().join(".deepseek")
            ],
        );
    }
}
