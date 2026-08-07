"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { useEffect, useRef, useState } from "react";
import type { ChromeLink } from "@/lib/i18n/links";

export function MobileMenu({
  links,
  installHref,
  installLabel,
  openLabel,
  closeLabel,
  navAria,
}: {
  links: ChromeLink[];
  installHref: string;
  installLabel: string;
  openLabel: string;
  closeLabel: string;
  /** Accessible name for the dialog's navigation landmark. */
  navAria: string;
}) {
  const [open, setOpen] = useState(false);
  // `closing` holds the panel mounted for a short exit fade; the unmount —
  // and the focus hand-back in the effect below — then happens on the
  // timeout, not on the click.
  const [closing, setClosing] = useState(false);
  const closeTimer = useRef<number | null>(null);
  const pathname = usePathname();
  const toggleRef = useRef<HTMLButtonElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);

  const close = () => {
    // Reduced motion keeps the original instant mount/unmount.
    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
      setOpen(false);
      return;
    }
    if (closeTimer.current !== null) window.clearTimeout(closeTimer.current);
    setClosing(true);
    closeTimer.current = window.setTimeout(() => {
      closeTimer.current = null;
      setOpen(false);
      setClosing(false);
    }, 180);
  };

  const onToggle = () => {
    if (!open) {
      setOpen(true);
      return;
    }
    if (closing) {
      // Re-open mid-exit: cancel the pending unmount and stay open.
      window.clearTimeout(closeTimer.current ?? undefined);
      closeTimer.current = null;
      setClosing(false);
      return;
    }
    close();
  };

  useEffect(() => {
    if (!open) return;
    const prev = document.body.style.overflow;
    document.body.style.overflow = "hidden";
    // aria-modal promises the dialog owns interaction: move focus inside on
    // open, and hand it back to the toggle on close (Escape, link, or the
    // toggle itself — re-focusing an already-focused button is a no-op).
    // The toggle node is captured now: reading toggleRef.current inside the
    // cleanup would race React clearing the ref.
    const toggle = toggleRef.current;
    menuRef.current?.querySelector<HTMLElement>("a, button")?.focus();
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") close();
    };
    window.addEventListener("keydown", onKey);
    return () => {
      document.body.style.overflow = prev;
      window.removeEventListener("keydown", onKey);
      toggle?.focus();
    };
  }, [open]);

  // A pending exit timer must not outlive the component (locale switches
  // remount the nav).
  useEffect(() => {
    return () => {
      if (closeTimer.current !== null) window.clearTimeout(closeTimer.current);
    };
  }, []);

  return (
    <>
      <button
        ref={toggleRef}
        type="button"
        onClick={onToggle}
        className="md:hidden inline-flex items-center justify-center w-9 h-9 hairline-t hairline-b hairline-l hairline-r hover:bg-paper-deep transition-colors"
        aria-label={open ? closeLabel : openLabel}
        aria-expanded={open}
        aria-controls="mobile-menu"
      >
        {open ? (
          <svg width="14" height="14" viewBox="0 0 14 14" fill="none" aria-hidden>
            <path d="M2 2L12 12M12 2L2 12" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" />
          </svg>
        ) : (
          <svg width="16" height="12" viewBox="0 0 16 12" fill="none" aria-hidden>
            <path d="M0 1H16M0 6H16M0 11H16" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" />
          </svg>
        )}
      </button>

      {open && (
        <div
          ref={menuRef}
          id="mobile-menu"
          className={`mm-panel md:hidden fixed inset-x-0 top-[5.75rem] bottom-0 z-40 bg-paper hairline-t overflow-y-auto${closing ? " mm-closing" : ""}`}
          role="dialog"
          aria-modal="true"
          aria-label={navAria}
        >
          {/* Only one nav landmark is exposed at a time (the desktop nav is
              display:none at these widths), so the named dialog carries the
              landmark name and the inner nav stays unlabeled — two nested
              "Primary" landmarks would read as duplication. */}
          <nav className="px-6 py-4">
            <ul className="divide-y divide-[rgba(27,34,48,0.18)]">
              {links.map((l) => {
                const isActive = pathname === l.href || pathname.startsWith(`${l.href}/`);
                return (
                  <li key={l.href}>
                    <Link
                      href={l.href}
                      onClick={() => setOpen(false)}
                      className={`flex items-baseline gap-3 py-4 hover:text-indigo transition-colors ${isActive ? "text-indigo" : ""}`}
                      aria-current={isActive ? "page" : undefined}
                    >
                      <span className="font-display text-lg">{l.label}</span>
                      {l.secondary && (
                        <span className="font-cjk text-sm text-ink-mute">{l.secondary}</span>
                      )}
                      <span className="ml-auto font-mono text-xs text-ink-mute">→</span>
                    </Link>
                  </li>
                );
              })}
            </ul>

            <Link
              href={installHref}
              onClick={() => setOpen(false)}
              className="mt-6 block w-full text-center px-5 py-3 bg-indigo text-paper font-mono text-sm uppercase tracking-wider hover:bg-indigo-deep transition-colors"
            >
              {installLabel}
            </Link>
          </nav>
        </div>
      )}
    </>
  );
}
