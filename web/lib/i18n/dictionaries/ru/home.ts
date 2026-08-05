import type { HomeDict } from "../types";

/**
 * Russian home dictionary. Key parity with `en/home.ts` is enforced by
 * `npm run check:locales` and `dictionaries.test.ts`.
 *
 * Fixed product vocabulary stays Latin and matches the TUI ru locale pack:
 * Plan / Act / Operate, Ask / Auto-Review / Full Access, Codewhale, Fleet.
 * "receipt" is rendered "квитанция", as in `crates/tui/locales/ru.json`.
 * The `seal*` values are the paper's marks, shared across locales.
 */
export const home: HomeDict = {
  metaTitle: "Codewhale — Мы ныряем в глубину, чтобы не пришлось вам.",
  metaDescription:
    "Codewhale ныряет в глубину, чтобы не пришлось вам — терминальный агент, который ставит силу LLM на расстоянии вытянутой руки. На вашей машине. Rust, MIT.",

  kicker: "Открытый код · Любая модель · В вашем терминале",
  heroTitleA: "Мы ныряем в глубину,",
  heroTitleB: "чтобы не пришлось вам.",
  heroIntro:
    "{brand} даёт обычным людям силу LLM, чтобы создавать вещи. В терминале читает репозиторий, правит файлы, запускает проверки и оставляет квитанцию — не требуя знания кода. Работает на вашей машине; модель — сменный компонент.",
  install: "Установка",
  docs: "Документация",
  copy: "Копировать",
  copied: "Скопировано ✓",

  installEyebrow: "установка одной строкой",
  installRequirement: "нужен Node 18+ — без тулчейна Rust",
  installOtherWays: "другие способы →",

  latestRelease: "Последний релиз {tag}",
  releaseUnavailable: "Статус релиза недоступен",
  currentSource: "Текущие исходники",
  sourceCandidate: "Кандидат из исходников",
  providerRoutes: "маршрутов провайдеров — {count}",
  publishedRelease: "опубликованный релиз",
  figcaptionSourceCandidate: "кандидат из исходников",

  shotSession: "Текущий сеанс",
  screenshotAlt:
    "Текущий терминальный сеанс Codewhale: режим Operate, кит, поле ввода и нижняя панель",
  figcaption: "Текущий сеанс Codewhale · режим Operate · разрешения Ask",

  proofHeading: "Подводная терминальная оболочка. Нейтральна к моделям. Работает локально.",
  proofBody:
    "Подключите модель, которой уже пользуетесь — облачную, шлюзовую или локальную. Plan / Act / Operate и явные режимы разрешений держат погружение под вашим контролем.",

  sealDecides: "法",
  decidesEyebrow: "Как он принимает решения",
  decidesHeading: "Правила видны прямо в ходе рассуждений",
  decidesLede:
    "Фрагменты реального сеанса — приоритет правил проекта виден в рассуждении модели, а не только заявлен на странице.",

  sealWorkflow: "行",
  workflowHeading: "От задачи к проверенному изменению.",
  workflow: [
    ["Осмотр", "Читает репозиторий, его инструкции и задачу."],
    ["Действие", "Правит файлы в рамках явных границ одобрения."],
    ["Проверка", "Выполняет проверки и изучает результат."],
    ["Отчёт", "Оставляет краткую и долговечную квитанцию."],
  ],
  receiptAria: "Пример рабочей квитанции",
  receiptInspect: "репозиторий и инструкции",
  receiptAct: "правка в рамках выбранного режима разрешений",
  receiptReport: "проверки пройдены · квитанция сохранена",

  sealStart: "起",
  startHeading: "Впервые в Codewhale? Четыре шага от начала до конца.",
  startLede:
    "Установка → первый сеанс без ключей → подключение провайдера → первый воркфлоу Fleet. Термины — на странице словаря.",
  startGuideLink: "Читать руководство «С чего начать» →",
  startVocabularyLink: "Посмотреть словарь продукта →",

  sealBoundaries: "界",
  boundariesHeadingA: "Ваша модель.",
  boundariesHeadingB: "Ваши границы.",
  boundariesBody:
    "Вы явно выбираете модель, рабочий режим и режим разрешений. Неизвестная стоимость остаётся неизвестной, а предварительные возможности прямо помечены как предварительные.",
  hostedGatewayLocal: "Облачные, шлюзовые и локальные модели",
  planActOperateDesc: "От планирования только для чтения до автономной работы",
  askAutoReviewDesc: "Выберите режим разрешений под задачу",
  tuiExecWebDesc: "Интерактивные и неинтерактивные интерфейсы рантайма",

  sealSurfaces: "面",
  surfacesHeading: "Используйте рантайм там, где идёт работа.",
  surfaces: [
    ["TUI", "Интерактивная работа в терминале"],
    ["codewhale exec", "Скрипты и CI"],
    ["Веб-клиент", "Клиент в браузере, только через loopback"],
    ["Runtime API + MCP", "Локальные интеграции"],
    ["Fleet", "Долговечная работа нескольких агентов"],
  ],
  runtimeLink: "Интерфейсы рантайма и заметки о стабильности →",

  installBandHeading: "Начните с одной команды.",
  binaries: "Бинарные сборки",
  chinaMirrors: "Зеркала в Китае",
  installGuideLink: "Читать руководство по установке →",

  sealCommunity: "众",
  communityHeading: "Разрабатывается открыто",
  communityBody:
    "Лицензия MIT; проект формируют контрибьюторы, работающие над рантаймами, провайдерами, платформами, документацией и тестами.",
  communityLinksAria: "Ссылки сообщества",
  contribute: "Участие",
};
