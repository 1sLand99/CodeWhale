//! MCP Registry sync tool.
//!
//! Provides `McpSyncRegistry` — fetches the MCP Registry index,
//! filters for stdio-type servers, caches locally, and returns a summary.
//! The cache is deliberately simple: every sync pulls the complete
//! paginated listing and atomically replaces the previous `mcp-index.json`
//! (no TTL freshness bookkeeping, no merge, no eviction). The on-disk file
//! is the launch-metadata store for `start_registry_mcp_server`, and a
//! failed sync leaves the previous snapshot untouched.
//!
//! Upstream contract (MCP Registry, preview — breaking changes possible):
//!   * List operation `GET /v0.1/servers` (cursor / limit / search / version
//!     / include_deleted params):
//!     <https://registry.modelcontextprotocol.io/docs#/operations/list-servers-v0.1>
//!     OpenAPI source: <https://github.com/modelcontextprotocol/registry/blob/main/docs/reference/api/openapi.yaml>
//!   * Aggregator integration guide (pagination format, server status
//!     lifecycle):
//!     <https://github.com/modelcontextprotocol/registry/blob/main/docs/modelcontextprotocol-io/registry-aggregators.mdx>

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::sync::Mutex as AsyncMutex;

use crate::mcp::McpPool;
use crate::tools::spec::{
    ApprovalRequirement, ToolCapability, ToolContext, ToolError, ToolResult, ToolSpec,
};
use crate::utils::write_atomic;

// === Registry API response types ===

#[derive(Deserialize)]
struct RegistryResponse {
    servers: Vec<RegistryServerEntry>,
    metadata: Option<RegistryMetadata>,
}

#[derive(Deserialize)]
struct RegistryServerEntry {
    server: RegistryServer,
    // Registry-managed metadata. Carries the lifecycle `status` under the
    // official extension key (see `RegistryOfficialMeta`); the
    // publisher-provided subkey is deliberately not declared.
    #[serde(rename = "_meta", default)]
    meta: Option<RegistryResponseMeta>,
}

impl RegistryServerEntry {
    /// Lifecycle status reported by the official registry extension.
    /// `"active"` (or an absent extension) keeps the entry; `"deprecated"`
    /// and `"deleted"` retire it — the aggregator guide recommends dropping
    /// `deleted` entries (moderation takedowns: spam/malware/illegal) from
    /// downstream indexes, and we treat `deprecated` the same so the model
    /// is only offered servers the publisher still stands behind.
    /// <https://github.com/modelcontextprotocol/registry/blob/main/docs/modelcontextprotocol-io/registry-aggregators.mdx>
    fn lifecycle_status(&self) -> Option<&str> {
        self.meta
            .as_ref()
            .and_then(|m| m.official.as_ref())
            .and_then(|o| o.status.as_deref())
    }
}

#[derive(Deserialize)]
struct RegistryResponseMeta {
    #[serde(rename = "io.modelcontextprotocol.registry/official", default)]
    official: Option<RegistryOfficialMeta>,
}

/// `status` is required upstream (enum `active | deprecated | deleted`);
/// kept Optional here so a missing extension never fails a page parse.
#[derive(Deserialize)]
struct RegistryOfficialMeta {
    #[serde(default)]
    status: Option<String>,
}

#[derive(Deserialize)]
struct RegistryServer {
    name: String,
    description: String,
    // `title`, `version`, `repository` are deliberately not declared —
    // the cache no longer carries them (see `McpRegistryServerEntry`)
    // and serde silently drops any extra fields, so we don't pay to
    // validate or store data we'd immediately throw away. All three are
    // optional in the upstream 2025-12 schema.
    #[serde(default)]
    packages: Option<Vec<RegistryPackage>>,
}

#[derive(Deserialize)]
struct RegistryPackage {
    #[serde(rename = "registryType")]
    registry_type: String,
    identifier: String,
    // The upstream OCI entries (e.g. docker.io/foo/bar:1.2.3) omit the
    // top-level `version` field because the tag is the version. Mirror that
    // — Optional, with a fallback that parses the trailing `:tag` from the
    // identifier when missing.
    #[serde(default)]
    version: Option<String>,
    // The upstream 2025-12 schema dropped `runtimeHint` for nearly every
    // entry (35/36 in the first page omit it; the runner is implied by
    // `registryType`). Keep it Optional and fall back to a registry-type
    // table when absent.
    #[serde(rename = "runtimeHint", default)]
    runtime_hint: Option<String>,
    // The upstream schema now models `transport` as an object:
    // `{"type": "stdio"}`. Older docs showed a bare string. Accept both
    // so a future flip-back doesn't break us.
    #[serde(deserialize_with = "deserialize_transport", default)]
    transport: Option<String>,
    #[serde(default)]
    #[serde(rename = "packageArguments")]
    package_arguments: Vec<RegistryArg>,
    #[serde(
        rename = "runtimeArguments",
        deserialize_with = "deserialize_runtime_arguments",
        default
    )]
    runtime_arguments: Vec<String>,
    /// Registry-provided environment requirements are intentionally kept
    /// transient. Runtime-discovered servers have no configuration channel
    /// for secrets/API keys, so any package declaring environment variables
    /// is ineligible and never reaches the on-disk cache.
    #[serde(rename = "environmentVariables", default)]
    environment_variables: Value,
}

impl RegistryPackage {
    fn declares_environment_variables(&self) -> bool {
        match &self.environment_variables {
            Value::Null => false,
            Value::Array(values) => !values.is_empty(),
            Value::Object(values) => !values.is_empty(),
            // Fail closed if a future Registry schema uses an unexpected shape.
            _ => true,
        }
    }
}

