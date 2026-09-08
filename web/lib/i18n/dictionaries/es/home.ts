import type { HomeDict } from "../types";

/**
 * Spanish home dictionary — native copy for the Tidal Folio landing page,
 * in the current direction: your models, more capable together; agents
 * and control on your own machine; availability stated per surface as it
 * is today. Product vocabulary stays literal (Plan / Work / Operate, Ask /
 * Auto-Review / Full Access, Codewhale, TUI, codewhale exec, Fleet).
 */

export const home: HomeDict = {
  metaTitle: "Codewhale — Crea lo que quieras.",
  metaDescription:
    "Crea software, trabaja con archivos y automatiza tareas con Codewhale. Elige modelos alojados o locales y cambia de proveedor según lo que necesites.",
  kicker: "Agentes de IA de código abierto",
  heroTitleA: "Crea lo que quieras.",
  heroTitleB: "Elige tus modelos.",
  heroIntro:
    "{brand} te ofrece agentes que pueden crear software, trabajar con archivos y automatizar tareas. Usa los modelos que elijas y cambia de proveedor según lo que necesites.",
  getCodewhale: "Obtener Codewhale",
  exploreProduct: "Explorar el producto",
  shotPreview: "Vista previa de la terminal",
  shotBuild: "build de desarrollo v{version}",
  screenshotAlt:
    "Build de desarrollo de Codewhale v0.9.12 en una terminal: la marca de la ballena en braille, una sesión nueva sin historial, el compositor de mensajes y un pie que muestra Full Access, modo Work, dos tareas programadas, servidores MCP conectándose y el modelo GLM-5.3 al máximo esfuerzo",
  latestRelease: "Último lanzamiento {tag}",
  releaseUnavailable: "Estado del lanzamiento no disponible",
  currentSource: "Fuente",
  sourceCandidate: "Sin publicar",
  providerRoutes: "{count} proveedores",
  publishedRelease: "publicado",
  figcaptionSourceCandidate: "sin publicar",
  chapterTerminal: "Tu terminal",
  chapterTerminalTitle: "Empieza con algo que quieras crear.",
  gainHeading:
    "Pon tus ideas a trabajar.",
  gainLede:
    "Crea un proyecto, investiga una pregunta o automatiza una tarea. Empieza con un agente y reparte los trabajos más grandes entre varios.",
  gain: [
    [
      "Crea algo",
      "Convierte una idea en software que funcione. Tus agentes pueden editar archivos, ejecutar comandos y comprobar el resultado."
    ],
    [
      "Automatiza las tareas repetitivas",
      "Crea scripts y flujos de trabajo para las tareas que repites y ejecútalos desde la terminal."
    ],
    [
      "Elige tus modelos",
      "Conecta modelos alojados o locales. Reparte las partes de un trabajo más grande entre agentes con distintos modelos y roles."
    ]
  ],
  chapterModels: "Tus modelos",
  modelsHeading: "Encuentra un modelo adecuado para cada tarea.",
  modelsBody:
    "Usa un proveedor de modelos alojados, conéctate a través de una pasarela o ejecuta un modelo en local. Elige un proveedor y un modelo para cada sesión y cámbialos mientras trabajas.",
  modelsFacts: [
    ["Alojado", "Tu propia clave de API, guardada con codewhale auth set --provider <id>"],
    ["Gateway", "Un endpoint para muchos modelos; el proveedor lo sigues eligiendo tú"],
    ["Local", "vLLM, SGLang, Ollama en localhost; normalmente sin clave"],
  ],
  modelsLink: "Explorar modelos y proveedores",
  startHeading: "Empieza tu primera tarea.",
  startLede:
    "Instala Codewhale, conecta un modelo y dile qué quieres hacer. Añade un Fleet cuando quieras repartir el trabajo entre varios agentes.",
  startGuideLink: "Leer la guía de primeros pasos",
  startVocabularyLink: "Ver el vocabulario del producto",
  chapterAccount: "Obtener Codewhale",
  availabilityHeading: "Dónde usar Codewhale.",
  availabilityLede:
    "Empieza en la terminal. La aplicación y las computadoras en la nube están en desarrollo.",
  availability: [
    [
      "Terminal",
      "Publicada",
      "Binarios de las versiones publicadas en GitHub para Linux, macOS y Windows; npm y Cargo son alternativas. Android en Termux está en vista previa."
    ],
    [
      "Aplicación web",
      "Vista previa de desarrollo",
      "Acceso a la cuenta y vinculación con el navegador en la vista previa de desarrollo."
    ],
    [
      "Escritorio",
      "Build de desarrollo",
      "La aplicación para macOS está en desarrollo; la descarga pública llegará más adelante."
    ],
    [
      "Computadoras en la nube",
      "En desarrollo",
      "Computadoras alojadas para ejecutar tus tareas."
    ]
  ],
  availabilityNote:
    "La terminal funciona sin una cuenta de Codewhale. Tu proveedor factura el uso de los modelos alojados.",
  accountLink: "Crear una cuenta",
  surfacesHeading: "Usa el runtime donde ocurre el trabajo.",
  surfaces: [
    ["TUI", "Trabajo interactivo en la terminal"],
    ["codewhale exec", "Scripts y CI"],
    ["Cliente web local","Interfaz en localhost; espacio de trabajo web alojado en desarrollo"],
    ["Runtime API + MCP", "Integraciones locales"],
    ["Fleet","Varios agentes en un mismo trabajo"],
  ],
  runtimeLink: "Explorar integraciones",
  installBandHeading: "Empieza con un solo comando.",
  copy: "Copiar",
  copied: "Copiado ✓",
  binaries: "Binarios",
  chinaMirrors: "Espejos en China",
  installGuideLink: "Leer la guía de instalación",
  communityHeading: "Construido en público",
  communityBody:
    "Con licencia MIT y moldeado por colaboradores en runtimes, proveedores, plataformas, documentación y pruebas.",
  communityLinksAria: "Enlaces de la comunidad",
  contribute: "Contribuir",
};
