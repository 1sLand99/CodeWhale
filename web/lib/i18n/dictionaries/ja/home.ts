import type { HomeDict } from "../types";

/**
 * Japanese home dictionary — native copy for the Tidal Folio landing page,
 * in the current direction: your models, more capable together; agents
 * and control on your own machine; availability stated per surface as it
 * is today. Product vocabulary stays literal (Plan / Work / Operate, Ask /
 * Auto-Review / Full Access, Codewhale, TUI, codewhale exec, Fleet).
 */

export const home: HomeDict = {
  metaTitle: "Codewhale — 作りたいものを、形に。",
  metaDescription:
    "Codewhale でソフトウェアを作り、ファイルを扱い、タスクを自動化。ホスト型やローカルのモデルを選び、作業に合わせてプロバイダーを切り替えられます。",
  kicker: "オープンソースの AI エージェント",
  heroTitleA: "作りたいものを、形に。",
  heroTitleB: "使うモデルは、自分で選ぶ。",
  heroIntro:
    "{brand} のエージェントは、ソフトウェアの開発、ファイルの操作、タスクの自動化を手伝います。好きなモデルを選び、作業に合わせてプロバイダーを切り替えられます。",
  getCodewhale: "Codewhale を入手",
  exploreProduct: "製品を見る",
  shotPreview: "ターミナルのプレビュー",
  shotBuild: "v{version} 開発ビルド",
  screenshotAlt:
    "ターミナル上の Codewhale v0.9.12 開発ビルド：点字で描かれたクジラのマーク、履歴のない新規セッション、メッセージ入力欄、そして Full Access、Work モード、予定タスク 2 件、MCP サーバー接続中、GLM-5.3 最大強度を示すフッター",
  latestRelease: "最新リリース {tag}",
  releaseUnavailable: "リリース情報を取得できません",
  currentSource: "ソース",
  sourceCandidate: "未リリース",
  providerRoutes: "{count} プロバイダー",
  publishedRelease: "リリース済み",
  figcaptionSourceCandidate: "未リリース",
  chapterTerminal: "あなたのターミナル",
  chapterTerminalTitle: "作りたいものから始めよう。",
  gainHeading: "アイデアを動かそう。",
  gainLede: "プロジェクトを作る、疑問を調べる、タスクを自動化する。まずはひとつのエージェントで始め、大きな仕事は複数のエージェントで分担できます。",
  gain: [
    [
      "作ってみよう",
      "アイデアを動くソフトウェアに。エージェントがファイルを編集し、コマンドを実行して、結果を確認できます。"
    ],
    [
      "繰り返す作業を自動化",
      "繰り返し行うタスクのスクリプトやワークフローを作り、ターミナルから実行できます。"
    ],
    [
      "使うモデルを選ぶ",
      "ホスト型やローカルのモデルに接続。大きな仕事を、異なるモデルや役割を持つエージェントに分担させられます。"
    ]
  ],
  chapterModels: "あなたのモデル",
  modelsHeading: "タスクに合うモデルを見つけよう。",
  modelsBody:
    "ホスト型プロバイダーの利用、ゲートウェイ経由の接続、ローカルでのモデル実行に対応。セッションごとにプロバイダーとモデルを選び、作業中にも変更できます。",
  modelsFacts: [
    ["ホスト型", "自分の API キーを codewhale auth set --provider <id> で保存"],
    ["ゲートウェイ", "ひとつのエンドポイントで多くのモデル、プロバイダーは自分で選ぶ"],
    ["ローカル", "localhost 上の vLLM、SGLang、Ollama。通常キー不要"],
  ],
  modelsLink: "モデルとプロバイダーを見る",
  startHeading: "最初のタスクを始めよう。",
  startLede: "Codewhale をインストールしてモデルを接続し、やりたいことを伝えてください。複数のエージェントで分担したいときは、Fleet を追加できます。",
  startGuideLink: "はじめかたガイドを読む",
  startVocabularyLink: "製品用語を見る",
  chapterAccount: "Codewhale を入手",
  availabilityHeading: "Codewhale を使える場所。",
  availabilityLede: "まずはターミナルから。アプリとクラウドコンピューターは開発中です。",
  availability: [
    [
      "ターミナル",
      "リリース済み",
      "Linux、macOS、Windows 向けのリリースバイナリを GitHub で提供しています。npm と Cargo からもインストールできます。Android の Termux 版はプレビューです。"
    ],
    [
      "ウェブアプリ",
      "開発プレビュー",
      "開発プレビューでアカウントへのアクセスとブラウザのペアリングを利用できます。"
    ],
    [
      "デスクトップ",
      "開発ビルド",
      "macOS アプリは開発中です。一般向けのダウンロードは後日提供予定です。"
    ],
    [
      "クラウドコンピューター",
      "開発中",
      "タスクを実行するためのホスト型コンピューター。"
    ]
  ],
  availabilityNote: "ターミナルは Codewhale アカウントなしで使えます。ホスト型モデルの利用料金は、契約先のプロバイダーから請求されます。",
  accountLink: "アカウントを作成",
  surfacesHeading: "作業のある場所で、そのままランタイムを使う。",
  surfaces: [
    ["TUI", "対話型のターミナル作業"],
    ["codewhale exec", "スクリプトと CI"],
    ["ローカル Web クライアント","localhost のインターフェース。ホスト型のブラウザ作業環境は開発中"],
    ["Runtime API + MCP", "ローカル連携"],
    ["Fleet","複数のエージェントでひとつの仕事に取り組む"],
  ],
  runtimeLink: "連携機能を見る",
  installBandHeading: "コマンド 1 つで始める。",
  copy: "コピー",
  copied: "コピー済み ✓",
  binaries: "バイナリ",
  chinaMirrors: "中国ミラー",
  installGuideLink: "インストールガイドを読む",
  communityHeading: "公開の場でつくる",
  communityBody: "MIT ライセンス。ランタイム、プロバイダー、プラットフォーム、ドキュメント、テストにまたがる貢献者たちの手で形づくられています。",
  communityLinksAria: "コミュニティリンク",
  contribute: "貢献する",
};
