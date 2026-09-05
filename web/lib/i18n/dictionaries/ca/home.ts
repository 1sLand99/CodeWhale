import type { HomeDict } from "../types";

/**
 * Catalan home dictionary — native copy for the Tidal Folio landing page,
 * in the current direction: your models, more capable together; agents
 * and control on your own machine; availability stated per surface as it
 * is today. Product vocabulary stays literal (Plan / Work / Operate, Ask /
 * Auto-Review / Full Access, Codewhale, TUI, codewhale exec, Fleet).
 */

export const home: HomeDict = {
  metaTitle: "Codewhale — Els teus models. Més capaços junts.",
  metaDescription:
    "Codewhale és un sistema de computació agèntica de codi obert. Porta els models que ja fas servir —allotjats, per gateway o locals— i fes-los treballar junts al teu terminal, a la teva màquina, sota el teu control. Rust, MIT.",
  kicker: "Computació agèntica, en els teus termes",
  heroTitleA: "Els teus models.",
  heroTitleB: "Més capaços junts.",
  heroIntro:
    "{brand} reuneix els models que ja fas servir en un sol terminal i els fa treballar com una tripulació —llegint el teu codi, editant, executant les comprovacions— mentre tu decideixes què pot fer cadascun. Codi obert, a la teva màquina.",
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
  chapterTerminalTitle: "Un lloc familiar per començar.",
  gainHeading:
    "El que obtens no és un chatbot. És palanca sobre els models que ja pagues.",
  gainLede:
    "Una sessió pot tenir diversos models alhora, cadascun en el rol que li has donat, tots treballant al mateix repositori amb les mateixes regles.",
  gain: [
    ["Els teus models", "Claus allotjades, un gateway o un runtime local sense cap clau. Fixa un model diferent a cada rol i conserva el proveïdor que has triat: un nom de model mai el canvia per tu."],
    ["Agents capaços", "Modes Plan, Work i Operate; un fleet de subagents per a una mateixa feina; eines per a fitxers, shell, web i MCP; sessions que es desen, es reprenen i es reverteixen."],
    ["Control a la teva màquina", "Ask, Auto-Review o Full Access: tu fixes quant fa abans de preguntar. S’executa en local, en sandbox on el sistema ho permet, amb un registre d’auditoria que pots llegir."],
  ],
  chapterModels: "Els teus models",
  modelsHeading: "Porta el que tens. No canviïs res que no hagis triat.",
  modelsBody:
    "Connecta un proveïdor allotjat compatible, una passarel·la o un servidor de models local. Revisa el proveïdor i el model abans de començar. Un servidor local pot requerir autenticació.",
  modelsFacts: [
    ["Allotjat", "La teva pròpia clau d’API, desada amb codewhale auth set"],
    ["Gateway", "Un endpoint per a molts models; el proveïdor el segueixes triant tu"],
    ["Local", "vLLM, SGLang, Ollama a localhost; normalment sense clau"],
  ],
  modelsLink: "Veure tots els proveïdors",
  startHeading: "Quatre passos fins a la primera sessió.",
  startLede:
    "Instal·la, obre una sessió sense clau, connecta un proveïdor i després configura un fleet quan un model no sigui prou.",
  startGuideLink: "Llegeix la guia d’inici",
  startVocabularyLink: "Consulta el vocabulari del producte",
  chapterAccount: "On funciona avui",
  availabilityHeading: "Disponible ara, en desenvolupament i encara no: dit clarament.",
  availabilityLede:
    "El terminal és el producte publicat. Tota la resta apareix amb l’estat en què realment es troba.",
  availability: [
    ["Terminal", "Publicat", "Binaris de GitHub Releases per a Linux, macOS i Windows; npm i Cargo són alternatives. Android amb Termux és una vista prèvia."],
    ["Aplicació web", "Inici de sessió i control remot disponibles", "Inicia sessió o crea un compte i escriu /rc en una sessió local en marxa per continuar exactament aquella sessió des del navegador. La resta del banc de treball al navegador continua sent una vista prèvia de desenvolupament."],
    ["Escriptori", "Build de desenvolupament", "Hi ha builds alfa per a macOS, Linux i Windows. Encara no hi ha cap aplicació d’escriptori publicada."],
    ["Ordinadors al núvol", "Encara no disponible", "Executar feina en un ordinador allotjat està en desenvolupament. Aquesta pàgina ho dirà quan funcioni."],
  ],
  availabilityNote:
    "El terminal no necessita compte. Un compte mai és per si sol un pla de pagament, i res en aquest lloc et pot cobrar.",
  accountLink: "Crear un compte",
  surfacesHeading: "Fes servir el runtime on passa la feina.",
  surfaces: [
    ["TUI", "Treball interactiu al terminal"],
    ["codewhale exec", "Scripts i CI"],
    ["Client web", "Client de navegador, només loopback"],
    ["Runtime API + MCP", "Integracions locals"],
    ["fleet", "Feina multiagent durable"],
  ],
  runtimeLink: "Veure les superfícies del runtime i les notes d’estabilitat",
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
