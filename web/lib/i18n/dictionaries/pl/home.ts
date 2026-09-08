import type { HomeDict } from "../types";

/**
 * Polish home dictionary — native copy for the Tidal Folio landing page,
 * in the current direction: your models, more capable together; agents
 * and control on your own machine; availability stated per surface as it
 * is today. Product vocabulary stays literal (Plan / Work / Operate, Ask /
 * Auto-Review / Full Access, Codewhale, TUI, codewhale exec, Fleet).
 */

export const home: HomeDict = {
  metaTitle: "Codewhale — Twórz to, co chcesz.",
  metaDescription:
    "Twórz oprogramowanie, pracuj z plikami i automatyzuj zadania z Codewhale. Wybieraj modele hostowane lub lokalne i zmieniaj dostawców wraz ze zmianą zadań.",
  kicker: "Agenci AI z otwartym kodem",
  heroTitleA: "Twórz to, co chcesz.",
  heroTitleB: "Wybieraj modele.",
  heroIntro:
    "{brand} daje Ci agentów, którzy mogą tworzyć oprogramowanie, pracować z plikami i automatyzować zadania. Korzystaj z wybranych przez siebie modeli i zmieniaj dostawców wraz ze zmianą zadań.",
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
  chapterTerminalTitle: "Zacznij od tego, co chcesz stworzyć.",
  gainHeading:
    "Wprowadzaj pomysły w życie.",
  gainLede:
    "Stwórz projekt, zbadaj zagadnienie lub zautomatyzuj zadanie. Zacznij od jednego agenta, a większą pracę rozdzielaj między kilku.",
  gain: [
    [
      "Stwórz coś",
      "Zamień pomysł w działające oprogramowanie. Twoi agenci mogą edytować pliki, uruchamiać polecenia i sprawdzać wynik."
    ],
    [
      "Automatyzuj powtarzalną pracę",
      "Twórz skrypty i procesy dla powtarzających się zadań, a potem uruchamiaj je z terminala."
    ],
    [
      "Wybieraj modele",
      "Podłącz modele hostowane lub lokalne. Przydzielaj części większego zadania agentom z różnymi modelami i rolami."
    ]
  ],
  chapterModels: "Twoje modele",
  modelsHeading: "Znajdź model pasujący do zadania.",
  modelsBody:
    "Korzystaj z dostawcy modeli hostowanych, łącz się przez bramkę lub uruchamiaj model lokalnie. Wybieraj dostawcę i model dla każdej sesji i zmieniaj je podczas pracy.",
  modelsFacts: [
    ["Hostowane", "Twój własny klucz API zapisany przez codewhale auth set --provider <id>"],
    ["Bramka", "Jeden endpoint do wielu modeli, dostawcę nadal wybierasz Ty"],
    ["Lokalne", "vLLM, SGLang, Ollama na localhost — zwykle bez klucza"],
  ],
  modelsLink: "Poznaj modele i dostawców",
  startHeading: "Rozpocznij pierwsze zadanie.",
  startLede:
    "Zainstaluj Codewhale, podłącz model i powiedz, co chcesz zrobić. Dodaj Fleet, gdy zechcesz rozdzielić pracę między kilku agentów.",
  startGuideLink: "Przeczytaj przewodnik na start",
  startVocabularyLink: "Zobacz słownik produktu",
  chapterAccount: "Pobierz Codewhale",
  availabilityHeading: "Gdzie korzystać z Codewhale.",
  availabilityLede:
    "Zacznij w terminalu. Aplikacja i komputery w chmurze są w przygotowaniu.",
  availability: [
    [
      "Terminal",
      "Wydany",
      "Gotowe pliki binarne z wydań GitHub dla systemów Linux, macOS i Windows; npm i Cargo to alternatywy. Android w Termux to wersja podglądowa."
    ],
    [
      "Aplikacja webowa",
      "Podgląd deweloperski",
      "Dostęp do konta i parowanie z przeglądarką w podglądzie deweloperskim."
    ],
    [
      "Aplikacja desktopowa",
      "Kompilacja deweloperska",
      "Aplikacja na macOS jest w przygotowaniu; publiczna wersja do pobrania pojawi się później."
    ],
    [
      "Komputery w chmurze",
      "W przygotowaniu",
      "Komputery w chmurze do wykonywania Twoich zadań."
    ]
  ],
  availabilityNote:
    "Terminal działa bez konta Codewhale. Za korzystanie z modeli hostowanych opłaty nalicza Twój dostawca.",
  accountLink: "Załóż konto",
  surfacesHeading: "Używaj runtime'u tam, gdzie odbywa się praca.",
  surfaces: [
    ["TUI", "Interaktywna praca w terminalu"],
    ["codewhale exec", "Skrypty i CI"],
    ["Lokalny klient webowy","Interfejs na localhost; hostowane środowisko pracy w przeglądarce jest w przygotowaniu"],
    ["Runtime API + MCP", "Lokalne integracje"],
    ["Fleet","Kilku agentów przy jednym zadaniu"],
  ],
  runtimeLink: "Poznaj integracje",
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
