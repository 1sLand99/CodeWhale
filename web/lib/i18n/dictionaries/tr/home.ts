import type { HomeDict } from "../types";

/**
 * Turkish home dictionary — native copy for the Tidal Folio landing page,
 * in the current direction: your models, more capable together; agents
 * and control on your own machine; availability stated per surface as it
 * is today. Product vocabulary stays literal (Plan / Work / Operate, Ask /
 * Auto-Review / Full Access, Codewhale, TUI, codewhale exec, Fleet).
 */

export const home: HomeDict = {
  metaTitle: "Codewhale — İstediğini oluştur.",
  metaDescription:
    "Codewhale ile yazılım geliştir, dosyalarla çalış ve görevleri otomatikleştir. Barındırılan veya yerel modelleri seç, işinin ihtiyaçları değiştikçe sağlayıcı değiştir.",
  kicker: "Açık kaynaklı yapay zekâ ajanları",
  heroTitleA: "İstediğini oluştur.",
  heroTitleB: "Modellerini seç.",
  heroIntro:
    "{brand}, yazılım geliştirebilen, dosyalarla çalışabilen ve görevleri otomatikleştirebilen ajanlar sunar. Seçtiğin modelleri kullan, işinin ihtiyaçları değiştikçe sağlayıcı değiştir.",
  getCodewhale: "Codewhale'i edin",
  exploreProduct: "Ürünü keşfet",
  shotPreview: "Terminal önizlemesi",
  shotBuild: "v{version} geliştirme derlemesi",
  screenshotAlt:
    "Terminalde Codewhale v0.9.12 geliştirme derlemesi: braille noktalarından balina işareti, geçmişi olmayan yeni bir oturum, mesaj yazma alanı ve Full Access, Work modu, iki zamanlanmış görev, bağlanan MCP sunucuları ile en yüksek çabada GLM-5.3 modelini gösteren alt bilgi",
  latestRelease: "En yeni sürüm {tag}",
  releaseUnavailable: "Sürüm durumu kullanılamıyor",
  currentSource: "Kaynak",
  sourceCandidate: "Yayımlanmadı",
  providerRoutes: "{count} sağlayıcı",
  publishedRelease: "yayımlandı",
  figcaptionSourceCandidate: "yayımlanmadı",
  chapterTerminal: "Senin terminalin",
  chapterTerminalTitle: "Yapmak istediğin bir şeyle başla.",
  gainHeading:
    "Fikirlerini hayata geçir.",
  gainLede:
    "Bir proje geliştir, bir soruyu araştır veya bir görevi otomatikleştir. Tek bir ajanla başla, daha büyük işleri birkaç ajana paylaştır.",
  gain: [
    [
      "Bir şey geliştir",
      "Bir fikri çalışan yazılıma dönüştür. Ajanların dosyaları düzenleyebilir, komutları çalıştırabilir ve sonucu kontrol edebilir."
    ],
    [
      "Tekrarlanan işleri otomatikleştir",
      "Tekrar tekrar yaptığın işler için betikler ve iş akışları oluştur, ardından bunları terminalden çalıştır."
    ],
    [
      "Modellerini seç",
      "Barındırılan veya yerel modelleri bağla. Büyük bir işin parçalarını farklı model ve rollere sahip ajanlara ver."
    ]
  ],
  chapterModels: "Senin modellerin",
  modelsHeading: "Göreve uygun bir model bul.",
  modelsBody:
    "Barındırılan bir model sağlayıcısı kullan, bir ağ geçidi üzerinden bağlan veya yerel bir model çalıştır. Her oturum için bir sağlayıcı ve model seç, çalışırken bunları değiştir.",
  modelsFacts: [
    ["Barındırılan", "codewhale auth set --provider <id> ile kaydedilen kendi API anahtarın"],
    ["Gateway", "Birçok model için tek uç nokta, sağlayıcıyı yine sen seçersin"],
    ["Yerel", "localhost üzerinde vLLM, SGLang, Ollama — genellikle anahtarsız"],
  ],
  modelsLink: "Modelleri ve sağlayıcıları keşfet",
  startHeading: "İlk görevine başla.",
  startLede:
    "Codewhale'i kur, bir model bağla ve ne yapmak istediğini söyle. İşi birkaç ajana paylaştırmak istediğinde bir Fleet ekle.",
  startGuideLink: "Başlangıç kılavuzunu oku",
  startVocabularyLink: "Ürün sözlüğünü gör",
  chapterAccount: "Codewhale'i edin",
  availabilityHeading: "Codewhale nerede kullanılır?",
  availabilityLede:
    "Terminalde başla. Uygulama ve bulut bilgisayarları geliştirme aşamasında.",
  availability: [
    [
      "Terminal",
      "Yayınlandı",
      "Linux, macOS ve Windows için GitHub sürüm ikili dosyaları; npm ve Cargo alternatiflerdir. Termux üzerinde Android desteği önizleme aşamasında."
    ],
    [
      "Web uygulaması",
      "Geliştirme önizlemesi",
      "Geliştirme önizlemesinde hesap erişimi ve tarayıcı eşleştirme."
    ],
    [
      "Masaüstü",
      "Geliştirme sürümü",
      "macOS uygulaması geliştirme aşamasında; herkese açık indirme daha sonra sunulacak."
    ],
    [
      "Bulut bilgisayarları",
      "Geliştirme aşamasında",
      "Görevlerini çalıştırmak için barındırılan bilgisayarlar."
    ]
  ],
  availabilityNote:
    "Terminal, Codewhale hesabı olmadan çalışır. Barındırılan model kullanımını sağlayıcın ücretlendirir.",
  accountLink: "Hesap oluştur",
  surfacesHeading: "Çalışma zamanını işin olduğu yerde kullan.",
  surfaces: [
    ["TUI", "Terminalde etkileşimli iş"],
    ["codewhale exec", "Betikler ve CI"],
    ["Yerel web istemcisi","localhost arayüzü; barındırılan tarayıcı çalışma alanı geliştirme aşamasında"],
    ["Runtime API + MCP", "Yerel entegrasyonlar"],
    ["Fleet","Tek bir işte birden çok ajan"],
  ],
  runtimeLink: "Entegrasyonları keşfet",
  installBandHeading: "Tek komutla başla.",
  copy: "Kopyala",
  copied: "Kopyalandı ✓",
  binaries: "İkililer",
  chinaMirrors: "Çin yansıları",
  installGuideLink: "Kurulum kılavuzunu oku",
  communityHeading: "Açıkça, halk önünde inşa edildi",
  communityBody:
    "MIT lisanslı; çalışma zamanları, sağlayıcılar, platformlar, belgelendirme ve testler katkısıyla şekillendi.",
  communityLinksAria: "Topluluk bağlantıları",
  contribute: "Katkıda bulun",
};
