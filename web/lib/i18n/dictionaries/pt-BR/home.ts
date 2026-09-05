import type { HomeDict } from "../types";

/**
 * Brazilian Portuguese home dictionary — native copy for the Tidal Folio landing page,
 * in the current direction: your models, more capable together; agents
 * and control on your own machine; availability stated per surface as it
 * is today. Product vocabulary stays literal (Plan / Work / Operate, Ask /
 * Auto-Review / Full Access, Codewhale, TUI, codewhale exec, Fleet).
 */

export const home: HomeDict = {
  metaTitle: "Codewhale — Seus modelos. Mais capazes juntos.",
  metaDescription:
    "Codewhale é um sistema de computação agêntica de código aberto. Traga os modelos que você já usa — hospedados, por gateway ou locais — e coloque-os para trabalhar juntos no seu terminal, na sua máquina, sob o seu controle. Rust, MIT.",
  kicker: "Computação agêntica, nos seus termos",
  heroTitleA: "Seus modelos.",
  heroTitleB: "Mais capazes juntos.",
  heroIntro:
    "{brand} reúne os modelos que você já usa em um único terminal e os faz trabalhar como uma tripulação — lendo seu código, editando, rodando as verificações — enquanto você decide o que cada um pode fazer. Código aberto, na sua máquina.",
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
  chapterTerminalTitle: "Um lugar familiar para começar.",
  gainHeading:
    "O que você recebe não é um chatbot. É alavancagem sobre os modelos que você já paga.",
  gainLede:
    "Uma sessão pode ter vários modelos ao mesmo tempo, cada um no papel que você deu, todos trabalhando no mesmo repositório sob as mesmas regras.",
  gain: [
    ["Seus modelos", "Chaves hospedadas, um gateway ou um runtime local sem chave nenhuma. Fixe um modelo diferente em cada papel e mantenha o provedor que você escolheu — um nome de modelo nunca o troca por você."],
    ["Agentes capazes", "Modos Plan, Work e Operate; um fleet de subagentes para um mesmo trabalho; ferramentas para arquivos, shell, web e MCP; sessões que salvam, retomam e voltam atrás."],
    ["Controle na sua máquina", "Ask, Auto-Review ou Full Access — você define quanto ele faz antes de perguntar. Roda localmente, em sandbox onde o sistema permite, com um registro de auditoria que você pode ler."],
  ],
  chapterModels: "Seus modelos",
  modelsHeading: "Traga o que você tem. Não mude nada que você não escolheu.",
  modelsBody:
    "Conecte um provedor hospedado compatível, um gateway ou um servidor de modelos local. Confira o provedor e o modelo antes de começar. Um servidor local pode exigir autenticação.",
  modelsFacts: [
    ["Hospedado", "Sua própria chave de API, salva com codewhale auth set"],
    ["Gateway", "Um endpoint para muitos modelos, o provedor continua sendo escolha sua"],
    ["Local", "vLLM, SGLang, Ollama em localhost — normalmente sem chave"],
  ],
  modelsLink: "Ver todos os provedores",
  startHeading: "Quatro passos até a primeira sessão.",
  startLede:
    "Instale, abra uma sessão sem chave, conecte um provedor e depois monte um fleet quando um modelo não bastar.",
  startGuideLink: "Ler o guia de primeiros passos",
  startVocabularyLink: "Ver o vocabulário do produto",
  chapterAccount: "Onde roda hoje",
  availabilityHeading: "Disponível agora, em desenvolvimento e ainda não — dito com clareza.",
  availabilityLede:
    "O terminal é o produto lançado. Todo o resto está listado no estado em que realmente se encontra.",
  availability: [
    ["Terminal", "Lançado", "Binários do GitHub Releases para Linux, macOS e Windows; npm e Cargo são alternativas. Android no Termux é uma prévia."],
    ["Aplicativo web", "Login e controle remoto disponíveis", "Entre ou crie uma conta e digite /rc em uma sessão local em andamento para continuar exatamente essa sessão pelo navegador. O restante da bancada no navegador ainda é uma prévia de desenvolvimento."],
    ["Desktop", "Build de desenvolvimento", "Existem builds alfa para macOS, Linux e Windows. Ainda não há um aplicativo desktop lançado."],
    ["Computadores na nuvem", "Ainda não disponível", "Executar trabalho em um computador hospedado está em desenvolvimento. Esta página dirá quando funcionar."],
  ],
  availabilityNote:
    "O terminal não precisa de conta. Uma conta nunca é, por si só, um plano pago, e nada neste site pode cobrar você.",
  accountLink: "Criar uma conta",
  surfacesHeading: "Use o runtime onde o trabalho acontece.",
  surfaces: [
    ["TUI", "Trabalho interativo no terminal"],
    ["codewhale exec", "Scripts e CI"],
    ["Cliente web", "Cliente de navegador, somente loopback"],
    ["Runtime API + MCP", "Integrações locais"],
    ["fleet", "Trabalho multiagente duradouro"],
  ],
  runtimeLink: "Ver interfaces de runtime e notas de estabilidade",
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