/// Deserialize `transport` as either a bare string (`"stdio"`) or an object
/// (`{"type": "stdio"}`). The MCP Registry 2025-12 schema ships the object
/// shape; older/draft docs showed the bare string. We accept both.
fn deserialize_transport<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum OneOrString {
        Bare(String),
        Wrapped {
            #[serde(rename = "type")]
            r#type: String,
        },
    }

    let opt: Option<OneOrString> = Option::deserialize(deserializer)?;
    Ok(opt.map(|v| match v {
        OneOrString::Bare(s) => s,
        OneOrString::Wrapped { r#type } => r#type,
    }))
}

/// Deserialize `runtimeArguments` as either `Vec<String>` (old schema)
/// or `Vec<{value, name, default, type, ...}>` (2025-12 schema). In the
/// object case we derive a string value from the available fields:
/// - Named args (`type: "named"`): `"{name} {default}"` or just `name`
/// - Positional args (`type: "positional"`): `default`
/// - Legacy objects with `value`: use `value` directly
fn deserialize_runtime_arguments<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrArg {
        Bare(String),
        Wrapped {
            #[serde(default)]
            value: Option<String>,
            #[serde(default)]
            name: Option<String>,
            #[serde(default)]
            default: Option<String>,
        },
    }

    let raw: Vec<StringOrArg> = Vec::deserialize(deserializer)?;
    let mut args = Vec::new();
    for arg in raw {
        match arg {
            StringOrArg::Bare(value) => args.push(value),
            StringOrArg::Wrapped {
                value: Some(value), ..
            } => args.push(value),
            StringOrArg::Wrapped { name, default, .. } => {
                if let Some(name) = name {
                    args.push(name);
                }
                if let Some(default) = default {
                    args.push(default);
                }
            }
        }
    }
    Ok(args)
}

/// Derive a runtime hint from `registryType` when the upstream omits one.
/// Kept small on purpose: only the runtimes we know how to launch.
fn default_runtime_hint(registry_type: &str) -> Option<&'static str> {
    match registry_type {
        "npm" => Some("npx"),
        "pypi" => Some("uvx"),
        _ => None,
    }
}

#[derive(Deserialize)]
struct RegistryArg {
    // Positional arguments ship without a name — only `value` and
    // `type`. Named arguments (`{"name": "--foo", "value": "bar", ...}`)
    // carry it. Accept both: when missing, downstream code uses `value`
    // as the arg name.
    #[serde(default)]
    name: Option<String>,
    description: Option<String>,
    // Upstream allows omitting `isRequired`; default false per spec.
    #[serde(rename = "isRequired", default)]
    is_required: bool,
    // Upstream `type` discriminator (`"positional"` / `"named"`). Drives
    // cmd-format decisions downstream. Renamed because `type` is a
    // reserved word in Rust.
    #[serde(rename = "type", default)]
    kind: Option<String>,
    #[serde(default)]
    value: Option<String>,
    default: Option<String>,
    // `format` dropped — never read by any consumer.
}

// === Cached index types ===

#[derive(Deserialize)]
struct RegistryMetadata {
    #[serde(rename = "nextCursor")]
    next_cursor: Option<String>,
}

// === Cached index types ===
//
// The cache file (`~/.codewhale/mcp-index.json`) is the on-disk source of
// truth for Registry-discovered local MCP launch metadata.

/// Bumped whenever the cache shape changes. Lets the loader detect an old
/// cache file and trigger a full resync instead of failing to deserialize.
pub const MCP_REGISTRY_CACHE_VERSION: u32 = 6;

#[derive(Serialize, Deserialize)]
pub struct McpRegistryIndex {
    pub version: u32,
    pub count: usize,
    pub servers: Vec<McpRegistryServerEntry>,
}

/// One Registry catalog entry exposed to the model for contextual selection.
#[derive(Serialize, Deserialize, Clone)]
pub struct DigestEntry {
    pub name: String,
    pub description: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_args: Vec<McpRegistryArgEntry>,
}

/// One cached server entry used by discovery and structured startup.
///
/// Everything else that the upstream Registry ships (`title`, repository,
/// `packages[]`, optional named args,
/// runtime_arguments at package level) is dropped. Fixed positional
/// `packageArguments` (e.g. an `mcp` subcommand) are folded into
/// `run_command` at render time rather than kept as fields.
#[derive(Serialize, Deserialize, Clone)]
pub struct McpRegistryServerEntry {
    pub name: String,
    pub description: String,
    pub launch: McpLaunchSpec,
}

/// Host-owned launch data for one zero-environment stdio server.
#[derive(Serialize, Deserialize, Clone)]
pub struct McpLaunchSpec {
    /// Template for the run command. The literal substring `<ARGS>` is
    /// replaced by host-rendered structured argument values.
    pub run_command: String,
    pub required_args: Vec<McpRegistryArgEntry>,
}

/// One CLI argument required at install time. `is_required` was dropped
/// because the cache only stores required args (others are filtered out
/// during sync). `kind` carries the upstream `type` discriminator
/// (`"positional"` vs `"named"`) so the cmd builder can decide whether
/// to emit `--name value` or just `value`.
#[derive(Serialize, Deserialize, Clone)]
pub struct McpRegistryArgEntry {
    pub name: String,
    pub kind: Option<String>,
    pub description: Option<String>,
    pub default: Option<String>,
}

// === Tool implementation ===

pub struct McpSyncRegistry;

