import type { HomeDict } from "../types";

/**
 * Italian home dictionary — native copy for the Tidal Folio landing page,
 * in the current direction: your models, more capable together; agents
 * and control on your own machine; availability stated per surface as it
 * is today. Product vocabulary stays literal (Plan / Work / Operate, Ask /
 * Auto-Review / Full Access, Codewhale, TUI, codewhale exec, Fleet).
 */

export const home: HomeDict = {
  metaTitle: "Codewhale — I tuoi modelli. Più capaci insieme.",
  metaDescription:
    "Codewhale è un sistema di calcolo agentico open source. Porta i modelli che già usi — hosted, tramite gateway o locali — e mettili a lavorare insieme nel tuo terminale, sulla tua macchina, sotto il tuo controllo. Rust, MIT.",
  kicker: "Calcolo agentico, alle tue condizioni",
  heroTitleA: "I tuoi modelli.",
  heroTitleB: "Più capaci insieme.",
  heroIntro:
    "{brand} riunisce i modelli che già usi in un solo terminale e li fa lavorare come un equipaggio — leggere il codice, modificare, eseguire i controlli — mentre tu decidi cosa può fare ciascuno. Open source, sulla tua macchina.",
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
  chapterTerminalTitle: "Un posto familiare da cui iniziare.",
  gainHeading: "Quello che ottieni non è un chatbot. È leva sui modelli che già paghi.",
  gainLede:
    "Una sessione può tenere più modelli insieme, ciascuno nel ruolo che gli hai dato, tutti al lavoro nello stesso repository con le stesse regole.",
  gain: [
    ["I tuoi modelli", "Chiavi hosted, un gateway o un runtime locale senza alcuna chiave. Fissa un modello diverso a ogni ruolo e mantieni il provider che hai scelto: un nome di modello non lo cambia mai al posto tuo."],
    ["Agenti capaci", "Modalità Plan, Work e Operate; un fleet di sub-agenti per un solo lavoro; strumenti per file, shell, web e MCP; sessioni che si salvano, riprendono e tornano indietro."],
    ["Controllo sulla tua macchina", "Ask, Auto-Review o Full Access: sei tu a stabilire quanto fa prima di chiedere. Gira in locale, in sandbox dove il sistema lo consente, con un registro di audit che puoi leggere."],
  ],
  chapterModels: "I tuoi modelli",
  modelsHeading: "Porta quello che hai. Non cambiare nulla che tu non abbia scelto.",
  modelsBody:
    "Codewhale include {count} provider e li tratta da pari. Salva una chiave una volta, indica un modello e la rotta resta esattamente come l’hai impostata. I modelli locali su vLLM, SGLang o Ollama non hanno bisogno di chiave.",
  modelsFacts: [
    ["Hosted", "La tua chiave API, salvata con codewhale auth set"],
    ["Gateway", "Un endpoint per molti modelli, il provider lo scegli sempre tu"],
    ["Locale", "vLLM, SGLang, Ollama su localhost — di solito senza chiave"],
  ],
  modelsLink: "Vedi tutti i provider",
  startHeading: "Quattro passi fino alla prima sessione.",
  startLede:
    "Installa, apri una sessione senza chiave, collega un provider e poi configura un fleet quando un modello non basta.",
  startGuideLink: "Leggi la guida introduttiva",
  startVocabularyLink: "Vedi il vocabolario del prodotto",
  chapterAccount: "Dove gira oggi",
  availabilityHeading: "Disponibile ora, in sviluppo e non ancora — detto chiaramente.",
  availabilityLede:
    "Il terminale è il prodotto rilasciato. Tutto il resto è elencato nello stato in cui si trova davvero.",
  availability: [
    ["Terminale", "Rilasciato", "npm, Cargo e binari precompilati per Linux, macOS e Windows. Android su Termux è un’anteprima."],
    ["App web", "Accesso e controllo remoto disponibili", "Accedi o crea un account, poi digita /rc in una sessione locale in corso per continuare proprio quella sessione dal browser. Il resto del banco di lavoro nel browser è ancora un’anteprima di sviluppo."],
    ["Desktop", "Build di sviluppo", "Esistono build alfa per macOS, Linux e Windows. Non c’è ancora un’app desktop rilasciata."],
    ["Computer cloud", "Non ancora disponibile", "Eseguire lavoro su un computer hosted è in sviluppo. Questa pagina lo dirà quando funzionerà."],
  ],
  availabilityNote:
    "Il terminale non richiede un account. Un account non è mai di per sé un piano a pagamento, e nulla su questo sito può addebitarti qualcosa.",
  accountLink: "Crea un account",
  surfacesHeading: "Usa il runtime dove avviene il lavoro.",
  surfaces: [
    ["TUI", "Lavoro interattivo nel terminale"],
    ["codewhale exec", "Script e CI"],
    ["Client web", "Client browser, solo in loopback"],
    ["Runtime API + MCP", "Integrazioni locali"],
    ["fleet", "Lavoro multi-agente durevole"],
  ],
  runtimeLink: "Vedi le superfici del runtime e le note di stabilità",
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
