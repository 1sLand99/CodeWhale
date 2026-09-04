import type { HomeDict } from "../types";

/**
 * French home dictionary — native copy for the Tidal Folio landing page,
 * in the current direction: your models, more capable together; agents
 * and control on your own machine; availability stated per surface as it
 * is today. Product vocabulary stays literal (Plan / Work / Operate, Ask /
 * Auto-Review / Full Access, Codewhale, TUI, codewhale exec, Fleet).
 */

export const home: HomeDict = {
  metaTitle: "Codewhale — Vos modèles. Plus capables ensemble.",
  metaDescription:
    "Codewhale est un système d’informatique agentique open source. Apportez les modèles que vous utilisez déjà — hébergés, via une passerelle ou en local — et faites-les travailler ensemble dans votre terminal, sur votre machine, sous votre contrôle. Rust, MIT.",
  kicker: "L’informatique agentique, à vos conditions",
  heroTitleA: "Vos modèles.",
  heroTitleB: "Plus capables ensemble.",
  heroIntro:
    "{brand} réunit les modèles que vous utilisez déjà dans un seul terminal et les fait travailler comme un équipage — lire votre code, éditer, lancer les vérifications — pendant que vous décidez de ce que chacun a le droit de faire. Open source, sur votre machine.",
  getCodewhale: "Obtenir Codewhale",
  exploreProduct: "Découvrir le produit",
  shotPreview: "Aperçu du terminal",
  shotBuild: "build de développement v{version}",
  screenshotAlt:
    "Build de développement de Codewhale v0.9.12 dans un terminal : la baleine en braille, une nouvelle session sans historique, le compositeur de message et un pied de page indiquant Full Access, le mode Work, deux tâches planifiées, des serveurs MCP en connexion et le modèle GLM-5.3 à l’effort maximal",
  latestRelease: "Dernière version {tag}",
  releaseUnavailable: "État des versions indisponible",
  currentSource: "Source",
  sourceCandidate: "Non publiée",
  providerRoutes: "{count} fournisseurs",
  publishedRelease: "publiée",
  figcaptionSourceCandidate: "non publiée",
  chapterTerminal: "Votre terminal",
  chapterTerminalTitle: "Un endroit familier pour commencer.",
  gainHeading:
    "Ce que vous obtenez n’est pas un chatbot. C’est un levier sur les modèles que vous payez déjà.",
  gainLede:
    "Une session peut tenir plusieurs modèles à la fois, chacun dans le rôle que vous lui avez donné, tous au travail dans le même dépôt selon les mêmes règles.",
  gain: [
    ["Vos modèles", "Des clés hébergées, une passerelle ou un runtime local sans aucune clé. Épinglez un modèle différent à chaque rôle et gardez le fournisseur que vous avez choisi — un nom de modèle ne le change jamais à votre place."],
    ["Des agents capables", "Les modes Plan, Work et Operate ; un fleet de sous-agents pour une même tâche ; des outils pour les fichiers, le shell, le web et MCP ; des sessions qui se sauvegardent, reprennent et se restaurent."],
    ["Le contrôle sur votre machine", "Ask, Auto-Review ou Full Access — c’est vous qui fixez ce qu’il fait avant de demander. Tourne en local, en bac à sable là où l’OS le permet, avec un journal d’audit que vous pouvez lire."],
  ],
  chapterModels: "Vos modèles",
  modelsHeading: "Apportez ce que vous avez. Ne changez rien que vous n’ayez choisi.",
  modelsBody:
    "Codewhale est livré avec {count} fournisseurs et les traite en égaux. Enregistrez une clé une fois, nommez un modèle, et la route reste exactement telle que vous l’avez fixée. Les modèles locaux via vLLM, SGLang ou Ollama n’ont pas besoin de clé.",
  modelsFacts: [
    ["Hébergé", "Votre propre clé d’API, enregistrée avec codewhale auth set"],
    ["Passerelle", "Un seul endpoint pour de nombreux modèles, le fournisseur reste votre choix"],
    ["Local", "vLLM, SGLang, Ollama sur localhost — généralement sans clé"],
  ],
  modelsLink: "Voir tous les fournisseurs",
  startHeading: "Quatre étapes jusqu’à une première session.",
  startLede:
    "Installez, ouvrez une session sans clé, connectez un fournisseur, puis mettez en place un fleet quand un seul modèle ne suffit plus.",
  startGuideLink: "Lire le guide de démarrage",
  startVocabularyLink: "Voir le vocabulaire du produit",
  chapterAccount: "Où ça tourne aujourd’hui",
  availabilityHeading: "Disponible, en développement, pas encore — dit simplement.",
  availabilityLede:
    "Le terminal est le produit publié. Tout le reste est listé dans l’état où il se trouve réellement.",
  availability: [
    ["Terminal", "Publié", "npm, Cargo et binaires précompilés pour Linux, macOS et Windows. Android sous Termux est en aperçu."],
    ["Application web", "Connexion au compte disponible", "Connectez-vous ou créez un compte sur app.codewhale.net. L’établi dans le navigateur reste un aperçu de développement."],
    ["Bureau", "Build de développement", "Des builds alpha existent pour macOS, Linux et Windows. Il n’y a pas encore d’application de bureau publiée."],
    ["Ordinateurs cloud", "Pas encore disponible", "Exécuter du travail sur un ordinateur hébergé est en développement. Cette page le dira quand ça fonctionnera."],
  ],
  availabilityNote:
    "Aucun compte n’est nécessaire pour le terminal. Un compte n’est jamais un abonnement payant en soi, et rien sur ce site ne peut vous facturer.",
  accountLink: "Créer un compte",
  surfacesHeading: "Utilisez le runtime là où se fait le travail.",
  surfaces: [
    ["TUI", "Travail interactif dans le terminal"],
    ["codewhale exec", "Scripts et CI"],
    ["Client web", "Client navigateur en boucle locale uniquement"],
    ["Runtime API + MCP", "Intégrations locales"],
    ["fleet", "Travail multi-agents durable"],
  ],
  runtimeLink: "Voir les surfaces du runtime et les notes de stabilité",
  installBandHeading: "Commencez avec une seule commande.",
  copy: "Copier",
  copied: "Copié ✓",
  binaries: "Binaires",
  chinaMirrors: "Miroirs en Chine",
  installGuideLink: "Lire le guide d’installation",
  communityHeading: "Construit en public",
  communityBody:
    "Sous licence MIT et façonné par des contributeurs sur les runtimes, les fournisseurs, les plateformes, la documentation et les tests.",
  communityLinksAria: "Liens de la communauté",
  contribute: "Contribuer",
};
