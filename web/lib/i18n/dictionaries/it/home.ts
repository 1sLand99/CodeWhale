import type { HomeDict } from "../types";

/**
 * Italian home dictionary — native copy for the Tidal Folio landing page,
 * in the current direction: your models, more capable together; agents
 * and control on your own machine; availability stated per surface as it
 * is today. Product vocabulary stays literal (Plan / Work / Operate, Ask /
 * Auto-Review / Full Access, Codewhale, TUI, codewhale exec, Fleet).
 */

export const home: HomeDict = {
  metaTitle: "Codewhale — Crea quello che vuoi.",
  metaDescription:
    "Crea software, lavora con i file e automatizza le attività con Codewhale. Scegli modelli ospitati o locali e cambia provider in base alle tue esigenze.",
  kicker: "Agenti di IA open source",
  heroTitleA: "Crea quello che vuoi.",
  heroTitleB: "Scegli i tuoi modelli.",
  heroIntro:
    "{brand} ti offre agenti che possono creare software, lavorare con i file e automatizzare le attività. Usa i modelli che scegli e cambia provider in base alle tue esigenze.",
  getCodewhale: "Ottieni Codewhale",
  exploreProduct: "Esplora il prodotto",
  shotPreview: "Anteprima del terminale",
  shotBuild: "build di sviluppo v{version}",
  screenshotAlt:
    "Build di sviluppo di Codewhale v0.9.12 in un terminale: il marchio della balena in braille, una nuova sessione senza cronologia, il compositore di messaggi e un piè di pagina che mostra Full Access, modalità Work, due attività pianificate, server MCP in connessione e il modello GLM-5.3 al massimo sforzo",
  latestRelease: "Ultima release {tag}",
  releaseUnavailable: "Stato delle release non disponibile",
  currentSource: "Sorgente",
  sourceCandidate: "Non rilasciata",
  providerRoutes: "{count} provider",
  publishedRelease: "rilasciata",
  figcaptionSourceCandidate: "non rilasciata",
  chapterTerminal: "Il tuo terminale",
  chapterTerminalTitle: "Inizia da qualcosa che vuoi creare.",
  gainHeading: "Metti in pratica le tue idee.",
  gainLede:
    "Crea un progetto, cerca una risposta o automatizza un’attività. Inizia con un agente e distribuisci i lavori più grandi tra più agenti.",
  gain: [
    [
      "Crea qualcosa",
      "Trasforma un’idea in software funzionante. I tuoi agenti possono modificare file, eseguire comandi e verificare il risultato."
    ],
    [
      "Automatizza le attività ripetitive",
      "Crea script e flussi di lavoro per le attività ricorrenti, poi eseguili dal terminale."
    ],
    [
      "Scegli i tuoi modelli",
      "Collega modelli ospitati o locali. Assegna parti di un lavoro più grande ad agenti con modelli e ruoli diversi."
    ]
  ],
  chapterModels: "I tuoi modelli",
  modelsHeading: "Trova un modello adatto all’attività.",
  modelsBody:
    "Usa un provider di modelli ospitati, collegati tramite un gateway o esegui un modello in locale. Scegli un provider e un modello per ogni sessione e cambiali mentre lavori.",
  modelsFacts: [
    ["Hosted", "La tua chiave API, salvata con codewhale auth set --provider <id>"],
    ["Gateway", "Un endpoint per molti modelli, il provider lo scegli sempre tu"],
    ["Locale", "vLLM, SGLang, Ollama su localhost — di solito senza chiave"],
  ],
  modelsLink: "Esplora modelli e provider",
  startHeading: "Inizia la tua prima attività.",
  startLede:
    "Installa Codewhale, collega un modello e digli cosa vuoi fare. Aggiungi un Fleet quando vuoi distribuire il lavoro tra più agenti.",
  startGuideLink: "Leggi la guida introduttiva",
  startVocabularyLink: "Vedi il vocabolario del prodotto",
  chapterAccount: "Ottieni Codewhale",
  availabilityHeading: "Dove usare Codewhale.",
  availabilityLede:
    "Inizia dal terminale. L’app e i computer cloud sono in sviluppo.",
  availability: [
    [
      "Terminale",
      "Rilasciato",
      "Binari delle versioni pubblicate su GitHub per Linux, macOS e Windows; npm e Cargo sono alternative. Android su Termux è disponibile in anteprima."
    ],
    [
      "App web",
      "Anteprima di sviluppo",
      "Accesso all’account e abbinamento con il browser nell’anteprima di sviluppo."
    ],
    [
      "Desktop",
      "Build di sviluppo",
      "L’app per macOS è in sviluppo; il download pubblico arriverà in seguito."
    ],
    [
      "Computer cloud",
      "In sviluppo",
      "Computer ospitati per eseguire le tue attività."
    ]
  ],
  availabilityNote:
    "Il terminale funziona senza un account Codewhale. L’utilizzo dei modelli ospitati viene fatturato dal tuo provider.",
  accountLink: "Crea un account",
  surfacesHeading: "Usa il runtime dove avviene il lavoro.",
  surfaces: [
    ["TUI", "Lavoro interattivo nel terminale"],
    ["codewhale exec", "Script e CI"],
    ["Client web locale","Interfaccia su localhost; ambiente di lavoro web ospitato in sviluppo"],
    ["Runtime API + MCP", "Integrazioni locali"],
    ["Fleet","Più agenti su un unico lavoro"],
  ],
  runtimeLink: "Esplora le integrazioni",
  installBandHeading: "Inizia con un solo comando.",
  copy: "Copia",
  copied: "Copiato ✓",
  binaries: "Binari",
  chinaMirrors: "Mirror in Cina",
  installGuideLink: "Leggi la guida d'installazione",
  communityHeading: "Costruito in pubblico",
  communityBody:
    "Con licenza MIT e plasmato da contributor su runtime, provider, piattaforme, documentazione e test.",
  communityLinksAria: "Link della community",
  contribute: "Contribuisci",
};