const REGISTRY_API: &str = "https://registry.modelcontextprotocol.io/v0.1/servers";
const PER_PAGE: usize = 100;
const REQUEST_TIMEOUT_SECS: u64 = 30;
/// Bounded retries for one sync. The on-disk cache only changes at the
/// final atomic replace, so a failed fetch (HTTP/parse error) never
/// mutates state and retrying is side-effect free.
const MAX_SYNC_ATTEMPTS: usize = 3;
/// Connect budget for Registry-launched servers. The 10s global default is
/// meant for pre-installed servers; Registry packages are typically fetched
/// on first launch via npx/uvx, which routinely exceeds it.
const REGISTRY_CONNECT_TIMEOUT_SECS: u64 = 60;

fn cache_path() -> Result<PathBuf, ToolError> {
    dirs::home_dir()
        .ok_or_else(|| ToolError::execution_failed("Cannot determine home directory"))
        .map(|h| h.join(".codewhale").join("mcp-index.json"))
}

fn read_cache(path: &PathBuf) -> Option<McpRegistryIndex> {
    let data = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&data).ok()
}

/// Convert one fetched listing into launchable cache entries. Servers the
/// upstream marks `deleted`/`deprecated` are dropped (the aggregator guide
/// recommends removing `deleted` entries — moderation takedowns — from
/// downstream indexes), as is anything the structured launcher cannot run.
/// The cache is a full snapshot every sync, so this filtering is the whole
/// story: retired servers simply never enter the fresh index.
/// Status lives in registry-managed `_meta`
/// (`ServerResponse._meta["io.modelcontextprotocol.registry/official"]
/// .status`, enum `active | deprecated | deleted`).
/// <https://github.com/modelcontextprotocol/registry/blob/main/docs/reference/api/openapi.yaml>
fn viable_entries(entries: Vec<RegistryServerEntry>) -> Vec<McpRegistryServerEntry> {
    entries
        .into_iter()
        .filter(|entry| {
            !matches!(
                entry.lifecycle_status(),
                Some("deleted") | Some("deprecated")
            )
        })
        .filter_map(|entry| server_to_entry(entry.server))
        .collect()
}

/// Render the run command template. Positional `packageArguments` with a
/// fixed literal (or defaulted) value are part of the invocation itself —
/// e.g. the `mcp` subcommand in `npx -y agentic-mermaid@0.1.2 mcp` — so
/// they are folded into the command right after the package spec.
/// `<ARGS>` is the splice point for everything user-supplied: positional
/// placeholders plus each `required_args` entry, rendered as named or
/// positional arguments by the structured Registry launcher.
fn build_run_command(
    runtime_hint: &str,
    identifier: &str,
    version: &str,
    runtime_arguments: &[String],
    package_arguments: &[RegistryArg],
) -> String {
    let runtime = runtime_hint;
    let mut normalized_runtime_arguments = runtime_arguments.to_vec();
    if runtime_hint == "npx"
        && !normalized_runtime_arguments
            .iter()
            .any(|argument| matches!(argument.as_str(), "-y" | "--yes"))
    {
        normalized_runtime_arguments.insert(0, "-y".to_string());
    }
    let mid = normalized_runtime_arguments
        .iter()
        .map(|argument| shell_words::quote(argument))
        .collect::<Vec<_>>()
        .join(" ");
    let mid_with_space = if mid.is_empty() {
        String::new()
    } else {
        format!("{mid} ")
    };
    let (sep, tail) = match runtime_hint {
        "npx" => ("@", version.to_string()),
        "uvx" => ("==", version.to_string()),
        _ => return String::new(),
    };
    // Upstream positional packageArguments ship without a `name`; named
    // args always carry one. A nameless arg with a literal `value` (or a
    // `default`) is a fixed token of the invocation, not user input —
    // dropping it renders a command that cannot start the server (the
    // agentic-mermaid `mcp` subcommand bug).
    let fixed: Vec<String> = package_arguments
        .iter()
        .filter(|a| !a.is_required)
        .filter(|a| a.name.is_none())
        .filter_map(|a| a.value.as_deref().or(a.default.as_deref()))
        .map(|value| shell_words::quote(value).into_owned())
        .collect();
    let fixed_str = if fixed.is_empty() {
        String::new()
    } else {
        format!(" {}", fixed.join(" "))
    };
    let package_spec = format!("{identifier}{sep}{tail}");
    let package = shell_words::quote(&package_spec);
    format!("{runtime} {mid_with_space}{package}{fixed_str} <ARGS>")
}

fn build_launch_spec(
    runtime_hint: &str,
    identifier: &str,
    version: &str,
    pkg: &RegistryPackage,
) -> McpLaunchSpec {
    McpLaunchSpec {
        run_command: build_run_command(
            runtime_hint,
            identifier,
            version,
            &pkg.runtime_arguments,
            &pkg.package_arguments,
        ),
        required_args: pkg
            .package_arguments
            .iter()
            .filter(|a| a.is_required)
            .enumerate()
            .map(|(index, a)| McpRegistryArgEntry {
                // Positional args omit `name` upstream; fall back to the
                // value so the cache still carries something the cmd
                // builder can render.
                name: a
                    .name
                    .clone()
                    .or_else(|| a.value.clone())
                    .unwrap_or_else(|| format!("arg_{}", index + 1)),
                kind: a.kind.clone(),
                description: a.description.clone(),
                default: a.default.clone(),
            })
            .collect(),
    }
}

