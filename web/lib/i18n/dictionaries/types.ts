/**
 * Dictionary shapes for the website localization layer (#3091).
 *
 * `ChromeDict` covers shared chrome: nav, footer, locale switcher, and the
 * visible partial-pack badge. `HomeDict` covers the landing page
 * (`app/[locale]/page.tsx`). Templates use `{name}` tokens interpolated
 * with `fill()` from dictionaries/index.ts — never concatenate translated
 * sentences around variables in JSX.
 *
 * English (`dictionaries/en/`) is the reference shape; every shipped or
 * partial locale must define exactly the same keys. Parity is enforced by
 * `web/scripts/check-locales.mjs` and `web/lib/i18n/dictionaries.test.ts`.
 * Missing locales fall back to the English dictionary at lookup time, so
 * an untranslated string renders English copy — never a dictionary key.
 */

export interface ChromeDict {
  navDocs: string;
  navInstall: string;
  navCommunity: string;
  navContribute: string;
  /** Mobile-menu call to action, e.g. "Install →". */
  installCta: string;
  footerTagline: string;
  footerProduct: string;
  footerProject: string;
  footerDocs: string;
  footerInstall: string;
  footerModels: string;
  footerRuntime: string;
  footerIssues: string;
  footerContribute: string;
  footerLicense: string;
  /** Prefix before the canonical-source link, e.g. "Canonical source: ". */
  footerCanonicalSource: string;
  /** Separator + label before the releases link, e.g. " · Releases: ". */
  footerReleases: string;
  /** aria-label for the locale switcher control. */
  switcherLabel: string;
  /**
   * Visible badge marking a partial locale pack in the switcher, e.g.
   * "(partial)" — honest scope signal, per the localization quality
   * contract. Keep it short.
   */
  partialBadge: string;
}

export interface HomeDict {
  kicker: string;
  heroTitleA: string;
  heroTitleB: string;
  heroIntro: string;
  install: string;
  docs: string;
  copy: string;
  copied: string;
  /** "Latest release {tag}" */
  latestRelease: string;
  releaseUnavailable: string;
  /** "Current source" / "Source candidate" — prepended to `v{version}:`. */
  currentSource: string;
  sourceCandidate: string;
  /** "{count} provider routes" */
  providerRoutes: string;
  /** Screenshot alt: "Fresh Codewhale v{version} terminal session …" */
  screenshotAlt: string;
  /** "v{version} {state} · local Ollama route · Plan / Act / Operate" */
  figcaption: string;
  /** "published release" / "source candidate" — used inside figcaption. */
  publishedRelease: string;
  figcaptionSourceCandidate: string;
  proofHeading: string;
  proofBody: string;
  workflowHeading: string;
  /** Four [title, description] steps. */
  workflow: [string, string][];
  receiptAria: string;
  boundariesHeadingA: string;
  boundariesHeadingB: string;
  boundariesBody: string;
  hostedGatewayLocal: string;
  planActOperateDesc: string;
  askAutoReviewDesc: string;
  tuiExecWebDesc: string;
  surfacesHeading: string;
  /** Five [name, description] surfaces. */
  surfaces: [string, string][];
  runtimeLink: string;
  installBandHeading: string;
  binaries: string;
  chinaMirrors: string;
  installGuideLink: string;
  communityHeading: string;
  communityBody: string;
  communityLinksAria: string;
  contribute: string;
}
