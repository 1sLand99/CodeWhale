import type { HomeDict } from "../types";

/**
 * Korean home dictionary — native copy for the Tidal Folio landing page,
 * in the current direction: your models, more capable together; agents
 * and control on your own machine; availability stated per surface as it
 * is today. Product vocabulary stays literal (Plan / Work / Operate, Ask /
 * Auto-Review / Full Access, Codewhale, TUI, codewhale exec, Fleet).
 */

export const home: HomeDict = {
  metaTitle: "Codewhale — 원하는 것을 만드세요.",
  metaDescription:
    "Codewhale로 소프트웨어를 만들고, 파일을 다루고, 작업을 자동화하세요. 호스팅 모델이나 로컬 모델을 선택하고, 작업에 맞춰 제공업체를 바꿀 수 있습니다.",
  kicker: "오픈소스 AI 에이전트",
  heroTitleA: "원하는 것을 만드세요.",
  heroTitleB: "모델을 직접 선택하세요.",
  heroIntro:
    "{brand}의 에이전트로 소프트웨어를 만들고, 파일을 다루고, 작업을 자동화할 수 있습니다. 원하는 모델을 사용하고, 작업에 맞춰 제공업체를 바꿔 보세요.",
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
  chapterTerminalTitle: "만들고 싶은 것부터 시작하세요.",
  gainHeading: "아이디어를 실행에 옮기세요.",
  gainLede: "프로젝트를 만들거나, 궁금한 것을 조사하거나, 작업을 자동화하세요. 에이전트 하나로 시작하고, 큰 작업은 여러 에이전트가 나눠 맡게 할 수 있습니다.",
  gain: [
    [
      "직접 만들어 보세요",
      "아이디어를 작동하는 소프트웨어로 만드세요. 에이전트가 파일을 편집하고, 명령을 실행하고, 결과를 확인할 수 있습니다."
    ],
    [
      "반복 작업을 자동화하세요",
      "반복하는 작업에 쓸 스크립트와 워크플로를 만들고, 터미널에서 실행하세요."
    ],
    [
      "모델을 직접 선택하세요",
      "호스팅 모델이나 로컬 모델을 연결하세요. 큰 작업을 나눠 서로 다른 모델과 역할을 가진 에이전트에 맡길 수 있습니다."
    ]
  ],
  chapterModels: "당신의 모델",
  modelsHeading: "작업에 맞는 모델을 찾으세요.",
  modelsBody:
    "호스팅 모델 제공업체를 이용하거나, 게이트웨이로 연결하거나, 모델을 로컬에서 실행하세요. 세션마다 제공업체와 모델을 선택하고 작업 중에도 바꿀 수 있습니다.",
  modelsFacts: [
    ["호스팅", "codewhale auth set --provider <id>으로 저장한 내 API 키"],
    ["게이트웨이", "하나의 엔드포인트로 여러 모델, 제공자는 여전히 내가 선택"],
    ["로컬", "localhost의 vLLM, SGLang, Ollama — 보통 키 불필요"],
  ],
  modelsLink: "모델과 제공업체 살펴보기",
  startHeading: "첫 작업을 시작하세요.",
  startLede: "Codewhale을 설치하고 모델을 연결한 뒤, 하고 싶은 일을 알려 주세요. 여러 에이전트가 작업을 나눠 맡게 하려면 Fleet을 추가하세요.",
  startGuideLink: "시작 가이드 읽기",
  startVocabularyLink: "제품 용어 보기",
  chapterAccount: "Codewhale 받기",
  availabilityHeading: "Codewhale을 사용할 수 있는 곳.",
  availabilityLede: "터미널에서 시작하세요. 앱과 클라우드 컴퓨터는 개발 중입니다.",
  availability: [
    [
      "터미널",
      "출시됨",
      "Linux, macOS, Windows용 릴리스 바이너리를 GitHub에서 제공합니다. npm과 Cargo로도 설치할 수 있습니다. Android에서 Termux로 실행하는 버전은 미리보기입니다."
    ],
    [
      "웹 앱",
      "개발 미리보기",
      "개발 미리보기에서 계정 접속과 브라우저 페어링을 이용할 수 있습니다."
    ],
    [
      "데스크톱",
      "개발 빌드",
      "macOS 앱은 개발 중이며, 공개 다운로드는 추후 제공될 예정입니다."
    ],
    [
      "클라우드 컴퓨터",
      "개발 중",
      "작업을 실행할 수 있는 호스팅 컴퓨터."
    ]
  ],
  availabilityNote: "터미널은 Codewhale 계정 없이 사용할 수 있습니다. 호스팅 모델 사용 요금은 이용하는 제공업체에서 청구합니다.",
  accountLink: "계정 만들기",
  surfacesHeading: "작업이 일어나는 자리에서 런타임을 사용하세요.",
  surfaces: [
    ["TUI", "대화형 터미널 작업"],
    ["codewhale exec", "스크립트와 CI"],
    ["로컬 웹 클라이언트","localhost 인터페이스. 호스팅형 브라우저 작업 공간은 개발 중"],
    ["Runtime API + MCP", "로컬 통합"],
    ["Fleet","여러 에이전트가 하나의 작업을 함께 수행"],
  ],
  runtimeLink: "연동 기능 살펴보기",
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
