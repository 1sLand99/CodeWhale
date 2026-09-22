import { describe, expect, it } from "vitest";
import { resolveWhale } from "./whale-tokens";
import { siteCss } from "./site-css";

const CSS = siteCss();
const ROLES = ["bg", "surface", "panel", "text", "muted", "line", "accent", "on-accent", "hover", "selected", "selection", "ring"];

function declarations(block: string): Record<string, string> {
  const vars: Record<string, string> = {};
  for (const match of block.matchAll(/--([\w-]+):\s*([^;]+);/g)) vars[match[1]] = match[2].trim();
  return vars;
}

/** The first block for `selector` (at any indent) that declares `--bg`. */
function roleBlock(selector: string): Record<string, string> {
  const escaped = selector.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  for (const match of CSS.matchAll(new RegExp(`(?:^|\\n)\\s*${escaped}\\s*\\{([^}]*)\\}`, "g"))) {
    const vars = declarations(match[1]);
    if ("bg" in vars) return vars;
  }
  throw new Error(`No role block for ${selector}`);
}

function hex(value: string): string {
  const resolved = resolveWhale(value);
  if (!/^#[0-9a-f]{6}$/i.test(resolved)) throw new Error(`not a hex color: ${value} -> ${resolved}`);
  return resolved.toLowerCase();
}

function luminance(color: string): number {
  const [r, g, b] = color
    .slice(1)
    .match(/.{2}/g)!
    .map((v) => Number.parseInt(v, 16) / 255)
    .map((v) => (v <= 0.04045 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4));
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

function contrast(a: string, b: string): number {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (hi + 0.05) / (lo + 0.05);
}

const light = () => roleBlock(":root");
const osDark = () => roleBlock(':root:not([data-theme="light"])');
const pinnedDark = () => roleBlock(':root[data-theme="dark"]');

describe("GPUI role tokens", () => {
  it("defines every role in light, OS-dark, and pinned-dark schemes", () => {
    for (const scheme of [light(), osDark(), pinnedDark()]) {
      for (const role of ROLES) expect(scheme, role).toHaveProperty(role);
    }
    // The OS-dark scheme is guarded so a pinned light page stays light, and
    // the pinned dark block repeats it exactly.
    expect(CSS).toMatch(/@media \(prefers-color-scheme: dark\)\s*\{\s*:root:not\(\[data-theme="light"\]\)\s*\{/);
    expect(pinnedDark()).toEqual(osDark());
  });

  it("uses generated GPUI tokens only, never a literal hex, in role positions", () => {
    for (const scheme of [light(), osDark()]) {
      for (const role of ROLES) {
        expect(scheme[role], role).not.toMatch(/#[0-9a-f]{3,8}\b/i);
        expect(scheme[role], role).toMatch(/var\(--gpui-(light|dark)-[\w-]+\)/);
      }
    }
  });

  it("paints the GPUI set_theme values", () => {
    expect(hex(light().bg)).toBe("#faf8f5");
    expect(hex(light().accent)).toBe("#245bc7");
    expect(hex(light().hover)).toBe("#e8e5e0");
    expect(hex(light().selected)).toBe("#dfdcd6");
    expect(hex(osDark().bg)).toBe("#202123");
    expect(hex(osDark().accent)).toBe("#90b9ff");
    expect(hex(osDark().hover)).toBe("#303134");
    expect(hex(osDark().selected)).toBe("#37393d");
    expect(light().selection).toBe("rgb(var(--gpui-light-primary-rgb) / 0.28)");
    expect(osDark().selection).toBe("rgb(var(--gpui-dark-primary-rgb) / 0.28)");
  });

  it("keeps text, muted text, links, and button text at WCAG AA in both schemes", () => {
    for (const scheme of [light(), osDark()]) {
      const bg = hex(scheme.bg);
      expect(contrast(hex(scheme.muted), bg)).toBeGreaterThanOrEqual(4.5);
      expect(contrast(hex(scheme.muted), hex(scheme.panel))).toBeGreaterThanOrEqual(4.5);
      expect(contrast(hex(scheme.text), bg)).toBeGreaterThanOrEqual(4.5);
      expect(contrast(hex(scheme.accent), bg)).toBeGreaterThanOrEqual(4.5);
      expect(contrast(hex(scheme["on-accent"]), hex(scheme.accent))).toBeGreaterThanOrEqual(4.5);
    }
  });
});
