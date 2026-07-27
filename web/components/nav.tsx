import Link from "next/link";
import type { Locale } from "@/lib/i18n/config";
import { getChrome } from "@/lib/i18n/dictionaries";
import { LocaleSwitcher } from "./locale-switcher";
import { MobileMenu } from "./mobile-menu";
import { NavLinks } from "./nav-links";
import { ThemeToggle } from "./theme-toggle";
import { Whale } from "./whale";

const EN_LINKS = [
  { href: "/en/docs", label: "Docs" },
  { href: "/en/install", label: "Install" },
  { href: "/en/community", label: "Community" },
  { href: "/en/contribute", label: "Contribute" },
];

const ZH_LINKS = [
  { href: "/zh/docs", label: "文档" },
  { href: "/zh/install", label: "安装" },
  { href: "/zh/community", label: "社区" },
  { href: "/zh/contribute", label: "贡献" },
];

export function Nav({ locale = "en" }: { locale?: Locale }) {
  const isZh = locale === "zh";
  // en/zh stay inline (copy-contract tests read them from this file); every
  // other routed locale resolves its chrome from the dictionary layer with
  // English as the fallback.
  const chrome = getChrome(locale);
  const links = isZh
    ? ZH_LINKS
    : locale === "en"
      ? EN_LINKS
      : [
          { href: `/${locale}/docs`, label: chrome.navDocs },
          { href: `/${locale}/install`, label: chrome.navInstall },
          { href: `/${locale}/community`, label: chrome.navCommunity },
          { href: `/${locale}/contribute`, label: chrome.navContribute },
        ];
  const homeHref = `/${locale}`;
  const installCta = isZh ? "安装 →" : chrome.installCta;

  return (
    <header className="site-nav">
      <div className="site-nav-inner">
        <Link href={homeHref} className="site-wordmark" aria-label="Codewhale home">
          <Whale size={31} className="text-current" />
          <span>Codewhale</span>
        </Link>

        <NavLinks links={links} isZh={isZh} />

        <div className="site-nav-actions">
          <ThemeToggle isZh={isZh} />
          <LocaleSwitcher current={locale} />
          <Link href="https://github.com/Hmbown/CodeWhale" className="site-github-link">
            GitHub
          </Link>
          <MobileMenu
            installHref={`/${locale}/install`}
            installLabel={installCta}
            links={links}
          />
        </div>
      </div>
    </header>
  );
}
