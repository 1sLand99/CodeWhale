import type { Metadata } from "next";
import { IBM_Plex_Sans, IBM_Plex_Sans_Condensed, JetBrains_Mono, Newsreader } from "next/font/google";
import { Nav } from "@/components/nav";
import { Footer } from "@/components/footer";
import { UsageCounting } from "@/components/usage-counting";
import { BUILD_FACTS } from "@/lib/facts";
import { localeDirection, locales, type Locale } from "@/lib/i18n/config";
import { getChrome, getHome } from "@/lib/i18n/dictionaries";
import { serializeJsonLd } from "@/lib/json-ld";
import { buildPageMetadata } from "@/lib/page-meta";
import { buildSiteJsonLd } from "@/lib/site-schema";
import "../globals.css";

// Type stacks resolve in globals.css: Newsreader carries the folio's display
// voice (h1/h2), IBM Plex Sans Condensed the small headings, IBM Plex Sans
// the body, JetBrains Mono the terminal.
const body = IBM_Plex_Sans({
  subsets: ["latin", "cyrillic", "vietnamese"],
  weight: ["400", "500", "600"],
  variable: "--font-body",
  display: "swap",
});

const mono = JetBrains_Mono({
  subsets: ["latin", "cyrillic"],
  weight: ["400", "500", "600"],
  variable: "--font-mono",
  display: "swap",
});

const display = IBM_Plex_Sans_Condensed({
  subsets: ["latin"],
  weight: ["500", "600"],
  variable: "--font-display",
  display: "swap",
});

// Newsreader's optical-size axis is what lets the same face set a 5rem title
// and a 1.3rem running head without looking like two fonts.
const serif = Newsreader({
  subsets: ["latin", "latin-ext"],
  weight: ["400", "500"],
  style: ["normal", "italic"],
  variable: "--font-serif",
  display: "swap",
});

export function generateStaticParams() {
  return locales.map((locale) => ({ locale }));
}

export async function generateMetadata({ params }: { params: Promise<{ locale: string }> }): Promise<Metadata> {
  const { locale } = await params;
  const home = getHome(locale);
  return buildPageMetadata({
    path: "/",
    locale,
    title: home.metaTitle,
    description: home.metaDescription,
  });
}

export default async function LocaleLayout({
  children,
  params,
}: {
  children: React.ReactNode;
  params: Promise<{ locale: string }>;
}) {
  const { locale } = await params;
  const chrome = getChrome(locale);
  // RTL locales (e.g. ar) set the document direction from the canonical
  // registry so the browser handles bidirectional layout from the root.
  const dir = localeDirection(locale);
  const siteJsonLd = buildSiteJsonLd(locale);

  return (
    <html
      lang={locale}
      dir={dir}
      className={`${body.variable} ${mono.variable} ${display.variable} ${serif.variable}`}
      suppressHydrationWarning
    >
      <body>
        <script
          type="application/ld+json"
          dangerouslySetInnerHTML={{ __html: serializeJsonLd(siteJsonLd) }}
        />
        {/* Apply the persisted docs theme before paint so there is no flash.
            The site default is the paper sheet; only an explicit "dark"
            choice re-themes the docs subtree to the whale's stage. */}
        <script
          dangerouslySetInnerHTML={{
            __html:
              "(function(){try{var t=localStorage.getItem('cw-theme');if(t==='light'||t==='dark'){document.documentElement.setAttribute('data-theme',t);}}catch(e){}})();",
          }}
        />
        <a href="#main-content" className="skip-link">
          {chrome.skipToContent}
        </a>
        <Nav locale={locale as Locale} />
        <main id="main-content">{children}</main>
        <Footer locale={locale as Locale} />
        {/* Aggregate usage counting, on by default — see lib/telemetry. The
            choice lives on the privacy page; every opt-out stays off. */}
        <UsageCounting appVersion={BUILD_FACTS.version ?? "0.0.0"} />
      </body>
    </html>
  );
}
