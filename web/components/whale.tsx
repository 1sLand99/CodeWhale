/** Static Signal Current mark from the managed Codewhale product contract. */
export function Whale({ size = 36, className = "" }: { size?: number; className?: string }) {
  return (
    <svg
      viewBox="0 0 64 64"
      width={size}
      height={size}
      className={`codewhale-mark ${className}`}
      aria-hidden="true"
      fill="none"
    >
      <path
        className="codewhale-mark-primary"
        d="M7 57c9-13 21-15 25-25 3-7-1-12-10-16 6-1 11 1 14 6 3-6 10-11 19-13-1 10-5 17-12 21-4 3-5 8-8 14-5 9-15 14-28 13Z"
      />
      <path
        className="codewhale-mark-current"
        d="M28 58c10-8 15-15 17-26 4 8 2 18-3 26H28Z"
      />
    </svg>
  );
}