fn server_to_entry(server: RegistryServer) -> Option<McpRegistryServerEntry> {
    // We only need the FIRST viable stdio package per server for
    // launch metadata; everything beyond it would just duplicate
    // info. Filter to stdio, resolve runtime_hint + version, and stop
    // at the first hit.
    let first_pkg = server
        .packages
        .unwrap_or_default()
        .into_iter()
        .filter(|p| p.transport.as_deref() == Some("stdio"))
        .filter(|p| !p.declares_environment_variables())
        // The automatic launcher currently has deterministic install/run
        // semantics for package-manager-backed npm and PyPI entries only.
        .filter(|p| matches!(p.registry_type.as_str(), "npm" | "pypi"))
        .find_map(|p| {
            let expected_hint = default_runtime_hint(&p.registry_type)?;
            let hint = p
                .runtime_hint
                .clone()
                .unwrap_or_else(|| expected_hint.to_string());
            if hint != expected_hint {
                return None;
            }
            let version = p.version.clone()?;
            Some((p, hint, version))
        });

    let (pkg, hint, version) = first_pkg?;

    Some(McpRegistryServerEntry {
        name: server.name,
        description: server.description,
        launch: build_launch_spec(&hint, &pkg.identifier, &version, &pkg),
    })
}

#[derive(Serialize)]
struct RegistryCatalogResult {
    instruction: &'static str,
    count: usize,
    servers: Vec<DigestEntry>,
}

fn catalog_from_cache(cache: &McpRegistryIndex) -> RegistryCatalogResult {
    let servers = cache
        .servers
        .iter()
        .map(|server| DigestEntry {
            name: server.name.clone(),
            description: server.description.clone(),
            required_args: server.launch.required_args.clone(),
        })
        .collect::<Vec<_>>();
    RegistryCatalogResult {
        instruction: "REGISTRY-FIRST POLICY: Compare the user's full task against every server name and description. Treat a server as a match when it plausibly covers the task's core specialized capability; wording need not be exact. If any plausible match exists, you must call start_registry_mcp_server with its exact name and inspect its tools before using shell commands, local programs, custom code, or a manual implementation. A familiar local alternative is not a reason to skip it. Fall back only when every catalog entry is clearly irrelevant or the matching server fails to start.",
        count: servers.len(),
        servers,
    }
}

async fn load_registry_catalog() -> Result<RegistryCatalogResult, ToolError> {
    let path = cache_path()?;
    sync_once(&path).await?;
    let cache = read_cache(&path).ok_or_else(|| {
        ToolError::execution_failed(format!(
            "Registry cache was not written at {}",
            path.display()
        ))
    })?;
    Ok(catalog_from_cache(&cache))
}

/// Paginate the full registry listing. Continues from each page's cursor,
/// appending entries, until the upstream reports no more pages — a complete
/// listing is the only shape ever cached. Never writes anything; callers
/// may retry freely because the on-disk cache only changes at the final
/// atomic replace.
async fn fetch_registry_entries(
    client: &reqwest::Client,
) -> Result<Vec<RegistryServerEntry>, ToolError> {
    let mut all_entries: Vec<RegistryServerEntry> = Vec::new();
    let mut cursor: Option<String> = None;
    loop {
        let mut url = format!("{REGISTRY_API}?version=latest&limit={PER_PAGE}");
        if let Some(ref c) = cursor {
            url.push_str(&format!("&cursor={}", urlencoding::encode(c)));
        }
        let resp = client
            .get(&url)
            .send()
            .await
            .and_then(reqwest::Response::error_for_status)
            .map_err(|e| ToolError::execution_failed(format!("Registry API: {e}")))?;
        let text = resp
            .text()
            .await
            .map_err(|e| ToolError::execution_failed(format!("Registry body: {e}")))?;
        let body: RegistryResponse = serde_json::from_str(&text)
            .map_err(|e| ToolError::execution_failed(format!("Registry JSON parse: {e}")))?;
        all_entries.extend(body.servers);
        cursor = body.metadata.and_then(|m| m.next_cursor);
        if cursor.is_none() {
            break;
        }
    }
    Ok(all_entries)
}

/// Pull the full registry listing, convert it to a fresh snapshot, and
/// atomically replace the cache file. The fetch is retried (bounded) on
/// failure; the old cache only changes at the atomic replace, so a failed
/// sync leaves the previous snapshot untouched. Returns `Err` only after
/// the retries are exhausted.
async fn sync_once(path: &Path) -> Result<(), ToolError> {
    // rustls default-provider install pattern (matches `client.rs`).
    let _ = rustls::crypto::ring::default_provider().install_default();
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .build()
        .map_err(|e| ToolError::execution_failed(format!("HTTP client: {e}")))?;

    let mut last_error: Option<ToolError> = None;
    let mut servers: Option<Vec<McpRegistryServerEntry>> = None;
    for _attempt in 0..MAX_SYNC_ATTEMPTS {
        match fetch_registry_entries(&client).await {
            Ok(entries) => {
                servers = Some(viable_entries(entries));
                break;
            }
            Err(error) => last_error = Some(error),
        }
    }
    let servers = servers.ok_or_else(|| {
        last_error.unwrap_or_else(|| ToolError::execution_failed("Registry sync failed"))
    })?;
    let index = McpRegistryIndex {
        version: MCP_REGISTRY_CACHE_VERSION,
        count: servers.len(),
        servers,
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| ToolError::execution_failed(format!("Create cache dir: {e}")))?;
    }
    let json_str = serde_json::to_string_pretty(&index)
        .map_err(|e| ToolError::execution_failed(format!("Serialize: {e}")))?;
    write_atomic(path, json_str.as_bytes())
        .map_err(|e| ToolError::execution_failed(format!("Write cache: {e}")))?;
    Ok(())
}

