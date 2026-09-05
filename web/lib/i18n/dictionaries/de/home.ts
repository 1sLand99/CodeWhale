import type { HomeDict } from "../types";

/**
 * German home dictionary — native copy for the Tidal Folio landing page,
 * in the current direction: your models, more capable together; agents
 * and control on your own machine; availability stated per surface as it
 * is today. Product vocabulary stays literal (Plan / Work / Operate, Ask /
 * Auto-Review / Full Access, Codewhale, TUI, codewhale exec, Fleet).
 */

export const home: HomeDict = {
  metaTitle: "Codewhale — Deine Modelle. Gemeinsam leistungsfähiger.",
  metaDescription:
    "Codewhale ist ein quelloffenes System für agentisches Computing. Bring die Modelle mit, die du schon nutzt — gehostet, über ein Gateway oder lokal — und lass sie in deinem Terminal zusammenarbeiten, auf deiner Maschine, unter deiner Kontrolle. Rust, MIT.",
  kicker: "Agentisches Computing, zu deinen Bedingungen",
  heroTitleA: "Deine Modelle.",
  heroTitleB: "Gemeinsam leistungsfähiger.",
  heroIntro:
    "{brand} holt die Modelle, die du schon nutzt, in ein Terminal und lässt sie wie eine Mannschaft arbeiten — Code lesen, Dateien ändern, Prüfungen laufen lassen —, während du entscheidest, was jedes davon darf. Open Source, auf deiner Maschine.",
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
  chapterTerminalTitle: "Ein vertrauter Ort zum Anfangen.",
  gainHeading:
    "Was du bekommst, ist kein Chatbot. Es ist Hebelwirkung für die Modelle, die du ohnehin bezahlst.",
  gainLede:
    "Eine Sitzung kann mehrere Modelle zugleich halten, jedes in der Rolle, die du ihm gegeben hast, alle im selben Repository nach denselben Regeln.",
  gain: [
    ["Deine Modelle", "Gehostete Schlüssel, ein Gateway oder eine lokale Runtime ganz ohne Schlüssel. Pinne jeder Rolle ein anderes Modell an und behalte den Provider, den du gewählt hast — ein Modellname wechselt ihn nie für dich."],
    ["Fähige Agenten", "Die Modi Plan, Work und Operate; ein fleet aus Unteragenten für eine Aufgabe; Werkzeuge für Dateien, Shell, Web und MCP; Sitzungen, die sich speichern, fortsetzen und zurückrollen lassen."],
    ["Kontrolle auf deiner Maschine", "Ask, Auto-Review oder Full Access — du legst fest, wie viel er tut, bevor er fragt. Läuft lokal, in einer Sandbox, wo das Betriebssystem es erlaubt, mit einem Audit-Log, das du lesen kannst."],
  ],
  chapterModels: "Deine Modelle",
  modelsHeading: "Bring mit, was du hast. Ändere nichts, was du nicht gewählt hast.",
  modelsBody:
    "Codewhale bringt {count} Provider mit und behandelt sie als Gleiche. Speichere einen Schlüssel einmal, nenne ein Modell, und die Route bleibt genau so, wie du sie gesetzt hast. Lokale Modelle über vLLM, SGLang oder Ollama brauchen keinen Schlüssel.",
  modelsFacts: [
    ["Gehostet", "Dein eigener API-Schlüssel, gespeichert mit codewhale auth set"],
    ["Gateway", "Ein Endpoint für viele Modelle, den Provider wählst weiterhin du"],
    ["Lokal", "vLLM, SGLang, Ollama auf localhost — meist ohne Schlüssel"],
  ],
  modelsLink: "Alle Provider ansehen",
  startHeading: "Vier Schritte bis zur ersten Sitzung.",
  startLede:
    "Installieren, eine Sitzung ohne Schlüssel öffnen, einen Provider verbinden und dann einen fleet einrichten, wenn ein Modell nicht reicht.",
  startGuideLink: "Leitfaden für die ersten Schritte lesen",
  startVocabularyLink: "Produktvokabular ansehen",
  chapterAccount: "Wo es heute läuft",
  availabilityHeading: "Verfügbar, in Entwicklung und noch nicht — klar benannt.",
  availabilityLede:
    "Das Terminal ist das veröffentlichte Produkt. Alles andere steht hier in dem Zustand, in dem es tatsächlich ist.",
  availability: [
    ["Terminal", "Veröffentlicht", "GitHub-Release-Binaries für Linux, macOS und Windows; npm und Cargo sind Alternativen. Android unter Termux ist eine Vorschau."],
    ["Web-App", "Anmeldung und Fernsteuerung verfügbar", "Melde dich an oder erstelle ein Konto und tippe /rc in einer laufenden lokalen Sitzung, um genau diese Sitzung im Browser fortzusetzen. Der Rest der Werkbank im Browser ist eine Entwicklungsvorschau."],
    ["Desktop", "Entwicklungsbuild", "Für macOS, Linux und Windows gibt es Alpha-Builds. Eine veröffentlichte Desktop-App gibt es noch nicht."],
    ["Cloud-Computer", "Noch nicht verfügbar", "Arbeit auf einem gehosteten Computer auszuführen ist in Entwicklung. Diese Seite sagt es, sobald es funktioniert."],
  ],
  availabilityNote:
    "Für das Terminal ist kein Konto nötig. Ein Konto ist für sich nie ein bezahlter Tarif, und nichts auf dieser Seite kann dir etwas berechnen.",
  accountLink: "Konto erstellen",
  surfacesHeading: "Nutze die Runtime dort, wo die Arbeit passiert.",
  surfaces: [
    ["TUI", "Interaktive Arbeit im Terminal"],
    ["codewhale exec", "Skripte und CI"],
    ["Web-Client", "Browser-Client, nur Loopback"],
    ["Runtime API + MCP", "Lokale Integrationen"],
    ["fleet", "Dauerhafte Multi-Agenten-Arbeit"],
  ],
  runtimeLink: "Runtime-Oberflächen und Stabilitätshinweise ansehen",
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
