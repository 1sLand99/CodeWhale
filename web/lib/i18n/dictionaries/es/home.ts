import type { HomeDict } from "../types";

/**
 * Spanish home dictionary — native copy for the Tidal Folio landing page,
 * in the current direction: your models, more capable together; agents
 * and control on your own machine; availability stated per surface as it
 * is today. Product vocabulary stays literal (Plan / Work / Operate, Ask /
 * Auto-Review / Full Access, Codewhale, TUI, codewhale exec, Fleet).
 */

export const home: HomeDict = {
  metaTitle: "Codewhale — Tus modelos. Más capaces juntos.",
  metaDescription:
    "Codewhale es un sistema de computación agéntica de código abierto. Trae los modelos que ya usas —alojados, por gateway o locales— y ponlos a trabajar juntos en tu terminal, en tu máquina, bajo tu control. Rust, MIT.",
  kicker: "Computación agéntica, en tus términos",
  heroTitleA: "Tus modelos.",
  heroTitleB: "Más capaces juntos.",
  heroIntro:
    "{brand} reúne los modelos que ya usas en una sola terminal y los hace trabajar como una tripulación —leyendo tu código, editando, ejecutando las comprobaciones— mientras tú decides qué puede hacer cada uno. Código abierto, en tu máquina.",
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
  chapterTerminalTitle: "Un lugar familiar para empezar.",
  gainHeading:
    "Lo que obtienes no es un chatbot. Es palanca sobre los modelos que ya pagas.",
  gainLede:
    "Una sesión puede tener varios modelos a la vez, cada uno en el rol que le diste, todos trabajando en el mismo repositorio bajo las mismas reglas.",
  gain: [
    ["Tus modelos", "Claves alojadas, un gateway o un runtime local sin clave alguna. Fija un modelo distinto a cada rol y conserva el proveedor que elegiste: un nombre de modelo nunca lo cambia por ti."],
    ["Agentes capaces", "Modos Plan, Work y Operate; un fleet de subagentes para un mismo trabajo; herramientas para archivos, shell, web y MCP; sesiones que se guardan, se reanudan y se revierten."],
    ["Control en tu máquina", "Ask, Auto-Review o Full Access: tú fijas cuánto hace antes de preguntar. Corre en local, en sandbox donde el sistema lo permite, con un registro de auditoría que puedes leer."],
  ],
  chapterModels: "Tus modelos",
  modelsHeading: "Trae lo que tienes. No cambies nada que no hayas elegido.",
  modelsBody:
    "Codewhale incluye {count} proveedores y los trata como iguales. Guarda una clave una vez, nombra un modelo y la ruta se queda exactamente como la fijaste. Los modelos locales sobre vLLM, SGLang u Ollama no necesitan clave.",
  modelsFacts: [
    ["Alojado", "Tu propia clave de API, guardada con codewhale auth set"],
    ["Gateway", "Un endpoint para muchos modelos; el proveedor lo sigues eligiendo tú"],
    ["Local", "vLLM, SGLang, Ollama en localhost; normalmente sin clave"],
  ],
  modelsLink: "Ver todos los proveedores",
  startHeading: "Cuatro pasos hasta la primera sesión.",
  startLede:
    "Instala, abre una sesión sin clave, conecta un proveedor y luego configura un fleet cuando un modelo no baste.",
  startGuideLink: "Leer la guía de primeros pasos",
  startVocabularyLink: "Ver el vocabulario del producto",
  chapterAccount: "Dónde funciona hoy",
  availabilityHeading: "Disponible ahora, en desarrollo y todavía no: dicho sin rodeos.",
  availabilityLede:
    "La terminal es el producto publicado. Todo lo demás aparece con el estado en que realmente está.",
  availability: [
    ["Terminal", "Publicada", "Binarios de GitHub Releases para Linux, macOS y Windows; npm y Cargo son alternativas. Android en Termux está en vista previa."],
    ["Aplicación web", "Inicio de sesión y control remoto disponibles", "Inicia sesión o crea una cuenta y escribe /rc en una sesión local en marcha para continuar esa misma sesión desde el navegador. El resto del banco de trabajo en el navegador sigue siendo una vista previa de desarrollo."],
    ["Escritorio", "Build de desarrollo", "Existen builds alfa para macOS, Linux y Windows. Todavía no hay una aplicación de escritorio publicada."],
    ["Computadoras en la nube", "Todavía no disponible", "Ejecutar trabajo en una computadora alojada está en desarrollo. Esta página lo dirá cuando funcione."],
  ],
  availabilityNote:
    "La terminal no necesita cuenta. Una cuenta nunca es por sí misma un plan de pago, y nada en este sitio puede cobrarte.",
  accountLink: "Crear una cuenta",
  surfacesHeading: "Usa el runtime donde ocurre el trabajo.",
  surfaces: [
    ["TUI", "Trabajo interactivo en la terminal"],
    ["codewhale exec", "Scripts y CI"],
    ["Cliente web", "Cliente de navegador, solo loopback"],
    ["Runtime API + MCP", "Integraciones locales"],
    ["fleet", "Trabajo multiagente duradero"],
  ],
  runtimeLink: "Ver las interfaces de runtime y las notas de estabilidad",
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
