import type { HomeDict } from "../types";

/**
 * French home dictionary — native copy for the Tidal Folio landing page,
 * in the current direction: your models, more capable together; agents
 * and control on your own machine; availability stated per surface as it
 * is today. Product vocabulary stays literal (Plan / Work / Operate, Ask /
 * Auto-Review / Full Access, Codewhale, TUI, codewhale exec, Fleet).
 */

export const home: HomeDict = {
  metaTitle: "Codewhale — Créez ce que vous voulez.",
  metaDescription:
    "Créez des logiciels, travaillez sur des fichiers et automatisez des tâches avec Codewhale. Choisissez des modèles hébergés ou locaux et changez de fournisseur selon vos besoins.",
  kicker: "Des agents d’IA open source",
  heroTitleA: "Créez ce que vous voulez.",
  heroTitleB: "Choisissez vos modèles.",
  heroIntro:
    "{brand} vous donne des agents capables de créer des logiciels, de travailler sur des fichiers et d’automatiser des tâches. Utilisez les modèles de votre choix et changez de fournisseur selon vos besoins.",
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
  chapterTerminalTitle: "Commencez par ce que vous voulez créer.",
  gainHeading:
    "Donnez vie à vos idées.",
  gainLede:
    "Créez un projet, explorez une question ou automatisez une tâche. Commencez avec un agent et répartissez les travaux plus importants entre plusieurs agents.",
  gain: [
    [
      "Créez quelque chose",
      "Transformez une idée en un logiciel qui fonctionne. Vos agents peuvent modifier des fichiers, exécuter des commandes et vérifier le résultat."
    ],
    [
      "Automatisez les tâches répétitives",
      "Créez des scripts et des workflows pour les tâches récurrentes, puis lancez-les depuis le terminal."
    ],
    [
      "Choisissez vos modèles",
      "Connectez des modèles hébergés ou locaux. Répartissez un travail plus important entre des agents aux modèles et aux rôles différents."
    ]
  ],
  chapterModels: "Vos modèles",
  modelsHeading: "Trouvez un modèle adapté à la tâche.",
  modelsBody:
    "Utilisez un fournisseur de modèles hébergés, connectez-vous via une passerelle ou exécutez un modèle en local. Choisissez un fournisseur et un modèle pour chaque session, et changez-en au fil de votre travail.",
  modelsFacts: [
    ["Hébergé", "Votre propre clé d’API, enregistrée avec codewhale auth set --provider <id>"],
    ["Passerelle", "Un seul endpoint pour de nombreux modèles, le fournisseur reste votre choix"],
    ["Local", "vLLM, SGLang, Ollama sur localhost — généralement sans clé"],
  ],
  modelsLink: "Explorer les modèles et les fournisseurs",
  startHeading: "Lancez votre première tâche.",
  startLede:
    "Installez Codewhale, connectez un modèle et dites-lui ce que vous voulez faire. Ajoutez un Fleet pour répartir le travail entre plusieurs agents.",
  startGuideLink: "Lire le guide de démarrage",
  startVocabularyLink: "Voir le vocabulaire du produit",
  chapterAccount: "Obtenir Codewhale",
  availabilityHeading: "Où utiliser Codewhale.",
  availabilityLede:
    "Commencez dans le terminal. L’application et les ordinateurs cloud sont en développement.",
  availability: [
    [
      "Terminal",
      "Publié",
      "Binaires des versions publiées sur GitHub pour Linux, macOS et Windows ; npm et Cargo sont des alternatives. Android sous Termux est disponible en aperçu."
    ],
    [
      "Application web",
      "Aperçu de développement",
      "Accès au compte et association avec le navigateur dans l’aperçu de développement."
    ],
    [
      "Bureau",
      "Build de développement",
      "L’application macOS est en développement ; un téléchargement public sera proposé ultérieurement."
    ],
    [
      "Ordinateurs cloud",
      "En développement",
      "Des ordinateurs hébergés pour exécuter vos tâches."
    ]
  ],
  availabilityNote:
    "Le terminal fonctionne sans compte Codewhale. L’utilisation des modèles hébergés est facturée par votre fournisseur.",
  accountLink: "Créer un compte",
  surfacesHeading: "Utilisez le runtime là où se fait le travail.",
  surfaces: [
    ["TUI", "Travail interactif dans le terminal"],
    ["codewhale exec", "Scripts et CI"],
    ["Client web local","Interface sur localhost ; espace de travail web hébergé en développement"],
    ["Runtime API + MCP", "Intégrations locales"],
    ["Fleet","Plusieurs agents sur une même tâche"],
  ],
  runtimeLink: "Explorer les intégrations",
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
