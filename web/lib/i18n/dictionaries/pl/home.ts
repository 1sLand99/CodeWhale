import type { HomeDict } from "../types";

/**
 * Polish home dictionary — native copy for the Tidal Folio landing page,
 * in the current direction: your models, more capable together; agents
 * and control on your own machine; availability stated per surface as it
 * is today. Product vocabulary stays literal (Plan / Work / Operate, Ask /
 * Auto-Review / Full Access, Codewhale, TUI, codewhale exec, Fleet).
 */

export const home: HomeDict = {
  metaTitle: "Codewhale — Twoje modele. Razem potrafią więcej.",
  metaDescription:
    "Codewhale to otwarty system obliczeń agentowych. Weź modele, których już używasz — hostowane, przez bramkę lub lokalne — i pozwól im pracować razem w Twoim terminalu, na Twojej maszynie, pod Twoją kontrolą. Rust, MIT.",
  kicker: "Obliczenia agentowe na Twoich warunkach",
  heroTitleA: "Twoje modele.",
  heroTitleB: "Razem potrafią więcej.",
  heroIntro:
    "{brand} zbiera modele, których już używasz, w jednym terminalu i pozwala im pracować jak załodze — czytać kod, edytować pliki, uruchamiać sprawdzenia — a Ty decydujesz, na co każdy z nich ma zgodę. Open source, na Twojej maszynie.",
  getCodewhale: "Pobierz Codewhale",
  exploreProduct: "Poznaj produkt",
  shotPreview: "Podgląd terminala",
  shotBuild: "kompilacja deweloperska v{version}",
  screenshotAlt:
    "Kompilacja deweloperska Codewhale v0.9.12 w terminalu: znak wieloryba z punktów brajlowskich, nowa sesja bez historii, pole wiadomości oraz stopka pokazująca Full Access, tryb Work, dwa zaplanowane zadania, łączące się serwery MCP i model GLM-5.3 na maksymalnym wysiłku",
  latestRelease: "Najnowsze wydanie {tag}",
  releaseUnavailable: "Status wydania niedostępny",
  currentSource: "Źródło",
  sourceCandidate: "Niewydane",
  providerRoutes: "{count} providerów",
  publishedRelease: "wydane",
  figcaptionSourceCandidate: "niewydane",
  chapterTerminal: "Twój terminal",
  chapterTerminalTitle: "Znajome miejsce na początek.",
  gainHeading:
    "Nie dostajesz chatbota. Dostajesz dźwignię na modele, za które już płacisz.",
  gainLede:
    "Jedna sesja może trzymać kilka modeli naraz, każdy w roli, którą mu nadałeś, wszystkie w tym samym repozytorium i według tych samych reguł.",
  gain: [
    ["Twoje modele", "Klucze hostowane, bramka albo lokalny runtime bez żadnego klucza. Przypnij inny model do każdej roli i zachowaj wybranego dostawcę — nazwa modelu nigdy nie zmieni go za Ciebie."],
    ["Sprawni agenci", "Tryby Plan, Work i Operate; fleet subagentów do jednego zadania; narzędzia do plików, powłoki, sieci i MCP; sesje, które można zapisać, wznowić i cofnąć."],
    ["Kontrola na Twojej maszynie", "Ask, Auto-Review lub Full Access — Ty ustalasz, ile zrobi, zanim zapyta. Działa lokalnie, w piaskownicy tam, gdzie pozwala system, z dziennikiem audytu, który możesz przeczytać."],
  ],
  chapterModels: "Twoje modele",
  modelsHeading: "Przynieś to, co masz. Nie zmieniaj niczego, czego nie wybrałeś.",
  modelsBody:
    "Podłącz obsługiwanego dostawcę zdalnego, bramę lub lokalny serwer modeli. Przed rozpoczęciem sprawdź dostawcę i model. Lokalny serwer również może wymagać uwierzytelnienia.",
  modelsFacts: [
    ["Hostowane", "Twój własny klucz API zapisany przez codewhale auth set"],
    ["Bramka", "Jeden endpoint do wielu modeli, dostawcę nadal wybierasz Ty"],
    ["Lokalne", "vLLM, SGLang, Ollama na localhost — zwykle bez klucza"],
  ],
  modelsLink: "Zobacz wszystkich dostawców",
  startHeading: "Cztery kroki do pierwszej sesji.",
  startLede:
    "Zainstaluj, otwórz sesję bez klucza, podłącz dostawcę, a gdy jeden model nie wystarcza — skonfiguruj fleet.",
  startGuideLink: "Przeczytaj przewodnik na start",
  startVocabularyLink: "Zobacz słownik produktu",
  chapterAccount: "Gdzie to dziś działa",
  availabilityHeading: "Dostępne teraz, w budowie i jeszcze nie — powiedziane wprost.",
  availabilityLede:
    "Terminal jest wydanym produktem. Wszystko inne wymieniamy w stanie, w jakim naprawdę jest.",
  availability: [
    ["Terminal", "Wydany", "Pliki binarne z GitHub Releases dla Linuksa, macOS i Windows; npm i Cargo to alternatywy. Android w Termuksie to wersja poglądowa."],
    ["Aplikacja webowa", "Logowanie i zdalne sterowanie dostępne", "Zaloguj się lub załóż konto, a potem wpisz /rc w uruchomionej sesji lokalnej, aby kontynuować dokładnie tę sesję z przeglądarki. Reszta warsztatu w przeglądarce to wciąż podgląd deweloperski."],
    ["Desktop", "Kompilacja deweloperska", "Istnieją kompilacje alfa dla macOS, Linuksa i Windows. Wydanej aplikacji desktopowej jeszcze nie ma."],
    ["Komputery w chmurze", "Jeszcze niedostępne", "Uruchamianie pracy na hostowanym komputerze jest w budowie. Ta strona powie, kiedy zacznie działać."],
  ],
  availabilityNote:
    "Terminal nie wymaga konta. Konto samo w sobie nigdy nie jest planem płatnym i nic na tej stronie nie może Cię obciążyć.",
  accountLink: "Załóż konto",
  surfacesHeading: "Używaj runtime'u tam, gdzie odbywa się praca.",
  surfaces: [
    ["TUI", "Interaktywna praca w terminalu"],
    ["codewhale exec", "Skrypty i CI"],
    ["Klient web", "Klient przeglądarkowy, tylko loopback"],
    ["Runtime API + MCP", "Lokalne integracje"],
    ["fleet", "Trwała praca wielu agentów"],
  ],
  runtimeLink: "Zobacz powierzchnie runtime'u i notatki o stabilności",
  installBandHeading: "Zacznij jedną komendą.",
  copy: "Kopiuj",
  copied: "Skopiowano ✓",
  binaries: "Binarki",
  chinaMirrors: "Mirrory w Chinach",
  installGuideLink: "Przeczytaj przewodnik instalacji",
  communityHeading: "Budowane jawnie",
  communityBody:
    "Na licencji MIT, kształtowane przez współtwórców od runtime'ów, przez providerów, platformy, dokumentację po testy.",
  communityLinksAria: "Linki społeczności",
  contribute: "Współtwórz",
};
