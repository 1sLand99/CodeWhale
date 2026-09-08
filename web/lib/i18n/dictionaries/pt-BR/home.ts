import type { HomeDict } from "../types";

/**
 * Brazilian Portuguese home dictionary — native copy for the Tidal Folio landing page,
 * in the current direction: your models, more capable together; agents
 * and control on your own machine; availability stated per surface as it
 * is today. Product vocabulary stays literal (Plan / Work / Operate, Ask /
 * Auto-Review / Full Access, Codewhale, TUI, codewhale exec, Fleet).
 */

export const home: HomeDict = {
  metaTitle: "Codewhale — Crie o que quiser.",
  metaDescription:
    "Crie software, trabalhe com arquivos e automatize tarefas com o Codewhale. Escolha modelos hospedados ou locais e troque de provedor conforme suas necessidades mudam.",
  kicker: "Agentes de IA de código aberto",
  heroTitleA: "Crie o que quiser.",
  heroTitleB: "Escolha seus modelos.",
  heroIntro:
    "{brand} oferece agentes que podem criar software, trabalhar com arquivos e automatizar tarefas. Use os modelos que você escolher e troque de provedor conforme suas necessidades mudam.",
  getCodewhale: "Obter o Codewhale",
  exploreProduct: "Explorar o produto",
  shotPreview: "Prévia do terminal",
  shotBuild: "build de desenvolvimento v{version}",
  screenshotAlt:
    "Build de desenvolvimento do Codewhale v0.9.12 em um terminal: a marca da baleia em braille, uma sessão nova sem histórico, o compositor de mensagens e um rodapé mostrando Full Access, modo Work, duas tarefas agendadas, servidores MCP conectando e o modelo GLM-5.3 no esforço máximo",
  latestRelease: "Último lançamento {tag}",
  releaseUnavailable: "Status do lançamento indisponível",
  currentSource: "Código-fonte",
  sourceCandidate: "Não publicado",
  providerRoutes: "{count} provedores",
  publishedRelease: "publicado",
  figcaptionSourceCandidate: "não publicado",
  chapterTerminal: "Seu terminal",
  chapterTerminalTitle: "Comece com algo que você queira criar.",
  gainHeading:
    "Coloque suas ideias em prática.",
  gainLede:
    "Crie um projeto, investigue uma questão ou automatize uma tarefa. Comece com um agente e divida trabalhos maiores entre vários.",
  gain: [
    [
      "Crie algo",
      "Transforme uma ideia em software que funciona. Seus agentes podem editar arquivos, executar comandos e conferir o resultado."
    ],
    [
      "Automatize as tarefas repetitivas",
      "Crie scripts e fluxos de trabalho para tarefas que você repete e execute-os pelo terminal."
    ],
    [
      "Escolha seus modelos",
      "Conecte modelos hospedados ou locais. Distribua partes de um trabalho maior entre agentes com modelos e papéis diferentes."
    ]
  ],
  chapterModels: "Seus modelos",
  modelsHeading: "Encontre um modelo adequado para a tarefa.",
  modelsBody:
    "Use um provedor de modelos hospedados, conecte-se por um gateway ou execute um modelo localmente. Escolha um provedor e um modelo para cada sessão e troque-os enquanto trabalha.",
  modelsFacts: [
    ["Hospedado", "Sua própria chave de API, salva com codewhale auth set --provider <id>"],
    ["Gateway", "Um endpoint para muitos modelos, o provedor continua sendo escolha sua"],
    ["Local", "vLLM, SGLang, Ollama em localhost — normalmente sem chave"],
  ],
  modelsLink: "Explorar modelos e provedores",
  startHeading: "Comece sua primeira tarefa.",
  startLede:
    "Instale o Codewhale, conecte um modelo e diga o que quer fazer. Adicione um Fleet quando quiser dividir o trabalho entre vários agentes.",
  startGuideLink: "Ler o guia de primeiros passos",
  startVocabularyLink: "Ver o vocabulário do produto",
  chapterAccount: "Obter o Codewhale",
  availabilityHeading: "Onde usar o Codewhale.",
  availabilityLede:
    "Comece pelo terminal. O aplicativo e os computadores na nuvem estão em desenvolvimento.",
  availability: [
    [
      "Terminal",
      "Lançado",
      "Binários das versões publicadas no GitHub para Linux, macOS e Windows; npm e Cargo são alternativas. Android no Termux é uma prévia."
    ],
    [
      "Aplicativo web",
      "Prévia de desenvolvimento",
      "Acesso à conta e pareamento com o navegador na prévia de desenvolvimento."
    ],
    [
      "Desktop",
      "Build de desenvolvimento",
      "O aplicativo para macOS está em desenvolvimento; o download público virá mais adiante."
    ],
    [
      "Computadores na nuvem",
      "Em desenvolvimento",
      "Computadores hospedados para executar suas tarefas."
    ]
  ],
  availabilityNote:
    "O terminal funciona sem uma conta do Codewhale. O uso de modelos hospedados é cobrado pelo seu provedor.",
  accountLink: "Criar uma conta",
  surfacesHeading: "Use o runtime onde o trabalho acontece.",
  surfaces: [
    ["TUI", "Trabalho interativo no terminal"],
    ["codewhale exec", "Scripts e CI"],
    ["Cliente web local","Interface em localhost; ambiente de trabalho web hospedado em desenvolvimento"],
    ["Runtime API + MCP", "Integrações locais"],
    ["Fleet","Vários agentes no mesmo trabalho"],
  ],
  runtimeLink: "Explorar integrações",
  installBandHeading: "Comece com um comando.",
  copy: "Copiar",
  copied: "Copiado ✓",
  binaries: "Binários",
  chinaMirrors: "Espelhos da China",
  installGuideLink: "Ler o guia de instalação",
  communityHeading: "Construído em público",
  communityBody:
    "Licenciado sob MIT e moldado por contribuidores em runtimes, provedores, plataformas, documentação e testes.",
  communityLinksAria: "Links da comunidade",
  contribute: "Contribuir",
};
