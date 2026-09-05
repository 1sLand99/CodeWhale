import type { HomeDict } from "../types";

/**
 * Japanese home dictionary — native copy for the Tidal Folio landing page,
 * in the current direction: your models, more capable together; agents
 * and control on your own machine; availability stated per surface as it
 * is today. Product vocabulary stays literal (Plan / Work / Operate, Ask /
 * Auto-Review / Full Access, Codewhale, TUI, codewhale exec, Fleet).
 */

export const home: HomeDict = {
  metaTitle: "Codewhale — あなたのモデルを、いっしょに、より強く。",
  metaDescription:
    "Codewhale はオープンソースのエージェント型コンピューティング・システムです。すでに使っているモデル（ホスト型、ゲートウェイ、ローカル）をターミナルに持ち込み、あなたのマシン上で、あなたの管理のもと協働させます。Rust 製、MIT。",
  kicker: "エージェント型コンピューティングを、あなたの条件で",
  heroTitleA: "あなたのモデルを、",
  heroTitleB: "いっしょに、より強く。",
  heroIntro:
    "{brand} は、すでに使っているモデルをひとつのターミナルに集め、クルーのように協働させます。コードを読み、編集し、チェックを走らせる。各モデルに何を許すかは、あなたが決めます。オープンソースで、あなたのマシン上で動きます。",
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
  chapterTerminalTitle: "慣れた場所から始める。",
  gainHeading: "手に入るのはチャットボットではなく、すでに支払っているモデルへのレバレッジです。",
  gainLede: "ひとつのセッションで複数のモデルを同時に持ち、それぞれに役割を与え、同じリポジトリで同じルールのもと働かせられます。",
  gain: [
    ["あなたのモデル", "ホスト型のキー、ゲートウェイ、あるいはキー不要のローカル・ランタイム。役割ごとに別のモデルを固定でき、選んだプロバイダーはそのまま。モデル名がプロバイダーを勝手に切り替えることはありません。"],
    ["有能なエージェント", "Plan、Work、Operate の各モード。ひとつの仕事に取り組むサブエージェントの fleet。ファイル、シェル、ウェブ、MCP のツール。保存・再開・ロールバックできるセッション。"],
    ["あなたのマシンでの制御", "Ask、Auto-Review、Full Access。確認前にどこまで進めるかはあなたが決めます。ローカルで動き、OS が許す範囲でサンドボックス化され、監査ログはいつでも読めます。"],
  ],
  chapterModels: "あなたのモデル",
  modelsHeading: "持っているものを持ち込む。選んでいないものは変えない。",
  modelsBody:
    "Codewhale には {count} のプロバイダーが同梱され、どれも対等に扱われます。キーを一度保存し、モデル名を指定すれば、ルートは設定したとおりに保たれます。vLLM、SGLang、Ollama 上のローカルモデルにキーは不要です。",
  modelsFacts: [
    ["ホスト型", "自分の API キーを codewhale auth set で保存"],
    ["ゲートウェイ", "ひとつのエンドポイントで多くのモデル、プロバイダーは自分で選ぶ"],
    ["ローカル", "localhost 上の vLLM、SGLang、Ollama。通常キー不要"],
  ],
  modelsLink: "すべてのプロバイダーを見る",
  startHeading: "最初のセッションまで四つのステップ。",
  startLede: "インストールし、キーなしでセッションを開き、プロバイダーを接続。モデルひとつでは足りなくなったら fleet を設定します。",
  startGuideLink: "はじめかたガイドを読む",
  startVocabularyLink: "製品用語を見る",
  chapterAccount: "いま動く場所",
  availabilityHeading: "利用可能、開発中、まだ未提供。ありのままに。",
  availabilityLede: "ターミナルがリリース済みの製品です。それ以外は実際の状態のまま並べています。",
  availability: [
    ["ターミナル", "リリース済み", "Linux・macOS・Windows 向けの GitHub Releases バイナリを推奨。npm と Cargo も利用できます。Android の Termux はプレビューです。"],
    ["ウェブアプリ", "サインインとリモートコントロールが利用可能", "サインインまたはアカウント作成のうえ、実行中のローカルセッションで /rc と入力すると、その同じセッションをブラウザから続けられます。ブラウザのワークベンチのそれ以外は開発プレビューです。"],
    ["デスクトップ", "開発ビルド", "macOS、Linux、Windows 向けのアルファ版があります。リリース済みのデスクトップアプリはまだありません。"],
    ["クラウドコンピューター", "まだ利用できません", "ホストされたコンピューター上で作業を実行する機能は開発中です。動くようになったら、このページでそう伝えます。"],
  ],
  availabilityNote: "ターミナルにアカウントは不要です。アカウントがそれ自体で有料プランになることはなく、このサイトから課金されることはありません。",
  accountLink: "アカウントを作成",
  surfacesHeading: "作業のある場所で、そのままランタイムを使う。",
  surfaces: [
    ["TUI", "対話型のターミナル作業"],
    ["codewhale exec", "スクリプトと CI"],
    ["Web クライアント", "ループバック限定のブラウザクライアント"],
    ["Runtime API + MCP", "ローカル連携"],
    ["fleet", "永続的なマルチエージェント作業"],
  ],
  runtimeLink: "ランタイムの各画面と安定性の注記を見る",
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
