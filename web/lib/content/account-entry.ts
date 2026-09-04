/**
 * account-entry.ts — copy for the public sign-in and create-account pages.
 *
 * codewhale.net is not the signed-in app; these pages send the person to
 * app.codewhale.net while saying plainly that the terminal needs no account
 * and that an account is never a paid plan by itself.
 */

import type { LocalizedText } from "./vocabulary";

export const ACCOUNT_ENTRY_COPY = {
  signIn: {
    kicker: { en: "Sign in", zh: "登录" },
    title: { en: "Sign in to Codewhale.", zh: "登录 Codewhale 账户。" },
    action: { en: "Sign in", zh: "登录" },
    switchPrompt: { en: "Need an account?", zh: "还没有账户？" },
    switchLabel: { en: "Create account", zh: "创建账户" },
  },
  signUp: {
    kicker: { en: "Create account", zh: "创建账户" },
    title: { en: "Create a Codewhale account.", zh: "创建 Codewhale 账户。" },
    action: { en: "Create account", zh: "创建账户" },
    switchPrompt: { en: "Already have an account?", zh: "已有账户？" },
    switchLabel: { en: "Sign in", zh: "去登录" },
  },
  lede: {
    en: "An account is for the web app, where you sign in, keep provider keys with the account, and use the browser workbench that is still a development preview. The open-source terminal works locally without one — install it and continue on your machine. An account is never a paid plan by itself.",
    zh: "账户用于网页应用：登录、把提供商密钥保存在账户里，以及使用仍处于开发预览的浏览器工作台。开源终端无需账户即可在本机使用——安装后即可继续。账户本身从不等于付费方案。",
  },
  installLocally: { en: "Install locally", zh: "本机安装" },
} as const satisfies Record<string, unknown> & { lede: LocalizedText; installLocally: LocalizedText };
