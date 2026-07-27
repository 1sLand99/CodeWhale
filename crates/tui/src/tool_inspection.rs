//! Truthful, bounded inspection of a prepared model-client request's tool field.
//!
//! Capture happens at the request-construction seam and retains only a bounded
//! projection. It never joins against mutable registry, MCP, provider, or
//! approval state, and it never claims that the prepared request was delivered.

use std::io::{self, Write};

use serde::Serialize;
use serde_json::Value;

use crate::models::Tool;

const MAX_RENDERED_TOOLS: usize = 32;
const MAX_NAME_CHARS: usize = 256;
const MAX_DESCRIPTION_CHARS: usize = 512;
const MAX_SCHEMA_BYTES: usize = 2_048;
const MAX_AUXILIARY_CHARS: usize = 512;
const MAX_ALLOWED_CALLERS: usize = 16;
const MAX_ALLOWED_CALLER_CHARS: usize = 128;
const MAX_PAYLOAD_MEASUREMENT_BYTES: usize = 1_048_576;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BoundedString {
    pub value: String,
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Evidence<T> {
    Known { value: T },
    Unknown { reason: String },
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BoundedList {
    pub count: usize,
    pub rendered: Vec<BoundedString>,
    pub omitted: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CountOnly {
    pub count: usize,
    pub values: &'static str,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ToolProjection {
    pub ordinal: usize,
    pub name: BoundedString,
    pub tool_type: Evidence<BoundedString>,
    pub description: BoundedString,
    pub input_schema_json: BoundedString,
    pub allowed_callers: Evidence<BoundedList>,
    pub defer_loading: Evidence<bool>,
    pub input_examples: Evidence<CountOnly>,
    pub strict: Evidence<bool>,
    pub cache_control_type: Evidence<BoundedString>,
}

/// Bounded evidence from a prepared model-client request.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ToolInspectionSnapshot {
    pub schema_version: u32,
    pub capture_source: &'static str,
    pub delivery_status: &'static str,
    pub turn_id: BoundedString,
    pub step: u32,
    pub tools_field_present: bool,
    pub tool_count: usize,
    pub rendered_tool_count: usize,
    pub omitted_tool_count: usize,
    pub payload_json_bytes: Option<usize>,
    pub payload_measurement_status: String,
    /// The active-tool-catalog digest, computed by the *same* function the
    /// request manifest uses for `active_tool_catalog_sha256`. Absent only when
    /// the request carried no tools field at all. Covers tool name,
    /// description, and canonical input schema — not transport-only fields —
    /// exactly as the manifest does.
    pub active_tool_catalog_sha256: Option<String>,
    pub unavailable_from_tool_schema: [&'static str; 6],
    pub tools: Vec<ToolProjection>,
}

impl ToolInspectionSnapshot {
    #[must_use]
    pub fn from_prepared_request(turn_id: &str, step: u32, tools: Option<&[Tool]>) -> Self {
        let tool_count = tools.map_or(0, <[Tool]>::len);
        let projected = tools
            .unwrap_or_default()
            .iter()
            .take(MAX_RENDERED_TOOLS)
            .enumerate()
            .map(|(index, tool)| project_tool(index, tool))
            .collect::<Vec<_>>();
        let (payload_json_bytes, payload_measurement_status) = measure_payload(tools);
        Self {
            schema_version: 1,
            capture_source: "prepared model-client request",
            delivery_status: "unknown (capture does not prove provider delivery)",
            turn_id: bounded_chars(turn_id, MAX_AUXILIARY_CHARS),
            step,
            tools_field_present: tools.is_some(),
            tool_count,
            rendered_tool_count: projected.len(),
            omitted_tool_count: tool_count.saturating_sub(projected.len()),
            payload_json_bytes,
            payload_measurement_status,
            active_tool_catalog_sha256: tools
                .map(crate::core::engine::preview::active_tool_catalog_sha256),
            unavailable_from_tool_schema: [
                "provider",
                "model",
                "approval",
                "provenance",
                "capabilities",
                "provider_wire_payload",
            ],
            tools: projected,
        }
    }

    #[must_use]
    pub fn render_text(&self) -> String {
        let mut out = String::new();
        out.push_str("Prepared Model-Client Tool Request (read-only)\n");
        out.push_str(&format!("Capture source: {}\n", self.capture_source));
        out.push_str(&format!("Delivery: {}\n", self.delivery_status));
        out.push_str(&format!(
            "Turn: {}\nTurn truncated: {}\n",
            json_string(&self.turn_id.value),
            yes_no(self.turn_id.truncated)
        ));
        out.push_str(&format!("Step: {}\n", self.step));
        out.push_str(&format!(
            "Tools field: {}\nTool count: {}\n",
            if self.tools_field_present {
                "present"
            } else {
                "absent"
            },
            self.tool_count
        ));
        out.push_str(&format!(
            "Rendered tools: {}; omitted by render bound: {}\n",
            self.rendered_tool_count, self.omitted_tool_count
        ));
        out.push_str(&format!(
            "Model-client payload measurement: {}\n",
            self.payload_measurement_status
        ));
        out.push_str(&format_optional_usize(
            "Model-client tool JSON bytes",
            self.payload_json_bytes,
        ));
        out.push_str(&format_optional_string(
            "Active tool catalog digest (same digest as the request manifest)",
            self.active_tool_catalog_sha256.as_deref(),
        ));
        out.push_str(
            "Provider-wire tool payload: unavailable (the provider adapter may transform or omit model-client fields)\n",
        );
        out.push_str(
            "Provider, model, approval, provenance, and capability metadata: unavailable (not carried by the request tool schema)\n",
        );

        for tool in &self.tools {
            out.push_str(&format!(
                "\n{}. {}\n",
                tool.ordinal,
                json_string(&tool.name.value)
            ));
            out.push_str(&format!(
                "   name truncated: {}\n",
                yes_no(tool.name.truncated)
            ));
            render_bounded_evidence(&mut out, "type", &tool.tool_type);
            out.push_str(&format!(
                "   description: {}\n   description truncated: {}\n",
                json_string(&tool.description.value),
                yes_no(tool.description.truncated)
            ));
            out.push_str(&format!(
                "   input schema JSON: {}\n   input schema truncated: {}\n",
                tool.input_schema_json.value,
                yes_no(tool.input_schema_json.truncated)
            ));
            match &tool.allowed_callers {
                Evidence::Known { value } => {
                    let rendered = value
                        .rendered
                        .iter()
                        .map(|entry| entry.value.as_str())
                        .collect::<Vec<_>>();
                    out.push_str(&format!(
                        "   allowed callers: {}\n   allowed callers count: {}\n   allowed callers omitted: {}\n   allowed callers truncated: {}\n",
                        serde_json::to_string(&rendered).unwrap_or_else(|_| "unavailable".to_string()),
                        value.count,
                        value.omitted,
                        yes_no(value.rendered.iter().any(|entry| entry.truncated))
                    ));
                }
                Evidence::Unknown { reason } => {
                    out.push_str(&format!("   allowed callers: unknown ({reason})\n"));
                }
            }
            render_bool_evidence(&mut out, "deferred loading", &tool.defer_loading);
            render_bool_evidence(&mut out, "strict", &tool.strict);
            match &tool.input_examples {
                Evidence::Known { value } => out.push_str(&format!(
                    "   input examples: present ({} value(s), {})\n",
                    value.count, value.values
                )),
                Evidence::Unknown { reason } => {
                    out.push_str(&format!("   input examples: unknown ({reason})\n"));
                }
            }
            render_bounded_evidence(&mut out, "cache control type", &tool.cache_control_type);
        }
        out
    }

    pub fn render_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

fn project_tool(index: usize, tool: &Tool) -> ToolProjection {
    ToolProjection {
        ordinal: index + 1,
        name: bounded_chars(&tool.name, MAX_NAME_CHARS),
        tool_type: optional_bounded(tool.tool_type.as_deref()),
        description: bounded_chars(&tool.description, MAX_DESCRIPTION_CHARS),
        input_schema_json: bounded_json(&tool.input_schema, MAX_SCHEMA_BYTES),
        allowed_callers: tool.allowed_callers.as_ref().map_or_else(
            || unknown("request field absent"),
            |values| {
                let rendered = values
                    .iter()
                    .take(MAX_ALLOWED_CALLERS)
                    .map(|value| bounded_chars(value, MAX_ALLOWED_CALLER_CHARS))
                    .collect::<Vec<_>>();
                Evidence::Known {
                    value: BoundedList {
                        count: values.len(),
                        omitted: values.len().saturating_sub(rendered.len()),
                        rendered,
                    },
                }
            },
        ),
        defer_loading: optional_copy(tool.defer_loading.as_ref()),
        input_examples: tool.input_examples.as_ref().map_or_else(
            || unknown("request field absent"),
            |values| Evidence::Known {
                value: CountOnly {
                    count: values.len(),
                    values: "values omitted from bounded projection",
                },
            },
        ),
        strict: optional_copy(tool.strict.as_ref()),
        cache_control_type: optional_bounded(
            tool.cache_control
                .as_ref()
                .map(|value| value.cache_type.as_str()),
        ),
    }
}

/// Byte accounting only. The digest is deliberately *not* computed here: it is
/// the request path's [`crate::core::engine::preview::active_tool_catalog_sha256`],
/// so this projection never defines a second catalog hash.
fn measure_payload(tools: Option<&[Tool]>) -> (Option<usize>, String) {
    let Some(tools) = tools else {
        return (None, "unavailable (tools field absent)".to_string());
    };
    let mut writer = BoundedWriter::new(MAX_PAYLOAD_MEASUREMENT_BYTES);
    match serde_json::to_writer(&mut writer, tools) {
        Ok(()) => (
            Some(writer.bytes.len()),
            "exact (within 1048576-byte measurement bound)".to_string(),
        ),
        Err(_) if writer.exceeded => (
            None,
            "unavailable (payload exceeds 1048576-byte measurement bound)".to_string(),
        ),
        Err(_) => (None, "unavailable (serialization failed)".to_string()),
    }
}

fn bounded_json(value: &Value, limit: usize) -> BoundedString {
    let mut writer = BoundedWriter::new(limit);
    let result = serde_json::to_writer(&mut writer, value);
    BoundedString {
        value: String::from_utf8_lossy(&writer.bytes).into_owned(),
        truncated: result.is_err() && writer.exceeded,
    }
}

struct BoundedWriter {
    bytes: Vec<u8>,
    limit: usize,
    exceeded: bool,
}

impl BoundedWriter {
    fn new(limit: usize) -> Self {
        Self {
            bytes: Vec::with_capacity(limit.min(8_192)),
            limit,
            exceeded: false,
        }
    }
}

impl Write for BoundedWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        let remaining = self.limit.saturating_sub(self.bytes.len());
        let accepted = buffer.len().min(remaining);
        self.bytes.extend_from_slice(&buffer[..accepted]);
        if accepted < buffer.len() {
            self.exceeded = true;
            return Err(io::Error::other("inspection bound exceeded"));
        }
        Ok(accepted)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn bounded_chars(value: &str, limit: usize) -> BoundedString {
    let mut chars = value.chars();
    let value = chars.by_ref().take(limit).collect::<String>();
    BoundedString {
        value,
        truncated: chars.next().is_some(),
    }
}

fn optional_bounded(value: Option<&str>) -> Evidence<BoundedString> {
    value.map_or_else(
        || unknown("request field absent"),
        |value| Evidence::Known {
            value: bounded_chars(value, MAX_AUXILIARY_CHARS),
        },
    )
}

fn optional_copy<T: Copy>(value: Option<&T>) -> Evidence<T> {
    value.map_or_else(
        || unknown("request field absent"),
        |value| Evidence::Known { value: *value },
    )
}

fn unknown<T>(reason: &str) -> Evidence<T> {
    Evidence::Unknown {
        reason: reason.to_string(),
    }
}

fn render_bounded_evidence(out: &mut String, label: &str, evidence: &Evidence<BoundedString>) {
    match evidence {
        Evidence::Known { value } => out.push_str(&format!(
            "   {label}: {}\n   {label} truncated: {}\n",
            json_string(&value.value),
            yes_no(value.truncated)
        )),
        Evidence::Unknown { reason } => {
            out.push_str(&format!("   {label}: unknown ({reason})\n"));
        }
    }
}

fn render_bool_evidence(out: &mut String, label: &str, evidence: &Evidence<bool>) {
    match evidence {
        Evidence::Known { value } => out.push_str(&format!("   {label}: {value}\n")),
        Evidence::Unknown { reason } => {
            out.push_str(&format!("   {label}: unknown ({reason})\n"));
        }
    }
}

fn json_string(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "\"unavailable\"".to_string())
}

fn format_optional_usize(label: &str, value: Option<usize>) -> String {
    value.map_or_else(
        || format!("{label}: unavailable\n"),
        |value| format!("{label}: {value}\n"),
    )
}

fn format_optional_string(label: &str, value: Option<&str>) -> String {
    value.map_or_else(
        || format!("{label}: unavailable\n"),
        |value| format!("{label}: {value}\n"),
    )
}

const fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn tool(name: &str) -> Tool {
        Tool {
            tool_type: Some("function".to_string()),
            name: name.to_string(),
            description: "Read a file".to_string(),
            input_schema: json!({"type": "object"}),
            allowed_callers: None,
            defer_loading: Some(false),
            input_examples: None,
            strict: Some(true),
            cache_control: None,
        }
    }

    #[test]
    fn absent_field_stays_distinct_from_present_empty_array() {
        let absent = ToolInspectionSnapshot::from_prepared_request("turn", 1, None);
        let empty = ToolInspectionSnapshot::from_prepared_request("turn", 1, Some(&[]));
        assert!(!absent.tools_field_present);
        assert_eq!(absent.payload_json_bytes, None);
        assert!(absent.active_tool_catalog_sha256.is_none());
        assert!(empty.tools_field_present);
        assert_eq!(empty.payload_json_bytes, Some(2));
        assert!(empty.active_tool_catalog_sha256.is_some());
    }

    #[test]
    fn catalog_digest_is_the_request_manifest_digest_not_a_second_definition() {
        let tools = vec![tool("read_file"), tool("write_file")];
        let snapshot = ToolInspectionSnapshot::from_prepared_request("turn", 1, Some(&tools));

        // Same prepared request, same accounting object: the value the request
        // manifest publishes as `active_tool_catalog_sha256`.
        assert_eq!(
            snapshot.active_tool_catalog_sha256.as_deref(),
            Some(crate::core::engine::preview::active_tool_catalog_sha256(&tools).as_str()),
        );

        // And it is a catalog digest, not an incidental byte hash: reordering
        // the same tools changes it.
        let reordered = vec![tools[1].clone(), tools[0].clone()];
        let reordered = ToolInspectionSnapshot::from_prepared_request("turn", 1, Some(&reordered));
        assert_ne!(
            snapshot.active_tool_catalog_sha256,
            reordered.active_tool_catalog_sha256
        );
    }

    #[test]
    fn projection_preserves_known_false_and_marks_unknown() {
        let snapshot =
            ToolInspectionSnapshot::from_prepared_request("turn", 3, Some(&[tool("read_file")]));
        let text = snapshot.render_text();
        assert!(text.contains("deferred loading: false"), "{text}");
        assert!(text.contains("strict: true"), "{text}");
        assert!(text.contains("allowed callers: unknown (request field absent)"));
        assert!(text.contains("Delivery: unknown"));
        assert!(text.contains("Provider-wire tool payload: unavailable"));
    }

    #[test]
    fn capture_and_rendering_are_bounded_with_explicit_receipts() {
        let mut tools = (0..40)
            .map(|index| {
                let mut value = tool(&format!("tool_{index}"));
                value.description = "x".repeat(MAX_DESCRIPTION_CHARS + 10);
                value.input_schema = json!({"large": "y".repeat(MAX_SCHEMA_BYTES * 600)});
                value
            })
            .collect::<Vec<_>>();
        tools[0].allowed_callers = Some(
            (0..20)
                .map(|caller| format!("caller-{caller}-{}", "z".repeat(200)))
                .collect(),
        );
        let snapshot = ToolInspectionSnapshot::from_prepared_request(
            &"t".repeat(MAX_AUXILIARY_CHARS + 1),
            1,
            Some(&tools),
        );
        assert_eq!(snapshot.rendered_tool_count, MAX_RENDERED_TOOLS);
        assert_eq!(snapshot.omitted_tool_count, 8);
        assert!(snapshot.turn_id.truncated);
        assert!(snapshot.tools[0].description.truncated);
        assert!(snapshot.tools[0].input_schema_json.truncated);
        assert_eq!(snapshot.payload_json_bytes, None);
        assert!(snapshot.payload_measurement_status.contains("exceeds"));
        // The catalog digest is fixed-width, so it survives the byte bound.
        assert!(snapshot.active_tool_catalog_sha256.is_some());
        let json = snapshot.render_json().expect("bounded JSON");
        assert!(
            json.len() < 131_072,
            "projection grew to {} bytes",
            json.len()
        );
    }
}
