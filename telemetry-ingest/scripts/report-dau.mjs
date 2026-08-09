#!/usr/bin/env node

import { pathToFileURL } from "node:url";

const SQL_ENDPOINT = (accountId) =>
  `https://api.cloudflare.com/client/v4/accounts/${encodeURIComponent(accountId)}/analytics_engine/sql`;

export function parseArgs(argv) {
  let days = 14;
  let json = false;
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--json") {
      json = true;
      continue;
    }
    if (arg === "--days") {
      const raw = argv[index + 1];
      index += 1;
      days = Number(raw);
      continue;
    }
    throw new Error(`unknown argument: ${arg}`);
  }
  if (!Number.isInteger(days) || days < 1 || days > 90) {
    throw new Error("--days must be an integer from 1 through 90");
  }
  return { days, json };
}

export function dauSql(days) {
  return `SELECT
  toDate(timestamp) AS day,
  count(DISTINCT index1) AS active_installs,
  sum(_sample_interval) AS sessions_started
FROM codewhale_telemetry
WHERE timestamp >= toStartOfDay(NOW()) - INTERVAL '${days - 1}' DAY
  AND blob1 = 'session_start'
GROUP BY day
ORDER BY day DESC
FORMAT JSON`;
}

export function rowsFromResponse(payload) {
  const rows = Array.isArray(payload) ? payload : payload?.data;
  if (!Array.isArray(rows)) {
    throw new Error("Cloudflare SQL response did not contain a data array");
  }
  return rows.map((row) => ({
    day: String(row.day),
    active_installs: Number(row.active_installs),
    sessions_started: Number(row.sessions_started),
  }));
}

export function formatReport(rows, now = new Date()) {
  const today = now.toISOString().slice(0, 10);
  const lines = [
    "Codewhale observed active installs (UTC)",
    "day         active installs   sessions started",
  ];
  for (const row of rows) {
    const day = row.day === today ? `${row.day}*` : row.day;
    lines.push(
      `${day.padEnd(12)} ${String(row.active_installs).padStart(15)} ${String(row.sessions_started).padStart(18)}`,
    );
  }
  lines.push("", "* current UTC day is partial");
  lines.push(
    "Definition: distinct random install IDs observed that day; not people or accounts.",
    "Coverage: lower bound; pre-v0.9.6 clients were opt-in, and opt-outs/offline or dropped flushes remain unobserved.",
  );
  return lines.join("\n");
}

async function queryDau({ accountId, apiToken, days, fetchImpl = fetch }) {
  const response = await fetchImpl(SQL_ENDPOINT(accountId), {
    method: "POST",
    headers: {
      Authorization: `Bearer ${apiToken}`,
      "content-type": "text/plain; charset=utf-8",
    },
    body: dauSql(days),
  });
  if (!response.ok) {
    const body = (await response.text()).slice(0, 500);
    throw new Error(`Cloudflare SQL request failed (${response.status}): ${body}`);
  }
  return rowsFromResponse(await response.json());
}

export async function main(argv = process.argv.slice(2), env = process.env) {
  const { days, json } = parseArgs(argv);
  const accountId = env.CF_ACCOUNT_ID?.trim();
  const apiToken = env.CF_API_TOKEN?.trim();
  if (!accountId || !apiToken) {
    throw new Error("CF_ACCOUNT_ID and CF_API_TOKEN are required");
  }
  const rows = await queryDau({ accountId, apiToken, days });
  if (json) {
    process.stdout.write(`${JSON.stringify({ timezone: "UTC", days, rows }, null, 2)}\n`);
  } else {
    process.stdout.write(`${formatReport(rows)}\n`);
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main().catch((error) => {
    process.stderr.write(`report:dau: ${error.message}\n`);
    process.exitCode = 1;
  });
}
