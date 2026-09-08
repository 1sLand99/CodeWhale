#!/usr/bin/env python3
"""Convert selected OpenCode/DSH data into a reviewable native plugin bundle.

This is an offline authoring tool, not a foreign plugin runtime or installer.
The existing Codewhale /plugin install and hash-bound review remain authoritative.
Requires Python 3.10+ and PyYAML 6+; never loads upstream code or configuration.
"""

import argparse
import ipaddress
import json
import os
from pathlib import Path
import re
import stat
import sys
from urllib.parse import urlsplit

import yaml


MAX_FILES = 4096
MAX_BYTES = 64 * 1024 * 1024
MAX_DOCUMENT = 1024 * 1024
NAME = re.compile(r"[a-z0-9](?:[a-z0-9.-]{0,62}[a-z0-9])?\Z")


class ConversionError(ValueError):
    pass


def require(condition, message):
    if not condition:
        raise ConversionError(message)


def mapping(value, allowed=None):
    require(isinstance(value, dict), "Expected a configuration object.")
    require(all(isinstance(k, str) for k in value), "Object keys must be strings.")
    if allowed is not None:
        require(not value.keys() - set(allowed), "Unsupported fields; select only documented portable declarations.")
    return value


def unique_pairs(pairs):
    result = {}
    for key, value in pairs:
        require(isinstance(key, str) and key not in result, "Duplicate or non-string object key.")
        result[key] = value
    return result


class DataLoader(yaml.SafeLoader):
    def construct_mapping(self, node, deep=False):
        return unique_pairs((self.construct_object(k, deep=deep), self.construct_object(v, deep=deep))
                            for k, v in node.value)


def data(text, *, json_only=False):
    """Closed data parsing: no YAML aliases/tags, duplicate keys or JS expressions."""
    try:
        if json_only:
            value = json.loads(text, object_pairs_hook=unique_pairs,
                               parse_constant=lambda _: (_ for _ in ()).throw(ConversionError("Non-finite JSON number.")))
        else:
            depth = 0
            for event in yaml.parse(text):
                require(not isinstance(event, yaml.AliasEvent) and not getattr(event, "tag", None),
                        "YAML aliases and explicit tags (including !!js) are unsupported.")
                if isinstance(event, (yaml.MappingStartEvent, yaml.SequenceStartEvent)):
                    depth += 1
                    require(depth <= 32, "Configuration nesting exceeds 32 levels.")
                elif isinstance(event, (yaml.MappingEndEvent, yaml.SequenceEndEvent)):
                    depth -= 1
            value = yaml.load(text, Loader=DataLoader)
        check_data(value)
        return value
    except (yaml.YAMLError, json.JSONDecodeError, RecursionError, TypeError):
        # Parser errors can contain source lines and credentials. Do not echo them.
        raise ConversionError("Cannot parse portable data; use JSON for OpenCode or plain YAML/JSON for DSH.") from None


def check_data(value, depth=0):
    require(depth <= 32, "Configuration nesting exceeds 32 levels.")
    if isinstance(value, dict):
        mapping(value)
        require("__jsExpr" not in value, "DSH executable expressions require a manual port.")
        for child in value.values():
            check_data(child, depth + 1)
    elif isinstance(value, list):
        for child in value:
            check_data(child, depth + 1)
    else:
        require(value is None or type(value) in (str, int, float, bool), "Unsupported data type.")


def plain_path(path):
    """Reject links/reparse points in the supplied path, including ancestors."""
    path = Path(os.path.abspath(path))
    for entry in (path, *path.parents):
        info = entry.lstat()
        require(not stat.S_ISLNK(info.st_mode)
                and not (getattr(info, "st_file_attributes", 0) & 0x400),
                "Source and output paths must not contain links or reparse points.")
    return path


def read_file(path, limit=MAX_DOCUMENT):
    path = plain_path(path)
    info = path.stat()
    require(stat.S_ISREG(info.st_mode) and info.st_nlink == 1, "Only regular, non-linked source files are supported.")
    require(info.st_size <= limit, "Source file exceeds the conversion size limit.")
    # O_NOFOLLOW protects the final component against replacement after lstat.
    fd = os.open(path, os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0))
    with os.fdopen(fd, "rb") as source:
        opened = os.fstat(source.fileno())
        require((info.st_dev, info.st_ino) == (opened.st_dev, opened.st_ino), "Source changed during conversion.")
        content = source.read(limit + 1)
    require(len(content) <= limit, "Source file exceeds the conversion size limit.")
    return content


