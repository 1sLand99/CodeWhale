/**
 * The "/" focus-search shortcut shared by the FAQ and docs search boxes.
 *
 * A single-character shortcut must not steal keys the user is typing
 * elsewhere (WCAG 2.1.4 Character Key Shortcuts): it ignores modified keys,
 * IME composition, and any event aimed at an editable control.
 */

type ShortcutTarget = {
  tagName?: string;
  isContentEditable?: boolean;
} | null;

export type ShortcutKeyEvent = Pick<
  KeyboardEvent,
  "key" | "ctrlKey" | "metaKey" | "altKey" | "isComposing"
> & { target: EventTarget | ShortcutTarget | null };

const EDITABLE_TAGS = new Set(["INPUT", "TEXTAREA", "SELECT"]);

export function isEditableTarget(target: EventTarget | ShortcutTarget | null): boolean {
  if (!target || typeof target !== "object") return false;
  const el = target as ShortcutTarget & object;
  if (el.isContentEditable) return true;
  return typeof el.tagName === "string" && EDITABLE_TAGS.has(el.tagName.toUpperCase());
}

export function isSlashShortcut(e: ShortcutKeyEvent): boolean {
  if (e.key !== "/") return false;
  if (e.ctrlKey || e.metaKey || e.altKey || e.isComposing) return false;
  return !isEditableTarget(e.target);
}
