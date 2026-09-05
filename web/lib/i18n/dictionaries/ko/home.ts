import type { HomeDict } from "../types";

/**
 * Korean home dictionary — native copy for the Tidal Folio landing page,
 * in the current direction: your models, more capable together; agents
 * and control on your own machine; availability stated per surface as it
 * is today. Product vocabulary stays literal (Plan / Work / Operate, Ask /
 * Auto-Review / Full Access, Codewhale, TUI, codewhale exec, Fleet).
 */

export const home: HomeDict = {
  metaTitle: "Codewhale — 당신의 모델, 함께라서 더 유능하게.",
  metaDescription:
    "Codewhale은 오픈소스 에이전트 컴퓨팅 시스템입니다. 이미 쓰고 있는 모델(호스팅, 게이트웨이, 로컬)을 터미널로 가져와 당신의 머신에서, 당신의 통제 아래 함께 일하게 하세요. Rust, MIT.",
  kicker: "에이전트 컴퓨팅, 당신의 조건대로",
  heroTitleA: "당신의 모델,",
  heroTitleB: "함께라서 더 유능하게.",
  heroIntro:
    "{brand}은 이미 쓰고 있는 모델들을 하나의 터미널로 모아 한 팀처럼 일하게 합니다. 코드를 읽고, 고치고, 검사를 돌리는 동안 각 모델이 무엇을 해도 되는지는 당신이 정합니다. 오픈소스이며 당신의 머신에서 실행됩니다.",
  getCodewhale: "Codewhale 받기",
  exploreProduct: "제품 살펴보기",
  shotPreview: "터미널 미리보기",
  shotBuild: "v{version} 개발 빌드",
  screenshotAlt:
    "터미널의 Codewhale v0.9.12 개발 빌드: 점자 고래 마크, 아직 기록이 없는 새 세션, 메시지 입력창, 그리고 Full Access, Work 모드, 예약 작업 2개, MCP 서버 연결 중, GLM-5.3 최대 강도를 보여주는 푸터",
  latestRelease: "최신 릴리스 {tag}",
  releaseUnavailable: "릴리스 상태를 확인할 수 없음",
  currentSource: "소스",
  sourceCandidate: "미공개",
  providerRoutes: "프로바이더 {count}개",
  publishedRelease: "공개됨",
  figcaptionSourceCandidate: "미공개",
  chapterTerminal: "당신의 터미널",
  chapterTerminalTitle: "익숙한 곳에서 시작합니다.",
  gainHeading: "챗봇이 아니라, 이미 비용을 내고 있는 모델에 대한 지렛대를 얻습니다.",
  gainLede: "한 세션에 여러 모델을 동시에 두고, 각각에 역할을 맡기고, 같은 저장소에서 같은 규칙으로 일하게 할 수 있습니다.",
  gain: [
    ["당신의 모델", "호스팅 키, 게이트웨이, 또는 키가 전혀 필요 없는 로컬 런타임. 역할마다 다른 모델을 고정하고, 선택한 제공자는 그대로 유지됩니다. 모델 이름이 제공자를 대신 바꾸지 않습니다."],
    ["유능한 에이전트", "Plan, Work, Operate 모드. 한 작업을 나눠 맡는 서브에이전트 fleet. 파일, 셸, 웹, MCP 도구. 저장하고 재개하고 되돌릴 수 있는 세션."],
    ["내 머신에서의 통제", "Ask, Auto-Review, Full Access. 묻기 전에 얼마나 할지는 당신이 정합니다. 로컬에서 실행되고, OS가 허용하는 곳에서는 샌드박스가 적용되며, 감사 로그를 직접 읽을 수 있습니다."],
  ],
  chapterModels: "당신의 모델",
  modelsHeading: "가진 것을 가져오세요. 고르지 않은 것은 바뀌지 않습니다.",
  modelsBody:
    "Codewhale에는 {count}개의 제공자가 내장되어 있고 모두 동등하게 다뤄집니다. 키를 한 번 저장하고 모델을 지정하면 경로는 설정한 그대로 유지됩니다. vLLM, SGLang, Ollama의 로컬 모델은 키가 필요 없습니다.",
  modelsFacts: [
    ["호스팅", "codewhale auth set으로 저장한 내 API 키"],
    ["게이트웨이", "하나의 엔드포인트로 여러 모델, 제공자는 여전히 내가 선택"],
    ["로컬", "localhost의 vLLM, SGLang, Ollama — 보통 키 불필요"],
  ],
  modelsLink: "모든 제공자 보기",
  startHeading: "첫 세션까지 네 단계.",
  startLede: "설치하고, 키 없이 세션을 열고, 제공자를 연결한 뒤, 모델 하나로 부족할 때 fleet을 설정하세요.",
  startGuideLink: "시작 가이드 읽기",
  startVocabularyLink: "제품 용어 보기",
  chapterAccount: "지금 실행되는 곳",
  availabilityHeading: "지금 가능한 것, 개발 중인 것, 아직 아닌 것 — 있는 그대로.",
  availabilityLede: "터미널이 출시된 제품입니다. 나머지는 실제 상태 그대로 적었습니다.",
  availability: [
    ["터미널", "출시됨", "npm, Cargo, 그리고 Linux·macOS·Windows용 빌드된 바이너리. Android의 Termux는 미리보기입니다."],
    ["웹 앱", "로그인과 원격 제어 가능", "로그인하거나 계정을 만든 뒤, 실행 중인 로컬 세션에서 /rc를 입력하면 바로 그 세션을 브라우저에서 이어갈 수 있습니다. 브라우저 워크벤치의 나머지는 개발 미리보기입니다."],
    ["데스크톱", "개발 빌드", "macOS, Linux, Windows용 알파 빌드가 있습니다. 출시된 데스크톱 앱은 아직 없습니다."],
    ["클라우드 컴퓨터", "아직 이용 불가", "호스팅된 컴퓨터에서 작업을 실행하는 기능은 개발 중입니다. 작동하게 되면 이 페이지에서 알리겠습니다."],
  ],
  availabilityNote: "터미널에는 계정이 필요 없습니다. 계정 자체가 유료 플랜이 되는 일은 없으며, 이 사이트는 요금을 청구할 수 없습니다.",
  accountLink: "계정 만들기",
  surfacesHeading: "작업이 일어나는 자리에서 런타임을 사용하세요.",
  surfaces: [
    ["TUI", "대화형 터미널 작업"],
    ["codewhale exec", "스크립트와 CI"],
    ["웹 클라이언트", "루프백 전용 브라우저 클라이언트"],
    ["Runtime API + MCP", "로컬 통합"],
    ["fleet", "지속형 멀티 에이전트 작업"],
  ],
  runtimeLink: "런타임 인터페이스와 안정성 노트 보기",
  installBandHeading: "명령 하나로 시작하세요.",
  copy: "복사",
  copied: "복사됨 ✓",
  binaries: "바이너리",
  chinaMirrors: "중국 미러",
  installGuideLink: "설치 가이드 읽기",
  communityHeading: "공개적으로 개발합니다",
  communityBody: "MIT 라이선스로 공개되어 있으며, 런타임과 프로바이더, 플랫폼, 문서, 테스트 전반의 기여자들이 함께 만들어 갑니다.",
  communityLinksAria: "커뮤니티 링크",
  contribute: "기여하기",
};
