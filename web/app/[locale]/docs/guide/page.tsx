import Link from "next/link";
import { GettingStartedSteps } from "@/components/getting-started-steps";
import { SessionMedia } from "@/components/session-media";
import { GUIDE_NEXT_LINKS } from "@/lib/content/getting-started";
import { getMediaAsset } from "@/lib/media-manifest";
import { buildPageMetadata } from "@/lib/page-meta";

export async function generateMetadata({ params }: { params: Promise<{ locale: string }> }) {
  const { locale } = await params;
  const isZh = locale === "zh";
  return buildPageMetadata({
    path: "/docs/guide",
    locale,
    title: isZh ? "新手指引 · Codewhale 文档" : "Getting started · Codewhale Docs",
    description: isZh
      ? "从安装到第一个 Fleet Workflow 的完整路径：安装、无需密钥的首次会话、连接提供商、运行 Fleet。"
      : "The full path from install to a first Fleet workflow: install, a first keyless session, provider connection, and Fleet.",
  });
}

export default async function GuidePage({ params }: { params: Promise<{ locale: string }> }) {
  const { locale } = await params;
  const isZh = locale === "zh";
  const bodyClass = isZh
    ? "text-ink-soft leading-[1.9] tracking-wide"
    : "text-ink-soft leading-relaxed";
  const session = getMediaAsset("first-fleet-session");

  return (
    <section className="space-y-10">
      <section id="overview" className="scroll-mt-32">
        <h2 className="font-display text-3xl mb-1">{isZh ? "新手指引" : "Getting started"}</h2>
        <p className={`${bodyClass} mt-3`}>
          {isZh
            ? "从一条安装命令到第一个 Fleet Workflow，四步走完。每一步都只陈述当前版本真实的行为；未发布或待录制的内容会明确标注。"
            : "Four steps from one install command to a first Fleet workflow. Every step states only what the current candidate actually does; anything unreleased or unrecorded is labeled as such."}
        </p>
      </section>

      <section id="path" className="scroll-mt-32">
        <GettingStartedSteps locale={locale} />
      </section>

      {session && (
        <section id="session-media" className="scroll-mt-32">
          <h2 className="font-display text-2xl mb-1">
            {isZh ? "看一次真实会话" : "Watch a real session"}
          </h2>
          <p className={`${bodyClass} mt-3 mb-4`}>
            {isZh
              ? "下方是真实会话媒体位。它当前处于待录制状态——这是有意为之：在 v0.9.2 候选版 dogfood 录制完成前，本站不展示任何占位或摆拍影像。"
              : "Below is the real-session media slot. It is deliberately in the pending state: until the v0.9.2 candidate dogfood recording exists, this site shows no placeholder or staged footage."}
          </p>
          <SessionMedia asset={session} locale={locale} />
        </section>
      )}

      <section id="next" className="scroll-mt-32">
        <h2 className="font-display text-2xl mb-1">{isZh ? "接下来" : "Where next"}</h2>
        <div className="hairline-t mt-4">
          {GUIDE_NEXT_LINKS.map((item) => (
            <div key={item.href} className="py-4 hairline-b">
              <h3 className="font-display text-xl">
                <Link href={`/${locale}${item.href}`} className="hover:text-indigo transition-colors">
                  {isZh ? item.label.zh : item.label.en}
                </Link>
              </h3>
              <p className={`${bodyClass} mt-1 text-sm`}>{isZh ? item.note.zh : item.note.en}</p>
            </div>
          ))}
        </div>
      </section>

      <section id="source" className="hairline-t pt-8">
        <p className="text-sm text-ink-mute">
          {isZh
            ? "来源文档：docs/GUIDE.md, docs/KEYBINDINGS.md · 步骤文案来自 web/lib/content/getting-started.ts；更新时请同步修改 docs-map.ts。"
            : "Source documents: docs/GUIDE.md, docs/KEYBINDINGS.md · Step copy lives in web/lib/content/getting-started.ts; update docs-map.ts when changing."}
        </p>
      </section>
    </section>
  );
}