#[async_trait::async_trait]
impl ToolSpec for McpSyncRegistry {
    fn name(&self) -> &str {
        "registry_sync"
    }

    fn description(&self) -> &str {
        "List installable local MCP servers for Registry-first capability \
         selection. Compare their names and descriptions against the user's \
         full task context before using shell commands, local programs, custom \
         code, or manually reproducing a specialized integration. \
         The index contains only stdio packages \
         that declare no environment variables or API keys. If any server \
         plausibly covers the task's core specialized capability, call \
         start_registry_mcp_server with its exact name and inspect its tools \
         before choosing a local alternative; do not run its package command \
         through exec_shell."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {},
            "additionalProperties": false
        })
    }

    fn capabilities(&self) -> Vec<ToolCapability> {
        vec![ToolCapability::Network]
    }

    fn approval_requirement(&self) -> ApprovalRequirement {
        ApprovalRequirement::Auto
    }

    fn supports_parallel(&self) -> bool {
        true
    }

    async fn execute(&self, _input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let result = load_registry_catalog().await?;
        let json = serde_json::to_string(&result)
            .map_err(|e| ToolError::execution_failed(format!("Serialize: {e}")))?;
        Ok(ToolResult::success(json))
    }
}

/// Start one zero-environment stdio server selected from the Registry cache.
/// The model supplies a Registry identity and structured CLI values; the host
/// owns command construction, so no arbitrary shell command or environment
/// channel is exposed by the discovery flow.
pub struct StartRegistryMcpServer {
    pool: Arc<AsyncMutex<McpPool>>,
}