def text_file(path):
    try:
        return read_file(path).decode("utf-8")
    except UnicodeError:
        raise ConversionError("Configuration and skill entrypoints must be UTF-8.") from None


def skill_files(path, max_files=MAX_FILES, max_bytes=MAX_BYTES):
    source = plain_path(path)
    entry = source / "SKILL.md" if source.is_dir() else source
    require(entry.name == "SKILL.md" or entry.suffix == ".md", "Select a skill directory or Markdown skill file.")
    text = text_file(entry)
    parts = re.split(r"^---\s*$", text, maxsplit=2, flags=re.MULTILINE)
    require(len(parts) == 3 and not parts[0].strip(), "Skills need YAML frontmatter with name and description.")
    meta = mapping(data(parts[1]), {"name", "description", "license", "compatibility", "metadata",
                                  "disable-model-invocation", "user-invocable"})
    name = meta.get("name")
    require(isinstance(name, str) and re.fullmatch(r"[a-z0-9]+(?:-[a-z0-9]+)*", name)
            and len(name) <= 64, "Skill name must be a kebab-case identifier of at most 64 characters.")
    description = meta.get("description")
    require(isinstance(description, str) and description.strip(), "Skills need a non-empty description.")
    require("---" not in description, "Skill description contains a delimiter the native reader cannot preserve.")
    require(type(meta.get("user-invocable", True)) is bool and meta.get("user-invocable", True),
            "user-invocable:false has no equivalent in the native skill adapter; port it manually.")
    explicit = meta.get("disable-model-invocation", False)
    require(type(explicit) is bool, "disable-model-invocation must be a boolean.")
    # Native parsing is deliberately simpler than YAML. Emit unambiguous core fields.
    front = f"---\nname: {name}\ndescription: |-\n"
    front += "\n".join("  " + line for line in description.splitlines()) + "\n"
    if explicit:
        front += "invocation: explicit-only\n"
    generated = (front + "---" + parts[2]).encode("utf-8")
    files = {f"skills/{name}/SKILL.md": generated}
    extra = {key: meta[key] for key in ("license", "compatibility", "metadata") if key in meta}
    if extra:
        # Native frontmatter is a flat parser: nested metadata must not override
        # its name/description/invocation. Preserve attribution as companion data.
        files[f"skills/{name}/SOURCE_SKILL_METADATA.json"] = (json.dumps(extra, ensure_ascii=False, indent=2) + "\n").encode()
    used_bytes = sum(map(len, files.values()))
    require(len(files) <= max_files and used_bytes <= max_bytes, "Selected skills exceed the bundle budget.")
    if source.is_dir():
        visited = 0
        for current, dirs, children in os.walk(source, followlinks=False):
            for child in dirs + children:
                visited += 1
                require(visited <= MAX_FILES, "Selected skill contains too many filesystem entries.")
                candidate = plain_path(Path(current) / child)
                require(candidate.name not in {".git", ".env", ".installed-from"},
                        "Remove local secrets, repository metadata or install receipts from the selected skill.")
                if candidate.is_dir() or candidate == entry:
                    continue
                relative = f"skills/{name}/{candidate.relative_to(source).as_posix()}"
                require(relative not in files, "Skill companion collides with generated metadata.")
                require(len(files) < max_files, "Selected skills exceed the file budget.")
                files[relative] = read_file(candidate, max_bytes - used_bytes)
                used_bytes += len(files[relative])
    return name, files


def timeout(value):
    require(type(value) is int and 1000 <= value <= 3600000 and value % 1000 == 0,
            "Timeouts must be whole seconds expressed in milliseconds (1000–3600000); port other values manually.")
    return value // 1000


