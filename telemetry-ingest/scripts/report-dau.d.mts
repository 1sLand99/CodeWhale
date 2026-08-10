export interface DauRow {
  day: string;
  active_installs: number;
  sessions_started: number;
}

export interface ReportArgs {
  days: number;
  json: boolean;
}

export function parseArgs(argv: string[]): ReportArgs;
export function dauSql(days: number): string;
export function rowsFromResponse(payload: unknown): DauRow[];
export function formatReport(rows: DauRow[], now?: Date): string;
export function main(argv?: string[], env?: NodeJS.ProcessEnv): Promise<void>;