impl StartRegistryMcpServer {
    pub fn new(pool: Arc<AsyncMutex<McpPool>>) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl ToolSpec for StartRegistryMcpServer {
    fn name(&self) -> &str {
        "start_registry_mcp_server"
    }

    fn description(&self) -> &str {
        "Install and start a local stdio MCP server previously returned by \
         registry_sync. Only Registry packages that declare no environment \
         variables are eligible. Pass the exact registry_name and, when the \
         discovery result lists required_args, provide their values in the \
         structured arguments object. The connected server's complete tool \
         schemas become callable in the same turn."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "registry_name": {
                    "type": "string",
                    "description": "Exact server name returned by registry_sync"
                },
                "arguments": {
                    "type": "object",
                    "additionalProperties": { "type": "string" },
                    "description": "Values keyed by required_args[].name; omit when none are required"
                }
            },
            "required": ["registry_name"]
        })
    }

    fn capabilities(&self) -> Vec<ToolCapability> {
        vec![ToolCapability::Network, ToolCapability::ExecutesCode]
    }

    fn approval_requirement(&self) -> ApprovalRequirement {
        ApprovalRequirement::Required
    }

    async fn execute(&self, input: Value, ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let registry_name = input
            .get("registry_name")
            .and_then(Value::as_str)
            .ok_or_else(|| ToolError::invalid_input("missing required field: registry_name"))?;
        let supplied: HashMap<String, String> = match input.get("arguments") {
            Some(value) => serde_json::from_value(value.clone())
                .map_err(|error| ToolError::invalid_input(format!("invalid arguments: {error}")))?,
            None => HashMap::new(),
        };

        let path = cache_path()?;
        let cache = read_cache(&path).ok_or_else(|| {
            ToolError::execution_failed(format!(
                "no current Registry cache at {}; run registry_sync first",
                path.display()
            ))
        })?;
        if cache.version != MCP_REGISTRY_CACHE_VERSION {
            return Err(ToolError::execution_failed(format!(
                "cache at {} is from an older schema version; run registry_sync first",
                path.display()
            )));
        }
        let entry = cache
            .servers
            .iter()
            .find(|server| server.name == registry_name)
            .ok_or_else(|| ToolError::invalid_input("registry_name is not present in the cache"))?;

        let expected: HashSet<&str> = entry
            .launch
            .required_args
            .iter()
            .map(|arg| arg.name.as_str())
            .collect();
        if let Some(unknown) = supplied
            .keys()
            .find(|name| !expected.contains(name.as_str()))
        {
            return Err(ToolError::invalid_input(format!(
                "unknown argument '{unknown}' for {registry_name}"
            )));
        }

        let mut rendered_args = Vec::new();
        for argument in &entry.launch.required_args {
            let value = supplied
                .get(&argument.name)
                .cloned()
                .or_else(|| argument.default.clone())
                .ok_or_else(|| {
                    ToolError::invalid_input(format!(
                        "missing required argument '{}' for {registry_name}",
                        argument.name
                    ))
                })?;
            if matches!(argument.kind.as_deref(), Some("named")) && !argument.name.is_empty() {
                rendered_args.push(shell_words::quote(&argument.name).into_owned());
            }
            rendered_args.push(shell_words::quote(&value).into_owned());
        }

        let command = entry
            .launch
            .run_command
            .replace("<ARGS>", &rendered_args.join(" "));
        let delegated = json!({
            "server": command.trim(),
            "name": registry_name,
            // Registry packages cold-start through npx/uvx downloads; the
            // 10s default connect budget is routinely exceeded on first
            // launch. This override is host-supplied only — it is not part
            // of the model-facing schema of either tool.
            "connect_timeout": REGISTRY_CONNECT_TIMEOUT_SECS,
        });
        crate::tools::runtime_mcp::StartRuntimeMcpServer::new(Arc::clone(&self.pool))
            .execute(delegated, ctx)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_no_packages_filtered() {
        let server = RegistryServer {
            name: "test".into(),
            description: "desc".into(),
            packages: None,
        };
        assert!(server_to_entry(server).is_none());
    }

    #[test]
    fn server_remote_only_filtered() {
        let server = RegistryServer {
            name: "test".into(),
            description: "desc".into(),
            packages: Some(vec![RegistryPackage {
                registry_type: "npm".into(),
                identifier: "@test/pkg".into(),
                version: Some("1.0.0".into()),
                runtime_hint: Some("npx".into()),
                transport: Some("streamable-http".into()),
                package_arguments: vec![],
                runtime_arguments: vec![],
                environment_variables: Value::Null,
            }]),
        };
        assert!(server_to_entry(server).is_none());
    }

    #[test]
    fn server_stdio_kept() {
        let server = RegistryServer {
            name: "test".into(),
            description: "desc".into(),
            packages: Some(vec![RegistryPackage {
                registry_type: "npm".into(),
                identifier: "@test/pkg".into(),
                version: Some("1.0.0".into()),
                runtime_hint: Some("npx".into()),
                transport: Some("stdio".into()),
                package_arguments: vec![],
                runtime_arguments: vec!["-y".into()],
                environment_variables: Value::Null,
            }]),
        };
        let entry = server_to_entry(server).unwrap();
        assert_eq!(entry.launch.run_command, "npx -y @test/pkg@1.0.0 <ARGS>");
    }

    #[test]
    fn server_declaring_environment_variables_is_filtered() {
        let server = RegistryServer {
            name: "needs-secret".into(),
            description: "requires an API key".into(),
            packages: Some(vec![RegistryPackage {
                registry_type: "npm".into(),
                identifier: "@test/secret-server".into(),
                version: Some("1.0.0".into()),
                runtime_hint: Some("npx".into()),
                transport: Some("stdio".into()),
                package_arguments: vec![],
                runtime_arguments: vec!["-y".into()],
                environment_variables: json!([{ "name": "API_KEY", "isRequired": true }]),
            }]),
        };
        assert!(server_to_entry(server).is_none());
    }

    #[test]
    fn fixed_positional_package_arguments_render_into_run_command() {
        // Regression for the agentic-mermaid launch failure: upstream
        // declares `packageArguments: [{"value": "mcp", "type":
        // "positional"}]` (no `isRequired`), and dropping that token
        // produced `npx -y agentic-mermaid@0.1.2 <ARGS>` — which starts
        // the package's default entrypoint, not the MCP server.
        let server = RegistryServer {
            name: "io.github.adewale/agentic-mermaid".into(),
            description: "Render Mermaid diagrams through MCP.".into(),
            packages: Some(vec![RegistryPackage {
                registry_type: "npm".into(),
                identifier: "agentic-mermaid".into(),
                version: Some("0.1.2".into()),
                runtime_hint: Some("npx".into()),
                transport: Some("stdio".into()),
                package_arguments: vec![RegistryArg {
                    name: None,
                    description: None,
                    is_required: false,
                    kind: Some("positional".into()),
                    value: Some("mcp".into()),
                    default: None,
                }],
                runtime_arguments: vec!["-y".into()],
                environment_variables: Value::Null,
            }]),
        };
        let entry = server_to_entry(server).unwrap();
        assert_eq!(
            entry.launch.run_command,
            "npx -y agentic-mermaid@0.1.2 mcp <ARGS>"
        );
        assert!(entry.launch.required_args.is_empty());
    }

    #[test]
    fn placeholder_positional_package_argument_stays_in_required_args() {
        // The flip side: a positional arg with neither `value` nor
        // `default` is user input (e.g. an allowed directory), not a
        // fixed token — it must NOT leak into the rendered command.
        let server = RegistryServer {
            name: "test".into(),
            description: "desc".into(),
            packages: Some(vec![RegistryPackage {
                registry_type: "npm".into(),
                identifier: "@test/fs".into(),
                version: Some("1.0.0".into()),
                runtime_hint: Some("npx".into()),
                transport: Some("stdio".into()),
                package_arguments: vec![RegistryArg {
                    name: None,
                    description: Some("Directory to expose".into()),
                    is_required: true,
                    kind: Some("positional".into()),
                    value: None,
                    default: None,
                }],
                runtime_arguments: vec!["-y".into()],
                environment_variables: Value::Null,
            }]),
        };
        let entry = server_to_entry(server).unwrap();
        assert_eq!(entry.launch.run_command, "npx -y @test/fs@1.0.0 <ARGS>");
        assert_eq!(entry.launch.required_args.len(), 1);
    }

    #[test]
    fn fixed_argument_with_spaces_preserves_one_process_argument() {
        let command = build_run_command(
            "npx",
            "@test/fs",
            "1.0.0",
            &[],
            &[RegistryArg {
                name: None,
                description: None,
                is_required: false,
                kind: Some("positional".into()),
                value: Some("/tmp/a folder".into()),
                default: None,
            }],
        );
        let parsed = shell_words::split(command.replace("<ARGS>", "").trim()).unwrap();
        assert_eq!(parsed.last().map(String::as_str), Some("/tmp/a folder"));
    }

    #[test]
    fn server_without_explicit_stdio_transport_is_filtered() {
        let server = RegistryServer {
            name: "test".into(),
            description: "desc".into(),
            packages: Some(vec![RegistryPackage {
                registry_type: "npm".into(),
                identifier: "@test/pkg".into(),
                version: Some("1.0.0".into()),
                runtime_hint: Some("npx".into()),
                transport: None,
                package_arguments: vec![],
                runtime_arguments: vec![],
                environment_variables: Value::Null,
            }]),
        };
        assert!(server_to_entry(server).is_none());
    }

    #[test]
    fn unsupported_registry_runtime_is_not_advertised() {
        let server = RegistryServer {
            name: "container-only".into(),
            description: "OCI stdio server".into(),
            packages: Some(vec![RegistryPackage {
                registry_type: "oci".into(),
                identifier: "docker.io/example/server:1.0.0".into(),
                version: None,
                runtime_hint: Some("docker".into()),
                transport: Some("stdio".into()),
                package_arguments: vec![],
                runtime_arguments: vec![],
                environment_variables: Value::Null,
            }]),
        };
        assert!(server_to_entry(server).is_none());
    }

    #[test]
    fn registry_type_and_runtime_must_match() {
        let server = RegistryServer {
            name: "mismatched".into(),
            description: "invalid npm runner".into(),
            packages: Some(vec![RegistryPackage {
                registry_type: "npm".into(),
                identifier: "example".into(),
                version: Some("1.0.0".into()),
                runtime_hint: Some("uvx".into()),
                transport: Some("stdio".into()),
                package_arguments: vec![],
                runtime_arguments: vec![],
                environment_variables: Value::Null,
            }]),
        };
        assert!(server_to_entry(server).is_none());
    }

    /// End-to-end smoke test: drive `McpSyncRegistry::execute()` against the
    /// real MCP Registry API, verify the cache file lands at the right
    /// location with a valid shape, and verify the returned summary is
    /// self-consistent.
    ///
    /// Marked `#[ignore]` because it depends on:
    ///   * network access to <https://registry.modelcontextprotocol.io>
    ///   * the upstream API schema matching our deserialization types
    ///   * ~14s of wall clock for the first cold-cache sync
    ///
    /// Calls the public tool against an empty cache to exercise cold sync.
    ///
    /// Run manually with:
    ///   cargo test -p codewhale-tui --bin codewhale-tui --locked \
    ///     execute_writes_cache_file_and_returns_summary -- --ignored --nocapture
    ///
    /// This test deliberately overrides `HOME` to a tempdir so it never
    /// touches the user's real `~/.codewhale/mcp-index.json`.
    #[tokio::test]
    #[ignore = "requires network access to the public MCP Registry; \
                run with `cargo test -- --ignored`"]
    async fn execute_writes_cache_file_and_returns_summary() {
        use crate::test_support::{EnvVarGuard, lock_test_env};
        use crate::tools::spec::ToolContext;

        // Serialize env mutation across all tests in this binary.
        let _env_lock = lock_test_env();

        let tmp = tempfile::tempdir().expect("tempdir");
        // Persist the tempdir so we can inspect the cache file after the
        // test returns. `TempDir::keep` (newer API) disables Drop's
        // cleanup so the directory leaks — acceptable for a manual-run
        // integration smoke test that intentionally outlives its scope.
        let tmp_path = tmp.keep();
        let _home_guard = EnvVarGuard::set("HOME", &tmp_path);

        let workspace = tmp_path.clone();
        let ctx = ToolContext::new(workspace);

        let input = json!({});

        let result = McpSyncRegistry
            .execute(input, &ctx)
            .await
            .expect("execute() should not error against the live Registry");

        assert!(
            result.success,
            "execute returned non-success: content={}",
            result.content
        );

        // Cache file should land under the overridden HOME.
        let cache_path = tmp_path.join(".codewhale").join("mcp-index.json");
        assert!(
            cache_path.exists(),
            "cache file should exist at {:?}",
            cache_path
        );

        // Cache file must be valid JSON matching the McpRegistryIndex
        // schema. If parsing fails here, either the write code is broken
        // or the schema drifted from what execute() writes.
        let raw = std::fs::read_to_string(&cache_path).expect("read cache");
        let cache: McpRegistryIndex =
            serde_json::from_str(&raw).expect("cache must parse as McpRegistryIndex");

        // The Registry has hundreds of stdio servers as of 2026; expect at
        // least 1 from a single page. If this fails the upstream either
        // removed all stdio entries or our filter is wrong.
        assert!(
            cache.count > 0,
            "expected at least 1 stdio server, got count={}",
            cache.count
        );
        assert_eq!(
            cache.servers.len(),
            cache.count,
            "cache.count must equal cache.servers.len()"
        );
        for entry in &cache.servers {
            assert!(!entry.name.is_empty(), "server entry has empty name");
            assert!(
                entry.launch.run_command.ends_with("<ARGS>"),
                "kept entry {} run_command should end with <ARGS>; got: {}",
                entry.name,
                entry.launch.run_command
            );
        }

        let payload: serde_json::Value =
            serde_json::from_str(&result.content).expect("result content must parse");
        assert_eq!(payload["count"].as_u64(), Some(cache.count as u64));
        assert_eq!(
            payload["servers"].as_array().map(Vec::len),
            Some(cache.count)
        );
    }

    fn make_test_cache() -> McpRegistryIndex {
        let server = McpRegistryServerEntry {
            name: "io.modelcontextprotocol/filesystem".into(),
            description: "Read/write local files with sandboxed paths".into(),
            launch: McpLaunchSpec {
                run_command: "npx -y @modelcontextprotocol/server-filesystem@1.0.0 <ARGS>".into(),
                required_args: vec![],
            },
        };
        McpRegistryIndex {
            version: MCP_REGISTRY_CACHE_VERSION,
            count: 1,
            servers: vec![server],
        }
    }

    #[test]
    fn catalog_exposes_every_server_for_model_selection() {
        let cache = make_test_cache();
        let catalog = catalog_from_cache(&cache);
        assert_eq!(catalog.count, 1);
        assert_eq!(catalog.servers.len(), 1);
        assert_eq!(
            catalog.servers[0].name,
            "io.modelcontextprotocol/filesystem"
        );
        assert_eq!(
            catalog.servers[0].description,
            "Read/write local files with sandboxed paths"
        );
    }

    /// Parse one `ServerResponse` JSON object the way the upstream list
    /// endpoint ships it. Lifecycle status travels in registry-managed
    /// `_meta` (`io.modelcontextprotocol.registry/official`), as a
    /// SIBLING of `server` — not inside the server body — and this
    /// helper keeps the tests honest about that path.
    /// <https://github.com/modelcontextprotocol/registry/blob/main/docs/reference/api/openapi.yaml>
    fn parse_server_entry(server_json: Value, status: Option<&str>) -> RegistryServerEntry {
        let mut response = json!({ "server": server_json });
        if let Some(status) = status {
            response["_meta"] =
                json!({ "io.modelcontextprotocol.registry/official": { "status": status } });
        }
        serde_json::from_value(response).expect("ServerResponse must deserialize")
    }

    #[test]
    fn viable_entries_keeps_only_active_launchable_servers() {
        let launchable = |name: &str, status: Option<&str>| {
            parse_server_entry(
                json!({
                    "name": name,
                    "description": "d",
                    "packages": [{
                        "registryType": "npm",
                        "identifier": "@test/pkg",
                        "version": "1.0.0",
                        "runtimeHint": "npx",
                        "transport": "stdio",
                        "packageArguments": [],
                        "runtimeArguments": [],
                        "environmentVariables": null
                    }]
                }),
                status,
            )
        };
        let entries = vec![
            launchable("a/active", Some("active")),
            launchable("b/deprecated", Some("deprecated")),
            launchable("c/deleted", Some("deleted")),
            // No `_meta` at all: treated as active.
            launchable("d/no-meta", None),
            // Missing a launchable package: dropped by the launcher filter.
            parse_server_entry(json!({ "name": "e/no-pkg", "description": "d" }), None),
        ];

        let viable = viable_entries(entries);

        let names: Vec<&str> = viable.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["a/active", "d/no-meta"]);
    }

    /// Minimal cache entry — only `name`/`description`/launch matter to the
    /// catalog conversion tests.
    fn cache_entry(name: &str, description: &str) -> McpRegistryServerEntry {
        McpRegistryServerEntry {
            name: name.into(),
            description: description.into(),
            launch: McpLaunchSpec {
                run_command: "npx pkg@1.0.0 <ARGS>".into(),
                required_args: vec![],
            },
        }
    }

    #[test]
    fn catalog_is_not_programmatically_filtered_or_bounded() {
        let servers = (0..12)
            .map(|index| cache_entry(&format!("example/file-{index}"), "file server"))
            .collect::<Vec<_>>();
        let cache = McpRegistryIndex {
            version: MCP_REGISTRY_CACHE_VERSION,
            count: servers.len(),
            servers,
        };
        let result = catalog_from_cache(&cache);
        assert_eq!(result.count, 12);
        assert_eq!(result.servers.len(), 12);
    }

    /// Manual smoke run: execute `McpSyncRegistry` against the live Registry
    /// API and print the catalog payload + cache file metadata to stdout so an
    /// operator can inspect what the model would receive. Pure stdout;
    /// assertions stay minimal so a flaky upstream does not mask a useful
    /// manual run.
    ///
    /// Run with:
    ///   cargo test -p codewhale-tui --bin codewhale-tui --locked \
    ///     execute_and_print_catalog_for_manual_inspection -- --ignored --nocapture
    #[tokio::test]
    #[ignore = "manual smoke run; prints the tool result to stdout \
                (requires network access to registry.modelcontextprotocol.io)"]
    // The module tree denies `print_stderr` (scroll-demon guard, #1085) so
    // TUI runtime code can never leak into ratatui's buffer. This test is
    // the deliberate exception: it only runs manually (`--ignored`) and its
    // entire purpose is printing the payload for operator inspection.
    #[allow(clippy::print_stderr)]
    async fn execute_and_print_catalog_for_manual_inspection() {
        use crate::test_support::{EnvVarGuard, lock_test_env};

        let _env_lock = lock_test_env();

        let tmp = tempfile::tempdir().expect("tempdir");
        // Persist the tempdir past the test so the cache file survives and
        // can be inspected from the shell after the test returns.
        let tmp_path = tmp.keep();
        let _home_guard = EnvVarGuard::set("HOME", &tmp_path);

        let ctx = ToolContext::new(tmp_path.clone());

        let input = json!({});

        let result = McpSyncRegistry
            .execute(input, &ctx)
            .await
            .expect("execute() should not error against the live Registry");

        let cache_path = tmp_path.join(".codewhale").join("mcp-index.json");

        eprintln!("\n=== registry_sync output ===");
        eprintln!("tool:               registry_sync");
        eprintln!(
            "status:             {}",
            if result.success { "ok" } else { "fail" }
        );
        eprintln!("cache_path:         {}", cache_path.display());
        eprintln!("cache_exists:       {}", cache_path.exists());
        if cache_path.exists() {
            match std::fs::metadata(&cache_path) {
                Ok(meta) => eprintln!("cache_size_bytes:   {}", meta.len()),
                Err(e) => eprintln!("cache_stat_error:   {e}"),
            }
        }
        eprintln!("--- catalog payload (what the model sees) ---");
        eprintln!("{}", result.content);
        eprintln!("=== end ===\n");
    }
}
