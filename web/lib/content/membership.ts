/**
 * Public membership copy is intentionally separate from the billing catalog.
 * The public site may describe what is usable now, but it must not promote a
 * price or allowance until that commercial term has its own approval.
 */

import type { LocalizedText } from "./vocabulary";

export interface PublicMembershipOption {
  id: "account" | "local" | "paid";
  title: LocalizedText;
  body: LocalizedText;
}

/** Machine-checkable launch state; copy tests assert these truths, not wording. */
export const PUBLIC_MEMBERSHIP_STATUS = {
  checkout: "dormant",
  paymentFromPage: false,
  localUseRequiresPaidMembership: false,
  commercialTerms: "not-published",
} as const;

export const PUBLIC_MEMBERSHIP_COPY = {
  metadata: {
    title: { en: "Pricing · Codewhale", zh: "价格 · Codewhale" },
    description: {
      en: "Install Codewhale free and use your own model provider. Explore local use and account options.",
      zh: "免费安装 Codewhale，使用你选择的模型提供商。了解本地使用与账户选项。",
    },
  },
  kicker: { en: "Pricing", zh: "价格" },
  title: {
    en: "Use Codewhale with your own models.",
    zh: "用你自己的模型使用 Codewhale。",
  },
  lead: {
    en: "Install Codewhale free and connect your model provider. Create an account to explore the web app development preview.",
    zh: "免费安装 Codewhale，连接模型提供商。创建账户后可体验网页应用开发预览版。",
  },
  options: [
    {
      id: "account",
      title: { en: "Codewhale account", zh: "Codewhale 账户" },
      body: {
        en: "Keep your conversations and work together in the app.",
        zh: "在应用中集中管理对话与工作。",
      },
    },
    {
      id: "local",
      title: { en: "Run locally", zh: "本地运行" },
      body: {
        en: "Install the open-source Codewhale runtime and connect your provider credentials. Local use is free.",
        zh: "安装开源 Codewhale 运行时，连接你的提供商凭据。本地使用免费。",
      },
    },
    {
      id: "paid",
      title: { en: "Paid plans", zh: "付费方案" },
      body: {
        en: "Paid plans are not available yet.",
        zh: "付费方案暂未开放。",
      },
    },
  ] satisfies PublicMembershipOption[],
  note: {
    en: "Hosted model usage is billed by your provider.",
    zh: "托管模型用量由模型提供商计费。",
  },
  actions: {
    createAccount: { en: "Create account", zh: "创建账户" },
    continueLocally: { en: "Install Codewhale", zh: "安装 Codewhale" },
    signIn: { en: "Already have an account? Sign in", zh: "已有账户？登录" },
  },
} as const;
