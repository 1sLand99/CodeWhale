import type { HomeDict } from "../types";

/**
 * Hindi home dictionary — native copy for the Tidal Folio landing page,
 * in the current direction: your models, more capable together; agents
 * and control on your own machine; availability stated per surface as it
 * is today. Product vocabulary stays literal (Plan / Work / Operate, Ask /
 * Auto-Review / Full Access, Codewhale, TUI, codewhale exec, Fleet).
 */

export const home: HomeDict = {
  metaTitle: "Codewhale — जो चाहें, बनाएँ।",
  metaDescription:
    "Codewhale से सॉफ़्टवेयर बनाएँ, फ़ाइलों पर काम करें और कामों को स्वचालित करें। होस्टेड या लोकल मॉडल चुनें और काम की ज़रूरत के अनुसार प्रदाता बदलें।",
  kicker: "ओपन-सोर्स AI एजेंट",
  heroTitleA: "जो चाहें, बनाएँ।",
  heroTitleB: "अपने मॉडल चुनें।",
  heroIntro:
    "{brand} आपको ऐसे एजेंट देता है जो सॉफ़्टवेयर बना सकते हैं, फ़ाइलों पर काम कर सकते हैं और कामों को स्वचालित कर सकते हैं। अपनी पसंद के मॉडल इस्तेमाल करें और काम की ज़रूरत के अनुसार प्रदाता बदलें।",
  getCodewhale: "Codewhale लें",
  exploreProduct: "उत्पाद देखें",
  shotPreview: "टर्मिनल पूर्वावलोकन",
  shotBuild: "v{version} डेवलपमेंट बिल्ड",
  screenshotAlt:
    "टर्मिनल में Codewhale v0.9.12 डेवलपमेंट बिल्ड: ब्रेल बिंदुओं से बना व्हेल चिह्न, बिना इतिहास वाला नया सत्र, संदेश कंपोज़र, और फ़ुटर जिसमें Full Access, Work मोड, दो निर्धारित कार्य, कनेक्ट होते MCP सर्वर और अधिकतम प्रयास पर GLM-5.3 मॉडल दिखता है",
  latestRelease: "नवीनतम रिलीज़ {tag}",
  releaseUnavailable: "रिलीज़ स्थिति उपलब्ध नहीं",
  currentSource: "सोर्स",
  sourceCandidate: "अप्रकाशित",
  providerRoutes: "{count} प्रोवाइडर",
  publishedRelease: "प्रकाशित",
  figcaptionSourceCandidate: "अप्रकाशित",
  chapterTerminal: "आपका टर्मिनल",
  chapterTerminalTitle: "शुरुआत उस चीज़ से करें जिसे आप बनाना चाहते हैं।",
  gainHeading:
    "अपने विचारों को साकार करें।",
  gainLede:
    "प्रोजेक्ट बनाएँ, किसी सवाल पर शोध करें या कोई काम स्वचालित करें। एक एजेंट से शुरू करें और बड़े काम कई एजेंटों में बाँट दें।",
  gain: [
    [
      "कुछ बनाएँ",
      "अपने विचार को काम करने वाले सॉफ़्टवेयर में बदलें। आपके एजेंट फ़ाइलें संपादित कर सकते हैं, कमांड चला सकते हैं और नतीजे जाँच सकते हैं।"
    ],
    [
      "दोहराए जाने वाले काम स्वचालित करें",
      "बार-बार किए जाने वाले कामों के लिए स्क्रिप्ट और वर्कफ़्लो बनाएँ, फिर उन्हें टर्मिनल से चलाएँ।"
    ],
    [
      "अपने मॉडल चुनें",
      "होस्टेड या लोकल मॉडल जोड़ें। किसी बड़े काम के हिस्से अलग-अलग मॉडल और भूमिकाओं वाले एजेंटों को दें।"
    ]
  ],
  chapterModels: "आपके मॉडल",
  modelsHeading: "काम के लिए सही मॉडल खोजें।",
  modelsBody:
    "प्रदाता के होस्टेड मॉडल इस्तेमाल करें, गेटवे के ज़रिए जुड़ें या मॉडल अपने कंप्यूटर पर चलाएँ। हर सत्र के लिए प्रदाता और मॉडल चुनें और काम करते समय उन्हें बदलें।",
  modelsFacts: [
    ["होस्टेड", "आपकी अपनी API कुंजी, codewhale auth set --provider <id> से सेव"],
    ["गेटवे", "कई मॉडलों के लिए एक एंडपॉइंट, प्रोवाइडर फिर भी आप चुनते हैं"],
    ["लोकल", "localhost पर vLLM, SGLang, Ollama — आमतौर पर बिना कुंजी"],
  ],
  modelsLink: "मॉडल और प्रदाता देखें",
  startHeading: "अपना पहला काम शुरू करें।",
  startLede:
    "Codewhale इंस्टॉल करें, मॉडल जोड़ें और बताएँ कि आप क्या करना चाहते हैं। जब आप कई एजेंटों में काम बाँटना चाहें, तो Fleet जोड़ें।",
  startGuideLink: "शुरुआती गाइड पढ़ें",
  startVocabularyLink: "उत्पाद शब्दावली देखें",
  chapterAccount: "Codewhale लें",
  availabilityHeading: "Codewhale कहाँ इस्तेमाल करें।",
  availabilityLede:
    "टर्मिनल से शुरू करें। ऐप और क्लाउड कंप्यूटर अभी विकासाधीन हैं।",
  availability: [
    [
      "टर्मिनल",
      "जारी",
      "Linux, macOS और Windows के लिए GitHub रिलीज़ बाइनरी उपलब्ध हैं; npm और Cargo वैकल्पिक तरीके हैं। Termux पर Android अभी प्रीव्यू में है।"
    ],
    [
      "वेब ऐप",
      "डेवलपमेंट प्रीव्यू",
      "डेवलपमेंट प्रीव्यू में खाते तक पहुँच और ब्राउज़र पेयरिंग।"
    ],
    [
      "डेस्कटॉप",
      "विकासाधीन बिल्ड",
      "macOS ऐप अभी विकासाधीन है; सभी के लिए डाउनलोड बाद में उपलब्ध होगा।"
    ],
    [
      "क्लाउड कंप्यूटर",
      "विकासाधीन",
      "आपके काम चलाने के लिए होस्टेड कंप्यूटर।"
    ]
  ],
  availabilityNote:
    "टर्मिनल Codewhale खाते के बिना काम करता है। होस्टेड मॉडल के इस्तेमाल का शुल्क आपका प्रदाता लेता है।",
  accountLink: "खाता बनाएँ",
  surfacesHeading: "रनटाइम को वहीं उपयोग करें जहाँ काम होता है।",
  surfaces: [
    ["TUI", "टर्मिनल में इंटरैक्टिव काम"],
    ["codewhale exec", "स्क्रिप्ट और CI"],
    ["लोकल वेब क्लाइंट","localhost पर इंटरफ़ेस; ब्राउज़र में होस्टेड कार्यक्षेत्र विकासाधीन है"],
    ["Runtime API + MCP", "लोकल इंटीग्रेशन"],
    ["Fleet","एक काम पर कई एजेंट"],
  ],
  runtimeLink: "इंटीग्रेशन देखें",
  installBandHeading: "एक ही कमांड से शुरू करें।",
  copy: "कॉपी करें",
  copied: "कॉपी हो गया ✓",
  binaries: "बाइनरी",
  chinaMirrors: "चीन मिरर",
  installGuideLink: "इंस्टॉल गाइड पढ़ें",
  communityHeading: "खुले में निर्मित",
  communityBody:
    "MIT-लाइसेंस प्राप्त और रनटाइम, प्रोवाइडर, प्लेटफ़ॉर्म, दस्तावेज़ीकरण और परीक्षणों के योगदानकर्ताओं द्वारा गढ़ा गया।",
  communityLinksAria: "समुदाय लिंक",
  contribute: "योगदान दें",
};
