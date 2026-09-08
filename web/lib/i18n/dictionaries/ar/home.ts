import type { HomeDict } from "../types";

/**
 * Arabic home dictionary — native copy for the Tidal Folio landing page,
 * in the current direction: your models, more capable together; agents
 * and control on your own machine; availability stated per surface as it
 * is today. Product vocabulary stays literal (Plan / Work / Operate, Ask /
 * Auto-Review / Full Access, Codewhale, TUI, codewhale exec, Fleet).
 */

export const home: HomeDict = {
  metaTitle: "Codewhale — أنشئ ما تريد.",
  metaDescription:
    "ابنِ برمجيات، واعمل على الملفات، وأتمت المهام مع Codewhale. اختر نماذج مستضافة أو محلية، وبدّل المزوّدين مع تغيّر احتياجات عملك.",
  kicker: "وكلاء ذكاء اصطناعي مفتوحو المصدر",
  heroTitleA: "أنشئ ما تريد.",
  heroTitleB: "اختر نماذجك.",
  heroIntro:
    "يمنحك {brand} وكلاء يمكنهم بناء البرمجيات والعمل على الملفات وأتمتة المهام. استخدم النماذج التي تختارها، وبدّل المزوّدين مع تغيّر احتياجات عملك.",
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
  chapterTerminalTitle: "ابدأ بشيء تريد صنعه.",
  gainHeading:
    "حوّل أفكارك إلى عمل.",
  gainLede:
    "ابنِ مشروعًا، أو ابحث في سؤال، أو أتمت مهمة. ابدأ بوكيل واحد، ووزّع الأعمال الأكبر على عدة وكلاء.",
  gain: [
    [
      "ابنِ شيئًا",
      "حوّل فكرة إلى برمجيات تعمل. يمكن لوكلائك تعديل الملفات وتشغيل الأوامر والتحقّق من النتيجة."
    ],
    [
      "أتمت العمل المتكرر",
      "أنشئ نصوصًا برمجية وسير عمل للمهام المتكررة، ثم شغّلها من الطرفية."
    ],
    [
      "اختر نماذجك",
      "اربط نماذج مستضافة أو محلية. وزّع أجزاء العمل الكبير على وكلاء بنماذج وأدوار مختلفة."
    ]
  ],
  chapterModels: "نماذجك",
  modelsHeading: "اعثر على نموذج يناسب المهمة.",
  modelsBody:
    "استخدم مزوّدًا لنماذج مستضافة، أو اتصل عبر بوابة، أو شغّل نموذجًا محليًا. اختر مزوّدًا ونموذجًا لكل جلسة، وغيّرهما أثناء العمل.",
  modelsFacts: [
    ["مستضاف", "مفتاح API الخاص بك، محفوظ عبر codewhale auth set --provider <id>"],
    ["بوابة", "نقطة نهاية واحدة لنماذج كثيرة، والمزوّد ما زال من اختيارك"],
    ["محلي", "vLLM وSGLang وOllama على localhost — غالبًا بلا مفتاح"],
  ],
  modelsLink: "استكشف النماذج والمزوّدين",
  startHeading: "ابدأ مهمتك الأولى.",
  startLede:
    "ثبّت Codewhale، واربط نموذجًا، وأخبره بما تريد فعله. أضف Fleet عندما تريد توزيع العمل على عدة وكلاء.",
  startGuideLink: "اقرأ دليل البداية ←",
  startVocabularyLink: "اطّلع على مفردات المنتج ←",
  chapterAccount: "احصل على Codewhale",
  availabilityHeading: "أين تستخدم Codewhale.",
  availabilityLede:
    "ابدأ في الطرفية. التطبيق وأجهزة الكمبيوتر السحابية قيد التطوير.",
  availability: [
    [
      "الطرفية",
      "تم الإصدار",
      "ملفات إصدار GitHub الثنائية لأنظمة Linux وmacOS وWindows؛ ويمكن استخدام npm وCargo كبديلين. دعم Android عبر Termux ما زال في مرحلة المعاينة."
    ],
    [
      "تطبيق الويب",
      "معاينة قيد التطوير",
      "الوصول إلى الحساب وإقران المتصفح ضمن المعاينة قيد التطوير."
    ],
    [
      "سطح المكتب",
      "نسخة قيد التطوير",
      "تطبيق macOS قيد التطوير؛ وسيتاح تنزيله للجميع لاحقًا."
    ],
    [
      "أجهزة الكمبيوتر السحابية",
      "قيد التطوير",
      "أجهزة كمبيوتر مستضافة لتشغيل مهامك."
    ]
  ],
  availabilityNote:
    "تعمل الطرفية دون حساب Codewhale. يتولى مزوّدك فوترة استخدام النماذج المستضافة.",
  accountLink: "أنشئ حسابًا",
  surfacesHeading: "استخدم الـ Runtime حيث يجري العمل.",
  surfaces: [
    ["TUI", "عمل تفاعلي في الطرفية"],
    ["codewhale exec", "سكربتات وCI"],
    ["عميل الويب المحلي","واجهة على localhost؛ مساحة العمل المستضافة في المتصفح قيد التطوير"],
    ["Runtime API + MCP", "تكاملات محلية"],
    ["Fleet","عدة وكلاء يعملون على مهمة واحدة"],
  ],
  runtimeLink: "استكشف التكاملات",
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
