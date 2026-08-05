import type { HomeDict } from "../types";

/**
 * Ukrainian home pack. "Receipt" renders as «протокол» throughout (a durable,
 * official record) so it stays distinct from the workflow step «Звіт»
 * (Report). Mode and permission names — Plan / Act / Operate, Ask /
 * Auto-Review / Full Access — stay literal, matching crates/tui/locales/uk.json.
 */
export const home: HomeDict = {
  metaTitle: "Codewhale — Ми занурюємось у глибину, щоб не довелося вам.",
  metaDescription:
    "Codewhale занурюється в глибину, щоб не довелося вам — термінальний агент, що ставить силу LLM на відстані руки. На вашій машині. Rust, MIT.",

  kicker: "Відкритий код · Будь-яка модель · Працює у вашому терміналі",
  heroTitleA: "Ми занурюємось у глибину,",
  heroTitleB: "щоб не довелося вам.",
  heroIntro:
    "{brand} дає звичайним людям силу LLM, щоб створювати речі. У терміналі читає репозиторій, редагує файли, запускає перевірки й лишає протокол — не вимагаючи знання коду. На вашій машині; модель — змінний компонент.",
  install: "Встановити",
  docs: "Документація",
  copy: "Копіювати",
  copied: "Скопійовано ✓",

  installEyebrow: "встановлення одним рядком",
  installRequirement: "потрібен Node 18+ — без тулчейна Rust",
  installOtherWays: "інші способи →",

  latestRelease: "Останній реліз {tag}",
  releaseUnavailable: "Статус релізу недоступний",
  currentSource: "Поточне джерело",
  sourceCandidate: "Кандидат із джерела",
  providerRoutes: "Маршрути провайдерів: {count}",
  publishedRelease: "опублікований реліз",
  figcaptionSourceCandidate: "кандидат із джерела",

  shotSession: "Поточний сеанс",
  screenshotAlt:
    "Поточний термінальний сеанс Codewhale: режим Operate, кит, композер і нижня панель",
  figcaption: "Поточний сеанс Codewhale · режим Operate · дозволи Ask",

  proofHeading:
    "Підводна термінальна оболонка. Нейтральна до моделей. Насамперед локальна.",
  proofBody:
    "Підключіть модель, якою вже користуєтеся — хмарну, шлюзову або локальну. Plan / Act / Operate та явні режими дозволів лишають занурення під вашим контролем.",

  sealDecides: "法",
  decidesEyebrow: "Подивіться, як він ухвалює рішення",
  decidesHeading: "Правила, які видно в ході міркувань",
  decidesLede:
    "Фрагменти справжнього сеансу — ранжовані правила проєкту видно в міркуваннях моделі, а не лише в заяві на сторінці.",

  sealWorkflow: "行",
  workflowHeading: "Від завдання до перевіреної зміни.",
  workflow: [
    ["Огляд", "Читає репозиторій, його інструкції та завдання."],
    ["Дія", "Редагує файли в явно окреслених межах схвалення."],
    ["Перевірка", "Запускає перевірки та вивчає результат."],
    ["Звіт", "Залишає стислий, довговічний протокол."],
  ],
  receiptAria: "Приклад робочого протоколу",
  receiptInspect: "репозиторій та інструкції",
  receiptAct: "редагування в межах обраного режиму дозволів",
  receiptReport: "перевірки пройдено · протокол збережено",

  sealStart: "起",
  startHeading: "Уперше в Codewhale? Чотири кроки від початку до кінця.",
  startLede:
    "Встановлення → перший сеанс без ключів → підключення провайдера → перший Workflow у Fleet. Терміни — на сторінці словника.",
  startGuideLink: "Читати посібник для початківців →",
  startVocabularyLink: "Переглянути словник продукту →",

  sealBoundaries: "界",
  boundariesHeadingA: "Ваша модель.",
  boundariesHeadingB: "Ваші межі.",
  boundariesBody:
    "Явно обирайте модель, режим роботи та режим дозволів. Невідома вартість лишається невідомою, а інтерфейси в статусі попереднього перегляду позначені саме так.",
  hostedGatewayLocal: "Хмарні, шлюзові та локальні моделі",
  planActOperateDesc: "Від планування лише для читання до автономного виконання",
  askAutoReviewDesc: "Оберіть режим дозволів для роботи",
  tuiExecWebDesc: "Інтерактивні та неінтерактивні інтерфейси рантайму",

  sealSurfaces: "面",
  surfacesHeading: "Використовуйте рантайм там, де відбувається робота.",
  surfaces: [
    ["TUI", "Інтерактивна робота в терміналі"],
    ["codewhale exec", "Скрипти та CI"],
    ["Вебклієнт", "Браузерний клієнт лише через loopback"],
    ["Runtime API + MCP", "Локальні інтеграції"],
    ["Fleet", "Стійка багатоагентна робота"],
  ],
  runtimeLink: "Інтерфейси рантайму та нотатки про стабільність →",

  installBandHeading: "Почніть з однієї команди.",
  binaries: "Бінарні файли",
  chinaMirrors: "дзеркала в Китаї",
  installGuideLink: "Читати посібник зі встановлення →",

  sealCommunity: "众",
  communityHeading: "Розробляємо відкрито",
  communityBody:
    "Ліцензія MIT; проєкт формують учасники, що працюють над рантаймами, провайдерами, платформами, документацією та тестами.",
  communityLinksAria: "Посилання спільноти",
  contribute: "Участь",
};
