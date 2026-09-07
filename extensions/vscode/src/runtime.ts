import * as vscode from "vscode";
import type { SnapshotEntry, ThreadSummary } from "./api";

export type { SnapshotEntry, ThreadSummary };

export interface RuntimeState {
  kind: "connected" | "offline" | "auth-required" | "error";
  baseUrl: string;
  detail: string;
  version?: string;
}

export interface RuntimeConfig {
  commandPath: string;
  host: string;
  port: number;
  token?: string;
  agentViewRefreshIntervalSeconds: number;
}

export function readRuntimeConfig(): RuntimeConfig {
  const config = vscode.workspace.getConfiguration("codewhale");
  const commandPath = config.get<string>("commandPath", "codewhale").trim() || "codewhale";
  const host = config.get<string>("runtimeHost", "127.0.0.1").trim() || "127.0.0.1";
  const port = config.get<number>("runtimePort", 7878);
  const token = config.get<string>("runtimeToken", "").trim();
  const interval = config.get<number>("agentViewRefreshIntervalSeconds", 15);
  return {
    commandPath,
    host,
    port,
    token: token.length > 0 ? token : undefined,
    agentViewRefreshIntervalSeconds: clampRefreshInterval(interval),
  };
}

export function runtimeBaseUrl(config: RuntimeConfig): string {
  return `http://${config.host}:${config.port}`;
}

export function startRuntimeTerminal(config: RuntimeConfig): vscode.Terminal {
  const terminal = vscode.window.createTerminal("CodeWhale Runtime");
  const args = [
    "serve",
    "--http",
    "--host",
    shellQuote(config.host),
    "--port",
    String(config.port),
  ];
  if (config.token) {
    args.push("--auth-token", shellQuote(config.token));
  }
  terminal.sendText(`${shellQuote(config.commandPath)} ${args.join(" ")}`);
  terminal.show();
  return terminal;
}

export function openCodeWhaleTerminal(config: RuntimeConfig): vscode.Terminal {
  const terminal = vscode.window.createTerminal("CodeWhale");
  terminal.sendText(shellQuote(config.commandPath));
  terminal.show();
  return terminal;
}

function clampRefreshInterval(value: number): number {
  if (!Number.isFinite(value)) {
    return 15;
  }
  return Math.max(0, Math.min(300, Math.floor(value)));
}

function shellQuote(value: string): string {
  if (/^[A-Za-z0-9_./:=+-]+$/.test(value)) {
    return value;
  }
  return `'${value.replace(/'/g, "'\\''")}'`;
}