def remote_server(config, dialect, defaults):
    extension = {}
    if dialect == "dsh":
        mapping(config)
        require(config.get("transport") == "streamable-http", "Local MCP processes require manual native authoring.")
        mapping(config, {"serverName", "transport", "url", "headers", "toolCallTimeoutMs", "failOnStartupError"})
        require(config.get("failOnStartupError", False) is False, "DSH startup-failure policy requires a manual port.")
        if "toolCallTimeoutMs" in config:
            extension["execute_timeout"] = timeout(config["toolCallTimeoutMs"])
    else:
        mapping(config)
        require(config.get("type") == "remote", "Local MCP processes require manual native authoring.")
        mapping(config, {"type", "url", "headers", "oauth", "enabled" if dialect == "opencode-v1" else "disabled", "timeout"})
        require(config.get("oauth") is False, "Set oauth:false explicitly; plugin OAuth and upstream auto-OAuth cannot be converted.")
        disabled = not config.get("enabled", True) if dialect == "opencode-v1" else config.get("disabled", False)
        flag = config.get("enabled", True) if dialect == "opencode-v1" else config.get("disabled", False)
        require(type(flag) is bool, "MCP enablement must be a boolean.")
        extension["disabled"] = disabled
        if dialect == "opencode-v1":
            if "timeout" in config:
                extension["connect_timeout"] = timeout(config["timeout"])
                extension["execute_timeout"] = timeout(config["timeout"])
        else:
            limits = {**mapping(defaults, {"startup", "request"}),
                      **mapping(config.get("timeout", {}), {"startup", "request"})}
            for old, new in (("startup", "connect_timeout"), ("request", "execute_timeout")):
                if old in limits:
                    extension[new] = timeout(limits[old])
    url = config.get("url")
    require(isinstance(url, str) and not re.search(r"[\s\\{}]", url), "MCP URL must be a literal endpoint without interpolation.")
    try:
        parsed = urlsplit(url)
        host = parsed.hostname
        require(bool(host) and parsed.port != 0, "MCP URL needs a valid host and port.")
    except ValueError:
        raise ConversionError("Invalid MCP endpoint URL.") from None
    require(parsed.scheme == "https" or (parsed.scheme == "http" and host in {"localhost", "127.0.0.1", "::1"}),
            "MCP endpoints need HTTPS (or explicit loopback HTTP).")
    require(parsed.username is None and parsed.password is None and not parsed.query and not parsed.fragment,
            "MCP URLs must not contain credentials, query strings or fragments.")
    require(host.isascii() and len(url) <= 4096, "Use an ASCII MCP hostname and URL of at most 4096 characters.")
    # URL implementations disagree on shorthand/hex IPv4 spellings. Emit only
    # canonical numeric hosts so the reviewed native host set is identical.
    if ":" in host or re.fullmatch(r"(?:[0-9]+|0x[0-9a-f]+)", host.rsplit(".", 1)[-1]):
        try:
            address = ipaddress.ip_address(host)
            require(str(address) == host, "Use a canonical numeric MCP address.")
            host = f"[{host}]" if address.version == 6 else host
        except ValueError:
            raise ConversionError("Use a canonical numeric MCP address.") from None
    headers = mapping(config.get("headers", {}))
    require(len(headers) <= 64, "At most 64 environment-backed headers are supported.")
    env_headers = {}
    seen_headers = set()
    for key, value in headers.items():
        require(re.fullmatch(r"[A-Za-z0-9!#$%&'*+.^_`|~-]+", key) is not None, "Invalid HTTP header name.")
        require(key.lower() not in seen_headers and key.lower() not in {"accept", "content-type"},
                "HTTP header names must be unique ignoring case; Accept and Content-Type belong to the native transport.")
        seen_headers.add(key.lower())
        require(isinstance(value, str), "HTTP headers must reference environment variable names.")
        reference = re.fullmatch(r"\{env:([A-Za-z_][A-Za-z0-9_]*)\}", value) if dialect != "dsh" else None
        require(reference is not None, "Literal headers, DSH expressions and file interpolation cannot be converted; author native env_headers manually.")
        env_headers[key] = reference[1]
    if env_headers:
        extension["env_headers"] = env_headers
    return {"type": "streamable-http", "url": url,
            "extensions": {"net.codewhale": extension}}, host


def mcp_config(path, dialect):
    document = data(text_file(path), json_only=dialect != "dsh")
    ignored = 0
    if dialect == "dsh":
        require(isinstance(document, list), "DSH input must be a plain Cordis entry list, not a profile or patch composition.")
        entries = []
        for row in document:
            mapping(row, {"name", "id", "config", "disabled"})
            require(row.get("name") == "@deepseek-ai/dsh-mcp-client", "DSH runtime plugins and patch operations require a manual port.")
            require(type(row.get("disabled", False)) is bool, "DSH disabled must be a boolean.")
            config = mapping(row.get("config"))
            entries.append((config.get("serverName"), config, row.get("disabled", False)))
        defaults = {}
    else:
        mapping(document)
        require(not document.get("plugin") and not document.get("plugins"), "OpenCode executable plugins require a manual port; select portable data only.")
        ignored = len(document.keys() - {"$schema", "mcp", "plugin", "plugins"})
        servers = mapping(document.get("mcp", {}))
        defaults = {}
        if dialect == "opencode-v2":
            mapping(servers, {"servers", "timeout"})
            defaults = servers.get("timeout", {})
            servers = mapping(servers.get("servers", {}))
        entries = [(key, value, False) for key, value in servers.items()]
    require(len(entries) <= 64, "At most 64 MCP servers can be converted at once.")
    result, hosts = {}, set()
    for name, config, disabled in entries:
        require(isinstance(name, str) and re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9_-]{0,31}", name), "Invalid or missing MCP server name.")
        require(name not in result, "Duplicate MCP server name; no entries were written.")
        converted, host = remote_server(config, dialect, defaults)
        if disabled:
            converted["extensions"]["net.codewhale"]["disabled"] = True
        result[name] = converted
        hosts.add(host)
    return result, sorted(hosts), ignored


