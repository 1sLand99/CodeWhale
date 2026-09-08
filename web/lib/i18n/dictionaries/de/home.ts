import type { HomeDict } from "../types";

/**
 * German home dictionary — native copy for the Tidal Folio landing page,
 * in the current direction: your models, more capable together; agents
 * and control on your own machine; availability stated per surface as it
 * is today. Product vocabulary stays literal (Plan / Work / Operate, Ask /
 * Auto-Review / Full Access, Codewhale, TUI, codewhale exec, Fleet).
 */

export const home: HomeDict = {
  metaTitle: "Codewhale — Erstelle, was du möchtest.",
  metaDescription:
    "Entwickle Software, arbeite mit Dateien und automatisiere Aufgaben mit Codewhale. Wähle gehostete oder lokale Modelle und wechsle den Anbieter, wenn sich deine Aufgaben ändern.",
  kicker: "Open-Source-KI-Agenten",
  heroTitleA: "Erstelle, was du möchtest.",
  heroTitleB: "Wähle deine Modelle.",
  heroIntro:
    "{brand} gibt dir Agenten, die Software entwickeln, mit Dateien arbeiten und Aufgaben automatisieren können. Nutze die Modelle deiner Wahl und wechsle den Anbieter, wenn sich deine Aufgaben ändern.",
  getCodewhale: "Codewhale holen",
  exploreProduct: "Produkt ansehen",
  shotPreview: "Terminal-Vorschau",
  shotBuild: "Entwicklungsbuild v{version}",
  screenshotAlt:
    "Codewhale v0.9.12 Entwicklungsbuild in einem Terminal: die Wal-Marke aus Braille-Punkten, eine neue Sitzung ohne Verlauf, das Eingabefeld und eine Fußzeile mit Full Access, Modus Work, zwei geplanten Aufgaben, verbindenden MCP-Servern und dem Modell GLM-5.3 auf maximaler Stufe",
  latestRelease: "Aktuellstes Release {tag}",
  releaseUnavailable: "Release-Status nicht verfügbar",
  currentSource: "Quelle",
  sourceCandidate: "Unveröffentlicht",
  providerRoutes: "{count} Provider",
  publishedRelease: "veröffentlicht",
  figcaptionSourceCandidate: "unveröffentlicht",
  chapterTerminal: "Dein Terminal",
  chapterTerminalTitle: "Beginne mit etwas, das du erstellen möchtest.",
  gainHeading:
    "Setze deine Ideen um.",
  gainLede:
    "Entwickle ein Projekt, gehe einer Frage nach oder automatisiere eine Aufgabe. Beginne mit einem Agenten und verteile größere Aufgaben auf mehrere.",
  gain: [
    [
      "Entwickle etwas",
      "Mach aus einer Idee funktionierende Software. Deine Agenten können Dateien bearbeiten, Befehle ausführen und das Ergebnis prüfen."
    ],
    [
      "Automatisiere wiederkehrende Arbeit",
      "Erstelle Skripte und Abläufe für wiederkehrende Aufgaben und führe sie dann im Terminal aus."
    ],
    [
      "Wähle deine Modelle",
      "Verbinde gehostete oder lokale Modelle. Verteile Teile einer größeren Aufgabe auf Agenten mit unterschiedlichen Modellen und Rollen."
    ]
  ],
  chapterModels: "Deine Modelle",
  modelsHeading: "Finde ein Modell, das zur Aufgabe passt.",
  modelsBody:
    "Nutze einen Cloud-Anbieter, verbinde dich über ein Gateway oder führe ein Modell lokal aus. Wähle für jede Sitzung einen Anbieter und ein Modell und wechsle sie während der Arbeit.",
  modelsFacts: [
    ["Gehostet", "Dein eigener API-Schlüssel, gespeichert mit codewhale auth set --provider <id>"],
    ["Gateway", "Ein Endpoint für viele Modelle, den Provider wählst weiterhin du"],
    ["Lokal", "vLLM, SGLang, Ollama auf localhost — meist ohne Schlüssel"],
  ],
  modelsLink: "Modelle und Anbieter entdecken",
  startHeading: "Starte deine erste Aufgabe.",
  startLede:
    "Installiere Codewhale, verbinde ein Modell und beschreibe, was du tun möchtest. Füge ein Fleet hinzu, wenn mehrere Agenten die Arbeit unter sich aufteilen sollen.",
  startGuideLink: "Leitfaden für die ersten Schritte lesen",
  startVocabularyLink: "Produktvokabular ansehen",
  chapterAccount: "Codewhale holen",
  availabilityHeading: "Wo du Codewhale nutzen kannst.",
  availabilityLede:
    "Starte im Terminal. Die App und die Cloud-Computer sind in Entwicklung.",
  availability: [
    [
      "Terminal",
      "Veröffentlicht",
      "Binärdateien aus den GitHub-Releases für Linux, macOS und Windows; npm und Cargo sind Alternativen. Android unter Termux ist eine Vorschau."
    ],
    [
      "Web-App",
      "Entwicklungsvorschau",
      "Kontozugang und Kopplung mit dem Browser in der Entwicklungsvorschau."
    ],
    [
      "Desktop-App",
      "Entwicklungsbuild",
      "Die macOS-App ist in Entwicklung; ein öffentlicher Download folgt später."
    ],
    [
      "Cloud-Computer",
      "In Entwicklung",
      "Gehostete Computer zum Ausführen deiner Aufgaben."
    ]
  ],
  availabilityNote:
    "Das Terminal funktioniert ohne Codewhale-Konto. Die Nutzung gehosteter Modelle rechnet dein Anbieter ab.",
  accountLink: "Konto erstellen",
  surfacesHeading: "Nutze die Runtime dort, wo die Arbeit passiert.",
  surfaces: [
    ["TUI", "Interaktive Arbeit im Terminal"],
    ["codewhale exec", "Skripte und CI"],
    ["Lokaler Web-Client","Oberfläche auf localhost; gehostete Arbeitsumgebung im Browser in Entwicklung"],
    ["Runtime API + MCP", "Lokale Integrationen"],
    ["Fleet","Mehrere Agenten für dieselbe Aufgabe"],
  ],
  runtimeLink: "Integrationen entdecken",
  installBandHeading: "Starte mit einem einzigen Befehl.",
  copy: "Kopieren",
  copied: "Kopiert ✓",
  binaries: "Binärdateien",
  chinaMirrors: "China-Mirrors",
  installGuideLink: "Installationsleitfaden lesen",
  communityHeading: "Öffentlich gebaut",
  communityBody:
    "MIT-lizenziert und geprägt von Beitragenden aus Runtimes, Providern, Plattformen, Dokumentation und Tests.",
  communityLinksAria: "Community-Links",
  contribute: "Mitwirken",
};
