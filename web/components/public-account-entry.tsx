import Link from "next/link";
import { ACCOUNT_ENTRY_COPY } from "@/lib/content/account-entry";
import { pickText } from "@/lib/i18n/dictionaries";
import {
  CANONICAL_MARK_SRC,
  publicAuthAppDestination,
  type PublicAuthKind,
} from "@/lib/public-auth-routes";

export function PublicAccountEntry({
  locale,
  kind,
}: {
  locale: string;
  kind: Exclude<PublicAuthKind, "callback">;
}) {
  const creating = kind === "sign-up";
  const copy = creating ? ACCOUNT_ENTRY_COPY.signUp : ACCOUNT_ENTRY_COPY.signIn;
  const appHref = publicAuthAppDestination(kind, locale);
  const otherHref = `/${locale}/${creating ? "signin" : "signup"}`;

  return (
    <div className="portal-home">
      <section className="portal-section">
        <div className="portal-container public-account-entry">
          {/* Pinned app-icon raster generated from brand/mark.svg; do not restyle. */}
          {/* eslint-disable-next-line @next/next/no-img-element */}
          <img
            className="public-account-mark"
            src={CANONICAL_MARK_SRC}
            alt=""
            width={64}
            height={64}
          />
          <p className="legal-doc-kicker">{pickText(copy.kicker, locale)}</p>
          <h1>{pickText(copy.title, locale)}</h1>
          <p className="portal-lede">{pickText(ACCOUNT_ENTRY_COPY.lede, locale)}</p>
          <div className="portal-actions">
            <a className="portal-button portal-button-primary" href={appHref} data-usage={creating ? "signup" : "login"}>
              {pickText(copy.action, locale)}
            </a>
            <Link className="portal-button portal-button-secondary" href={`/${locale}/install`}>
              {pickText(ACCOUNT_ENTRY_COPY.installLocally, locale)}
            </Link>
          </div>
          <p className="portal-meta">
            {pickText(copy.switchPrompt, locale)}{" "}
            <Link href={otherHref} className="body-link">
              {pickText(copy.switchLabel, locale)}
            </Link>
          </p>
        </div>
      </section>
    </div>
  );
}
