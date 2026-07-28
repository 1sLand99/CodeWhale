import Link from "next/link";
import { GITEE_ENABLED, type Locale } from "@/lib/i18n/config";
import { getChrome } from "@/lib/i18n/dictionaries";
import { Whale } from "./whale";

export function Footer({ locale = "en" }: { locale?: Locale }) {
  const isZh = locale === "zh";
  // en/zh stay inline (copy-contract tests read them from this file); every
  // other routed locale resolves its chrome from the dictionary layer with
  // English as the fallback.
  const chrome = getChrome(locale);
  const homeHref = `/${locale}`;

  const product = isZh
    ? [
        { label: "文档", href: "/zh/docs" },
        { label: "新手指引", href: "/zh/docs/guide" },
        { label: "安装", href: "/zh/install" },
        { label: "模型", href: "/zh/models" },
        { label: "运行时", href: "/zh/runtime" },
        { label: "常见问题", href: "/zh/faq" },
      ]
    : locale === "en"
      ? [
          { label: "Docs", href: "/en/docs" },
          { label: "Getting started", href: "/en/docs/guide" },
          { label: "Install", href: "/en/install" },
          { label: "Models", href: "/en/models" },
          { label: "Runtime", href: "/en/runtime" },
          { label: "FAQ", href: "/en/faq" },
        ]
      : [
          // No `footerGuide`/`footerFaq` keys exist in the chrome dictionaries,
          // so the dictionary-driven locales keep main's link set rather than
          // linking English-only labels. Adding them is a locale-parity change,
          // not a rebase resolution.
          { label: chrome.footerDocs, href: `/${locale}/docs` },
          { label: chrome.footerInstall, href: `/${locale}/install` },
          { label: chrome.footerModels, href: `/${locale}/models` },
          { label: chrome.footerRuntime, href: `/${locale}/runtime` },
        ];

  const project = isZh
    ? [
        { label: "GitHub", href: "https://github.com/Hmbown/CodeWhale" },
        { label: "议题", href: "https://github.com/Hmbown/CodeWhale/issues" },
        { label: "参与贡献", href: "/zh/contribute" },
        { label: "MIT 许可证", href: "https://github.com/Hmbown/CodeWhale/blob/main/LICENSE" },
      ]
    : locale === "en"
      ? [
          { label: "GitHub", href: "https://github.com/Hmbown/CodeWhale" },
          { label: "Issues", href: "https://github.com/Hmbown/CodeWhale/issues" },
          { label: "Contribute", href: "/en/contribute" },
          { label: "MIT license", href: "https://github.com/Hmbown/CodeWhale/blob/main/LICENSE" },
        ]
      : [
          { label: "GitHub", href: "https://github.com/Hmbown/CodeWhale" },
          { label: chrome.footerIssues, href: "https://github.com/Hmbown/CodeWhale/issues" },
          { label: chrome.footerContribute, href: `/${locale}/contribute` },
          { label: chrome.footerLicense, href: "https://github.com/Hmbown/CodeWhale/blob/main/LICENSE" },
        ];

  const tagline = isZh
    ? "Codewhale 开源运行时的文档、源码与社区入口。"
    : chrome.footerTagline;
  const productHeading = isZh ? "产品" : chrome.footerProduct;
  const projectHeading = isZh ? "项目" : chrome.footerProject;
  const canonicalSource = isZh ? "官方源码：" : chrome.footerCanonicalSource;
  const releases = isZh ? " · 发布：" : chrome.footerReleases;

  return (
    <footer className="site-footer">
      <div className="site-footer-main">
        <div className="site-footer-brand">
          <Link href={homeHref} className="site-wordmark site-wordmark-footer">
            <Whale size={31} className="text-current" />
            <span>Codewhale</span>
          </Link>
          <p>{tagline}</p>
        </div>

        <div className="site-footer-links">
          <div>
            <span>{productHeading}</span>
            {product.map((item) => <Link key={item.href} href={item.href}>{item.label}</Link>)}
          </div>
          <div>
            <span>{projectHeading}</span>
            {project.map((item) => <Link key={item.href} href={item.href}>{item.label}</Link>)}
          </div>
        </div>
      </div>

      <div className="site-footer-meta">
        <p>
          {canonicalSource}
          <a href="https://github.com/Hmbown/CodeWhale">github.com/Hmbown/CodeWhale</a>
          {releases}
          <a href="https://github.com/Hmbown/CodeWhale/releases">GitHub Releases</a>
        </p>
        <div>
          {GITEE_ENABLED && <a href="https://gitee.com/Hmbown/CodeWhale">Gitee</a>}
          <a href="https://cnb.cool/codewhale.net/codewhale">CNB</a>
          <a href="https://npmmirror.com/package/codewhale">npmmirror</a>
          <a href="mailto:hmbown@gmail.com">Security</a>
        </div>
        <span>© {new Date().getFullYear()} Codewhale</span>
      </div>
    </footer>
  );
}
