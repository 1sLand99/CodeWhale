/**
 * usage-consent.ts — the disclosure behind the website's usage counting.
 *
 * Names exactly what is counted, where it goes (Codewhale's own endpoint,
 * which may pass aggregate counts to PostHog as a processor), and what never
 * leaves the browser. The choice is explicit and reversible from the footer.
 */

import type { LocalizedText } from "./vocabulary";

export const USAGE_CONSENT_COPY = {
  title: { en: "Count anonymous usage?", zh: "是否统计匿名使用量？" },
  summary: { en: "Optional usage totals help improve Codewhale. No page addresses or content. Counting stays off until you allow it.", zh: "可选的使用次数统计有助于改进 Codewhale，不包含页面地址或内容。只有你允许后才开启。" },
  details: { en: "What gets counted and where it goes", zh: "统计哪些数据，发送到哪里" },
  body: {
    en: "Codewhale can count page views, docs views, install copies, and downloads on this site. Only those totals leave your browser — no page addresses, referrers, account, or content — sent to Codewhale's own endpoint, which may pass the totals to PostHog. Off unless you say yes; change your mind any time from the footer.",
    zh: "Codewhale 可以统计本站的页面浏览、文档浏览、安装命令复制和下载次数。离开浏览器的只有这些总数——没有页面地址、来源、账户或内容——发送到 Codewhale 自己的端点，该端点可能把总数交给 PostHog 处理。默认关闭，只有你同意才开启；随时可在页脚更改。",
  },
  allow: { en: "Allow counting", zh: "允许统计" },
  decline: { en: "No thanks", zh: "不用了" },
  footerLink: { en: "Usage data", zh: "使用数据" },
  status: {
    granted: { en: "Anonymous usage counting is on.", zh: "匿名使用统计已开启。" },
    declined: { en: "Anonymous usage counting is off.", zh: "匿名使用统计已关闭。" },
    undecided: { en: "Anonymous usage counting is off until you allow it.", zh: "匿名使用统计默认关闭，需你允许后才开启。" },
  },
  turnOff: { en: "Turn off", zh: "关闭" },
  turnOn: { en: "Turn on", zh: "开启" },
  close: { en: "Close", zh: "关闭对话框" },
  dialogAria: { en: "Usage data preference", zh: "使用数据偏好" },
} as const satisfies Record<string, LocalizedText | Record<string, LocalizedText>>;
