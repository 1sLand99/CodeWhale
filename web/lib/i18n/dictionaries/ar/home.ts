import type { HomeDict } from "../types";

/**
 * Arabic home dictionary — native copy for the Tidal Folio landing page,
 * in the current direction: your models, more capable together; agents
 * and control on your own machine; availability stated per surface as it
 * is today. Product vocabulary stays literal (Plan / Work / Operate, Ask /
 * Auto-Review / Full Access, Codewhale, TUI, codewhale exec, Fleet).
 */

export const home: HomeDict = {
  metaTitle: "Codewhale — نماذجك. أكثر قدرة معًا.",
  metaDescription:
    "Codewhale نظام حوسبة وكيلية مفتوح المصدر. أحضر النماذج التي تستخدمها بالفعل — مستضافة أو عبر بوابة أو محلية — ودعها تعمل معًا في طرفيتك، على جهازك، وتحت سيطرتك. Rust، رخصة MIT.",
  kicker: "حوسبة وكيلية، بشروطك",
  heroTitleA: "نماذجك.",
  heroTitleB: "أكثر قدرة معًا.",
  heroIntro:
    "{brand} يجمع النماذج التي تستخدمها بالفعل في طرفية واحدة ويجعلها تعمل كطاقم — تقرأ الشيفرة، وتعدّل الملفات، وتشغّل الفحوصات — بينما تقرر أنت ما يُسمح لكل منها به. مفتوح المصدر، على جهازك.",
  getCodewhale: "احصل على Codewhale",
  exploreProduct: "استكشف المنتج",
  shotPreview: "معاينة الطرفية",
  shotBuild: "إصدار تطوير v{version}",
  screenshotAlt:
    "إصدار تطوير Codewhale v0.9.12 في طرفية: شعار الحوت بنقاط برايل، وجلسة جديدة بلا سجل، وحقل كتابة الرسالة، وتذييل يعرض Full Access ووضع Work ومهمتين مجدولتين وخوادم MCP قيد الاتصال ونموذج GLM-5.3 بأقصى جهد",
  latestRelease: "أحدث إصدار {tag}",
  releaseUnavailable: "حالة الإصدار غير متاحة",
  currentSource: "المصدر",
  sourceCandidate: "غير منشور",
  providerRoutes: "{count} مزوّد",
  publishedRelease: "منشور",
  figcaptionSourceCandidate: "غير منشور",
  chapterTerminal: "طرفيتك",
  chapterTerminalTitle: "مكان مألوف تبدأ منه.",
  gainHeading:
    "ما تحصل عليه ليس روبوت دردشة، بل قوة رفع على النماذج التي تدفع ثمنها بالفعل.",
  gainLede:
    "يمكن لجلسة واحدة أن تضم عدة نماذج في آن واحد، كلٌّ في الدور الذي أسندته إليه، وكلها تعمل في المستودع نفسه وبالقواعد نفسها.",
  gain: [
    ["نماذجك", "مفاتيح مستضافة، أو بوابة، أو بيئة تشغيل محلية بلا أي مفتاح. ثبّت نموذجًا مختلفًا لكل دور واحتفظ بالمزوّد الذي اخترته — اسم النموذج لا يبدّل المزوّد عنك أبدًا."],
    ["وكلاء أكفاء", "أوضاع Plan وWork وOperate؛ أسطول fleet من الوكلاء الفرعيين لمهمة واحدة؛ أدوات للملفات والصدفة والويب وMCP؛ جلسات تُحفظ وتُستأنف وتُرجَع."],
    ["التحكم على جهازك", "Ask أو Auto-Review أو Full Access — أنت تحدد كم ينجز قبل أن يسأل. يعمل محليًا، داخل صندوق رملي حيث يسمح نظام التشغيل، مع سجل تدقيق يمكنك قراءته."],
  ],
  chapterModels: "نماذجك",
  modelsHeading: "أحضر ما لديك. لا تغيّر ما لم تختره.",
  modelsBody:
    "يأتي Codewhale مع {count} مزوّدًا ويعاملهم على قدم المساواة. احفظ المفتاح مرة واحدة، وسمِّ نموذجًا، ويبقى المسار كما ضبطته تمامًا. النماذج المحلية عبر vLLM أو SGLang أو Ollama لا تحتاج إلى مفتاح.",
  modelsFacts: [
    ["مستضاف", "مفتاح API الخاص بك، محفوظ عبر codewhale auth set"],
    ["بوابة", "نقطة نهاية واحدة لنماذج كثيرة، والمزوّد ما زال من اختيارك"],
    ["محلي", "vLLM وSGLang وOllama على localhost — غالبًا بلا مفتاح"],
  ],
  modelsLink: "اطّلع على كل المزوّدين",
  startHeading: "أربع خطوات حتى أول جلسة.",
  startLede:
    "ثبّت، وافتح جلسة بلا مفتاح، واربط مزوّدًا، ثم أنشئ fleet عندما لا يكفي نموذج واحد.",
  startGuideLink: "اقرأ دليل البداية ←",
  startVocabularyLink: "اطّلع على مفردات المنتج ←",
  chapterAccount: "أين يعمل اليوم",
  availabilityHeading: "متاح الآن، وقيد التطوير، وليس بعد — بوضوح.",
  availabilityLede:
    "الطرفية هي المنتج الصادر. كل ما عداها مدرج بالحالة التي هو عليها فعلًا.",
  availability: [
    ["الطرفية", "صدرت", "npm وCargo وملفات ثنائية جاهزة للينكس وmacOS وويندوز. أندرويد على Termux معاينة."],
    ["تطبيق الويب", "تسجيل الدخول إلى الحساب متاح", "سجّل الدخول أو أنشئ حسابًا على app.codewhale.net. طاولة العمل في المتصفح نفسها ما زالت معاينة تطوير."],
    ["سطح المكتب", "إصدار تطوير", "توجد إصدارات ألفا لـ macOS ولينكس وويندوز. لا يوجد تطبيق سطح مكتب صادر بعد."],
    ["حواسيب سحابية", "غير متاح بعد", "تشغيل العمل على حاسوب مستضاف قيد التطوير. ستقول هذه الصفحة ذلك عندما يعمل."],
  ],
  availabilityNote:
    "الطرفية لا تحتاج إلى حساب. الحساب بحد ذاته ليس خطة مدفوعة أبدًا، ولا شيء في هذا الموقع يمكنه تحصيل أي رسوم منك.",
  accountLink: "أنشئ حسابًا",
  surfacesHeading: "استخدم الـ Runtime حيث يجري العمل.",
  surfaces: [
    ["TUI", "عمل تفاعلي في الطرفية"],
    ["codewhale exec", "سكربتات وCI"],
    ["عميل الويب", "عميل متصفح محصور في loopback"],
    ["Runtime API + MCP", "تكاملات محلية"],
    ["fleet", "عمل متعدد الوكلاء دائم"],
  ],
  runtimeLink: "اطّلع على واجهات الـ Runtime وملاحظات الاستقرار ←",
  installBandHeading: "ابدأ بأمر واحد.",
  copy: "انسخ",
  copied: "نُسخ ✓",
  binaries: "الملفات الثنائية",
  chinaMirrors: "مرايا في الصين",
  installGuideLink: "اقرأ دليل التثبيت ←",
  communityHeading: "يُبنى علنًا",
  communityBody:
    "برخصة MIT، وبتشكيل من المساهمين عبر الـ Runtimeات والمزودين والمنصات والتوثيق والاختبارات.",
  communityLinksAria: "روابط المجتمع",
  contribute: "ساهم",
};