def convert(args):
    require(NAME.fullmatch(args.name) is not None and ".." not in args.name and "--" not in args.name,
            "Choose a native plugin name: 1–64 lowercase letters/digits with single internal dots or hyphens.")
    output = Path(os.path.abspath(args.output))
    plain_path(output.parent)
    require(not os.path.lexists(output), "Output already exists; choose a fresh directory. Nothing was overwritten.")
    files, skill_names = {}, set()
    for path in args.skill:
        source = plain_path(path)
        require(source != output and source not in output.parents, "Output must be outside the selected skill.")
        name, additions = skill_files(source, MAX_FILES - len(files), MAX_BYTES - sum(map(len, files.values())))
        require(name not in skill_names, "Duplicate skill name; no files were written.")
        skill_names.add(name)
        files.update(additions)
    servers, hosts, ignored = mcp_config(args.config, args.format) if args.config else ({}, [], 0)
    require(files or servers, "No portable components selected. Use --skill and/or --config.")
    manifest = {"$schema": "https://agent-plugins.org/schemas/plugin.json", "name": args.name}
    if hosts:
        manifest["extensions"] = {"net.codewhale": {"capabilities": {"network_hosts": hosts}}}
    files["plugin.json"] = (json.dumps(manifest, indent=2) + "\n").encode()
    if servers:
        files["mcp.json"] = (json.dumps({"mcpServers": servers}, indent=2) + "\n").encode()
    files["CONVERSION.md"] = (f"# Conversion receipt\n\nSource dialect: {args.format}.\n"
        f"Converted {len(skill_names)} selected Skills and {len(servers)} remote MCP declarations.\n"
        f"Ignored {ignored} unrelated top-level application settings.\n\n"
        "No source code, package manager, install hook, network request or credential lookup ran.\n"
        "Companion skill files were copied as data; review them before loading a skill.\n"
        "This output is not installed, trusted or enabled. Run `/plugin install <directory>`,\n"
        "then `/plugin validate <name>` and review the exact trust token before enabling.\n"
        "Conversion does not prove server connectivity or foreign runtime compatibility.\n").encode()
    require(len(files) <= MAX_FILES and sum(map(len, files.values())) <= MAX_BYTES,
            "Output exceeds the 4096-file / 64 MiB bundle budget.")
    # Complete all parsing before exclusive creation. Never install or alter a source tree.
    output.mkdir(mode=0o700)
    written = []
    try:
        for relative, content in files.items():
            destination = output / relative
            destination.parent.mkdir(parents=True, exist_ok=True)
            with destination.open("xb") as handle:
                written.append(destination)
                handle.write(content)
    except OSError:
        # Remove only files this operation created, never a pre-existing tree.
        for destination in reversed(written):
            destination.unlink(missing_ok=True)
        for current, dirs, _ in os.walk(output, topdown=False):
            for directory in dirs:
                (Path(current) / directory).rmdir()
        output.rmdir()
        raise
    return len(skill_names), len(servers), ignored


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--format", choices=("opencode-v1", "opencode-v2", "dsh"), required=True)
    parser.add_argument("--config", type=Path, help="OpenCode JSON or static DSH Cordis YAML/JSON (optional)")
    parser.add_argument("--skill", type=Path, action="append", default=[], help="Explicit skill directory or Markdown file; repeatable")
    parser.add_argument("--name", required=True, help="Name for the new native bundle")
    parser.add_argument("--output", type=Path, required=True, help="Fresh directory whose parent already exists")
    args = parser.parse_args()
    try:
        skills, servers, ignored = convert(args)
    except (ConversionError, OSError, UnicodeError) as error:
        message = str(error) if isinstance(error, ConversionError) else "File operation failed; source and output must be accessible regular paths."
        print(f"Conversion refused: {message}", file=sys.stderr)
        return 1
    print(f"Prepared {skills} Skills and {servers} remote MCP declarations; {ignored} unrelated settings omitted.")
    print("Review CONVERSION.md, then use the existing /plugin install, validate, trust and enable commands.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
