import type { HomeDict } from "../types";

/**
 * Turkish home dictionary — native copy for the Tidal Folio landing page,
 * in the current direction: your models, more capable together; agents
 * and control on your own machine; availability stated per surface as it
 * is today. Product vocabulary stays literal (Plan / Work / Operate, Ask /
 * Auto-Review / Full Access, Codewhale, TUI, codewhale exec, Fleet).
 */

export const home: HomeDict = {
  metaTitle: "Codewhale — Senin modellerin. Birlikte daha yetenekli.",
  metaDescription:
    "Codewhale açık kaynaklı bir ajan tabanlı bilişim sistemidir. Zaten kullandığın modelleri —barındırılan, gateway üzerinden ya da yerel— terminaline getir ve senin makinende, senin denetiminde birlikte çalışmalarını sağla. Rust, MIT.",
  kicker: "Ajan tabanlı bilişim, senin şartlarınla",
  heroTitleA: "Senin modellerin.",
  heroTitleB: "Birlikte daha yetenekli.",
  heroIntro:
    "{brand} zaten kullandığın modelleri tek bir terminalde toplar ve bir mürettebat gibi çalıştırır — kodunu okur, dosyaları düzenler, kontrolleri çalıştırır — her birinin neye izinli olduğuna ise sen karar verirsin. Açık kaynak, senin makinende.",
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
  chapterTerminalTitle: "Başlamak için tanıdık bir yer.",
  gainHeading:
    "Elde ettiğin bir sohbet botu değil. Zaten ödediğin modeller üzerinde kaldıraç.",
  gainLede:
    "Tek bir oturum aynı anda birden çok modeli tutabilir; her biri verdiğin rolde, hepsi aynı depoda, aynı kurallarla çalışır.",
  gain: [
    ["Senin modellerin", "Barındırılan anahtarlar, bir gateway ya da hiç anahtar gerektirmeyen yerel bir runtime. Her role ayrı bir model sabitle ve seçtiğin sağlayıcıyı koru — bir model adı sağlayıcıyı senin yerine asla değiştirmez."],
    ["Yetenekli ajanlar", "Plan, Work ve Operate modları; tek bir iş için alt ajanlardan oluşan bir fleet; dosya, kabuk, web ve MCP araçları; kaydedilen, sürdürülen ve geri alınan oturumlar."],
    ["Makinende denetim", "Ask, Auto-Review ya da Full Access — sormadan önce ne kadar yapacağını sen belirlersin. Yerelde çalışır, işletim sisteminin izin verdiği yerde sandbox içinde, okuyabileceğin bir denetim günlüğüyle."],
  ],
  chapterModels: "Senin modellerin",
  modelsHeading: "Elindekini getir. Seçmediğin hiçbir şeyi değiştirme.",
  modelsBody:
    "Codewhale {count} sağlayıcıyla gelir ve hepsine eşit davranır. Anahtarı bir kez kaydet, bir model adı ver; rota tam senin ayarladığın gibi kalır. vLLM, SGLang ya da Ollama üzerindeki yerel modeller anahtar gerektirmez.",
  modelsFacts: [
    ["Barındırılan", "codewhale auth set ile kaydedilen kendi API anahtarın"],
    ["Gateway", "Birçok model için tek uç nokta, sağlayıcıyı yine sen seçersin"],
    ["Yerel", "localhost üzerinde vLLM, SGLang, Ollama — genellikle anahtarsız"],
  ],
  modelsLink: "Tüm sağlayıcıları gör",
  startHeading: "İlk oturuma dört adım.",
  startLede:
    "Kur, anahtarsız bir oturum aç, bir sağlayıcı bağla; tek model yetmediğinde bir fleet kur.",
  startGuideLink: "Başlangıç kılavuzunu oku",
  startVocabularyLink: "Ürün sözlüğünü gör",
  chapterAccount: "Bugün nerede çalışıyor",
  availabilityHeading: "Şu an kullanılabilir, geliştirmede ve henüz değil — açıkça söylenmiş.",
  availabilityLede:
    "Terminal yayınlanmış üründür. Geri kalan her şey gerçekte bulunduğu durumla listelenir.",
  availability: [
    ["Terminal", "Yayınlandı", "Linux, macOS ve Windows için GitHub Releases ikili dosyaları; npm ve Cargo alternatiflerdir. Termux üzerinde Android önizlemedir."],
    ["Web uygulaması", "Giriş ve uzaktan kontrol kullanılabilir", "Giriş yap ya da hesap oluştur, sonra çalışan yerel bir oturumda /rc yazarak tam o oturuma tarayıcıdan devam et. Tarayıcıdaki çalışma tezgâhının geri kalanı geliştirme önizlemesidir."],
    ["Masaüstü", "Geliştirme derlemesi", "macOS, Linux ve Windows için alfa derlemeleri var. Henüz yayınlanmış bir masaüstü uygulaması yok."],
    ["Bulut bilgisayarlar", "Henüz kullanılamıyor", "Barındırılan bir bilgisayarda iş çalıştırmak geliştirme aşamasında. Çalıştığında bu sayfa bunu söyleyecek."],
  ],
  availabilityNote:
    "Terminal için hesap gerekmez. Bir hesap tek başına asla ücretli bir plan değildir ve bu sitedeki hiçbir şey senden ücret alamaz.",
  accountLink: "Hesap oluştur",
  surfacesHeading: "Çalışma zamanını işin olduğu yerde kullan.",
  surfaces: [
    ["TUI", "Terminalde etkileşimli iş"],
    ["codewhale exec", "Betikler ve CI"],
    ["Web istemcisi", "Yalnızca geri döngülü tarayıcı istemcisi"],
    ["Runtime API + MCP", "Yerel entegrasyonlar"],
    ["fleet", "Kalıcı çok ajanlı iş"],
  ],
  runtimeLink: "Çalışma zamanı yüzeylerini ve kararlılık notlarını gör",
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
