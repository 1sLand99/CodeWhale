import { describe, expect, it } from "vitest";

import {
  dauSql,
  formatReport,
  parseArgs,
  rowsFromResponse,
} from "../scripts/report-dau.mjs";

describe("DAU owner report", () => {
  it("bounds the query to Analytics Engine retention", () => {
    expect(parseArgs([])).toEqual({ days: 14, json: false });
    expect(parseArgs(["--days", "7", "--json"])).toEqual({ days: 7, json: true });
    expect(() => parseArgs(["--days", "0"])).toThrow(/1 through 90/);
    expect(() => parseArgs(["--days", "91"])).toThrow(/1 through 90/);
  });

  it("counts distinct installs only from session starts", () => {
    const sql = dauSql(14);
    expect(sql).toContain("count(DISTINCT index1) AS active_installs");
    expect(sql).toContain("sum(_sample_interval) AS sessions_started");
    expect(sql).toContain("AND blob1 = 'session_start'");
    expect(sql).toContain("INTERVAL '13' DAY");
    expect(sql).toContain("GROUP BY day");
  });

  it("labels the partial UTC day and the lower-bound definition", () => {
    const rows = rowsFromResponse({
      data: [
        {
          day: "2026-08-09",
          active_installs: "43",
          sessions_started: "67",
        },
      ],
    });
    const report = formatReport(rows, new Date("2026-08-09T20:00:00Z"));
    expect(report).toContain("2026-08-09*");
    expect(report).toContain("43");
    expect(report).toContain("not people or accounts");
    expect(report).toContain("pre-v0.9.6 clients were opt-in");
  });
});
