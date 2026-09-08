import type { HomeDict } from "../types";

/**
 * Catalan home dictionary — native copy for the Tidal Folio landing page,
 * in the current direction: your models, more capable together; agents
 * and control on your own machine; availability stated per surface as it
 * is today. Product vocabulary stays literal (Plan / Work / Operate, Ask /
 * Auto-Review / Full Access, Codewhale, TUI, codewhale exec, Fleet).
 */

export const home: HomeDict = {
  metaTitle: "Codewhale — Crea el que vulguis.",
  metaDescription:
    "Crea programari, treballa amb fitxers i automatitza tasques amb Codewhale. Tria models allotjats o locals i canvia de proveïdor segons les teves necessitats.",
  kicker: "Agents d’IA de codi obert",
  heroTitleA: "Crea el que vulguis.",
  heroTitleB: "Tria els teus models.",
  heroIntro:
    "{brand} et proporciona agents que poden crear programari, treballar amb fitxers i automatitzar tasques. Fes servir els models que triïs i canvia de proveïdor segons les teves necessitats.",
  getCodewhale: "Obtenir Codewhale",
  exploreProduct: "Explorar el producte",
  shotPreview: "Vista prèvia del terminal",
  shotBuild: "build de desenvolupament v{version}",
  screenshotAlt:
    "Build de desenvolupament de Codewhale v0.9.12 en un terminal: la marca de la balena en braille, una sessió nova sense historial, el compositor de missatges i un peu que mostra Full Access, mode Work, dues tasques programades, servidors MCP connectant-se i el model GLM-5.3 al màxim esforç",
  latestRelease: "Última versió {tag}",
  releaseUnavailable: "Estat de la versió no disponible",
  currentSource: "Font",
  sourceCandidate: "Sense publicar",
  providerRoutes: "{count} proveïdors",
  publishedRelease: "publicada",
  figcaptionSourceCandidate: "sense publicar",
  chapterTerminal: "El teu terminal",
  chapterTerminalTitle: "Comença amb alguna cosa que vulguis crear.",
  gainHeading:
    "Posa les teves idees en marxa.",
  gainLede:
    "Crea un projecte, investiga una qüestió o automatitza una tasca. Comença amb un agent i reparteix les feines més grans entre diversos.",
  gain: [
    [
      "Crea alguna cosa",
      "Converteix una idea en programari que funcioni. Els teus agents poden editar fitxers, executar ordres i comprovar el resultat."
    ],
    [
      "Automatitza les tasques repetitives",
      "Crea scripts i fluxos de treball per a les tasques que repeteixes i executa’ls des del terminal."
    ],
    [
      "Tria els teus models",
      "Connecta models allotjats o locals. Reparteix les parts d’una feina més gran entre agents amb models i rols diferents."
    ]
  ],
  chapterModels: "Els teus models",
  modelsHeading: "Troba un model adequat per a la tasca.",
  modelsBody:
    "Fes servir un proveïdor de models allotjats, connecta’t a través d’una passarel·la o executa un model en local. Tria un proveïdor i un model per a cada sessió i canvia’ls mentre treballes.",
  modelsFacts: [
    ["Allotjat", "La teva pròpia clau d’API, desada amb codewhale auth set --provider <id>"],
    ["Gateway", "Un endpoint per a molts models; el proveïdor el segueixes triant tu"],
    ["Local", "vLLM, SGLang, Ollama a localhost; normalment sense clau"],
  ],
  modelsLink: "Explora els models i els proveïdors",
  startHeading: "Comença la teva primera tasca.",
  startLede:
    "Instal·la Codewhale, connecta un model i digues-li què vols fer. Afegeix un Fleet quan vulguis repartir la feina entre diversos agents.",
  startGuideLink: "Llegeix la guia d’inici",
  startVocabularyLink: "Consulta el vocabulari del producte",
  chapterAccount: "Obtenir Codewhale",
  availabilityHeading: "On fer servir Codewhale.",
  availabilityLede:
    "Comença al terminal. L’aplicació i els ordinadors al núvol estan en desenvolupament.",
  availability: [
    [
      "Terminal",
      "Publicat",
      "Binaris de les versions publicades a GitHub per a Linux, macOS i Windows; npm i Cargo són alternatives. Android amb Termux és una vista prèvia."
    ],
    [
      "Aplicació web",
      "Vista prèvia de desenvolupament",
      "Accés al compte i vinculació amb el navegador a la vista prèvia de desenvolupament."
    ],
    [
      "Escriptori",
      "Build de desenvolupament",
      "L’aplicació per a macOS està en desenvolupament; la descàrrega pública arribarà més endavant."
    ],
    [
      "Ordinadors al núvol",
      "En desenvolupament",
      "Ordinadors allotjats per executar les teves tasques."
    ]
  ],
  availabilityNote:
    "El terminal funciona sense un compte de Codewhale. El teu proveïdor factura l’ús dels models allotjats.",
  accountLink: "Crear un compte",
  surfacesHeading: "Fes servir el runtime on passa la feina.",
  surfaces: [
    ["TUI", "Treball interactiu al terminal"],
    ["codewhale exec", "Scripts i CI"],
    ["Client web local","Interfície a localhost; espai de treball web allotjat en desenvolupament"],
    ["Runtime API + MCP", "Integracions locals"],
    ["Fleet","Diversos agents en una mateixa feina"],
  ],
  runtimeLink: "Explora les integracions",
  installBandHeading: "Comença amb una sola ordre.",
  copy: "Copia",
  copied: "Copiat ✓",
  binaries: "Binaris",
  chinaMirrors: "Mirrors a la Xina",
  installGuideLink: "Llegeix la guia d’instal·lació",
  communityHeading: "Construït en públic",
  communityBody:
    "Amb llicència MIT i format per col·laboradors de runtimes, proveïdors, plataformes, documentació i tests.",
  communityLinksAria: "Enllaços de la comunitat",
  contribute: "Col·labora",
};
