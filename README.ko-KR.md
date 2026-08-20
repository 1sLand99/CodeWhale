<!-- source: README.md sha256:1f5bf984e975 -->
# Codewhale

터미널에서 쓰는 오픈소스 코딩 에이전트 — 모델은 당신이 가져옵니다.

Codewhale은 DeepSeek을 위한 네이티브 경험으로 시작했습니다. 이후 커뮤니티가 이끄는 프로젝트로 성장했습니다. 점점 커지는 국제 커뮤니티에 맞고, 가능한 한 많은 모델과 프로바이더를 지원하는 하나의 코딩 하네스입니다 — 오픈 모델을 가장 먼저, 호스팅이든 로컬이든, 어느 하나를 특별 대우하지 않습니다.

프로바이더, 모델, 작업을 지정하면 코드를 읽고, 파일을 편집하고, 명령을 실행하고, 스스로 작업을 확인하며, 작업이 끝나거나 사용자의 판단이 필요해지면 멈춥니다. 작업 도중에도 `/model`로 모델을 바꿀 수 있습니다. 대화형 작업에는 TUI를, 스크립트와 CI에는 `codewhale exec`를 사용합니다. Rust로 작성했고, MIT 라이선스이며, 당신의 컴퓨터에서 실행됩니다.

다른 하네스와 다른 점은 이것입니다. **역할마다 어떤 모델을 쓸지 당신이 고르고, 서로 같을 필요가 없습니다** — 그리고 **Codewhale의 에이전트들은 모델을 가로질러 서로 대화합니다.** Fleet은 역할별로 프로바이더, 모델, 추론 등급을 각각 고정합니다. 그래서 빠르고 저렴한 모델이 값비싼 추론 모델을 지휘할 수도 있고, GLM builder와 Kimi reviewer가 같은 작업을 함께 처리할 수도 있습니다. 실행 중에는 그중 누구에게든 중간에 메시지를 보내고, transcript를 들여다보고, 중단할 수 있습니다 — 그리고 부모-자식 관계뿐이 아닙니다. 같은 워크스페이스의 서로 다른 Codewhale 태스크끼리는 재시작을 견디는 Agent Mail을 주고받고, 안전한 경계에서 정확히 한 번 전달되며, 자격 증명은 가려집니다. `/goal`은 장기 목표를 턴을 넘어 지키며 정말 끝날 때까지 놓지 않습니다. 역할은 파일이고, 하네스 전체는 당신 것으로 남습니다.

우리는 항상 기여자와 개선할 방법을 찾고 있습니다. 사용하는 모델이나 프로바이더가 빠져 있거나 무언가가 깨진다면, 그것을 알려 주는 일이 할 수 있는 가장 유용한 일 중 하나입니다 — [기여](#기여)를 참고하세요.

[English](README.md) · [简体中文](README.zh-CN.md) · [日本語](README.ja-JP.md) · [Tiếng Việt](README.vi.md) · [Bahasa Indonesia](README.id.md) · [Español](README.es-419.md) · [Português](README.pt-BR.md) · [Русский](README.ru.md) · [Українська](README.uk.md) · [Français](README.fr.md) · [Deutsch](README.de.md) · [繁體中文](README.zh-TW.md) · [हिन्दी](README.hi.md) · [Türkçe](README.tr.md) · [Italiano](README.it.md) · [Polski](README.pl.md) · [العربية](README.ar.md) · [Català](README.ca.md) · [codewhale.net](https://codewhale.net/) · [Docs](docs) · [Changelog](CHANGELOG.md) · [Discord](https://discord.gg/37gfS3ksug)

[![CI](https://github.com/Hmbown/CodeWhale/actions/workflows/ci.yml/badge.svg)](https://github.com/Hmbown/CodeWhale/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/codewhale-cli?label=crates.io)](https://crates.io/crates/codewhale-cli)
[![npm](https://img.shields.io/npm/v/codewhale?label=npm)](https://www.npmjs.com/package/codewhale)
[![Discord](https://img.shields.io/badge/Discord-join%20the%20community-5865F2?logo=discord&logoColor=white)](https://discord.gg/37gfS3ksug)

![터미널에서 실행 중인 Codewhale](assets/screenshot.webp)

## 설치

```bash
npm install -g codewhale
```

Cargo, Docker, Nix, Scoop, 사전 빌드 아카이브, Android/Termux, 그리고 GitHub에 접근할 수 없는 사용자를 위한 CNB 미러는 [docs/INSTALL.md](docs/INSTALL.md)에서 다룹니다. `deepseek-tui`에서 넘어오나요? 설정과 세션은 그대로 이어집니다 — [docs/REBRAND.md](docs/REBRAND.md)를 참고하세요.

## 사용

```bash
codewhale auth set --provider deepseek   # or export ANTHROPIC_API_KEY, etc.
codewhale                                # open the TUI
codewhale exec "fix the failing test"    # headless
codewhale web                            # local browser client on 127.0.0.1
```


TUI 안에서: `/model`은 프로바이더와 모델을 함께 전환하고, `/fleet`은 팀을 구성하고 실행하며(한 번에 한 역할씩, 각자 자기 모델을 가집니다), `/undo`는 직전 턴을 되돌리고, `/restore <N>`은 워크스페이스를 이전 스냅샷으로 되돌립니다(인자 없는 `/restore`는 스냅샷 목록만 보여줍니다). 입력창이 비어 있을 때 `Tab`은 Plan / Work / Operate 모드를 순환하고, 입력창에 내용이 있으면 `Tab`은 슬래시 명령과 `@` 멘션을 자동 완성합니다. `Shift+Tab`은 언제든지 Ask / Auto-Review / Full Access 권한 태세를 순환합니다. `!`는 일반 승인 경로를 거쳐 셸 명령을 실행합니다.

## 기능

- **언제든 이어서 하는 작업.** Fleet은 모든 단계를 추가 전용 장부에 기록하고, `fleet resume`으로 멈춘 곳에서 이어갑니다. `/goal`은 턴을 넘어 계속 추구하는 목표를 붙들어 — 일시정지도, 재개도 가능하고 세션과 함께 재시작 후에도 복원됩니다 — `/workflows`는 이 워크스페이스의 저널이 보관하는 모든 실행을 보여주는 실시간 대시보드입니다.
- **에이전트끼리, 모델을 넘어 대화합니다.** Codewhale의 모든 에이전트는 일하는 동안 늘 닿을 수 있습니다: `message`는 실행 중인 서브에이전트에게 메모를 예약하고, `followup`은 다음 안전한 경계에서 그 메모와 함께 깨우며, `peek`은 transcript를 읽고, 중단은 해당 턴만 멈춥니다. 부모-자식 트리에 머무르지 않습니다. 같은 워크스페이스의 서로 다른 태스크는 지속적인 **Agent Mail**을 주고받습니다 — 재시작을 살아남고, 수신 쪽의 안전한 경계에서 정확히 한 번 배달되며, 자격 증명과 경로를 가리는, 대기열에 들어간 인계 요약입니다. 이렇게 GLM 세션과 Kimi 세션이 두 터미널에서 스스로 조율하고, 당신은 중계역에서 벗어납니다. 양쪽 모두 서로 다른 모델이어도 되고, 그 대화를 Codewhale이 실어 나릅니다.
- **당신이 직접 쓰는 하네스.** 역할은 읽고 수정할 수 있는 파일입니다. 역할마다 모델, 도구 태세, 상시 지시를 담아 팀과 공유하려면 프로젝트에, 저장소를 옮겨 다니며 쓰려면 개인 설정 옆에 둡니다. constitution은 모든 세션에서 에이전트가 어떻게 행동하기를 바라는지 기록해, 하네스가 우리 방식이 아니라 당신의 방식에 맞도록 합니다.
- **허용하기 전까지는 읽기 전용.** Plan 모드는 파일을 바꾸지 않고, 위험한 명령은 승인을 거칩니다. OS 샌드박스가 실제로 명령을 래핑할 때 Codewhale은 이를 그대로 표시합니다. macOS에서는 사용 가능한 Seatbelt, Linux에서는 옵트인 bubblewrap입니다. 저장소의 `constitution.json`은 Full Access조차 건너뛸 수 없는 쓰기 홀드로 컴파일됩니다.
- **이어서 할 수 있는 작업.** Fleet은 모든 단계를 추가 전용 원장에 기록하므로, `fleet resume`으로 멈춘 지점부터 이어갈 수 있습니다.

## 통합

- **DeepSeek Harness(dsh) — Codewhale로 연결.**
  `codewhale integrations dsh connect`는 기존 `@deepseek-ai/dsh` 설치를
  Codewhale의 제공자 라우트·권한·작업 공간에 연결하고,
  `integrations dsh install-bundle`은 옵트인 DSH 플러그인 번들을 추가해
  `dsh --profile codewhale`이 해당 정체성을 단독으로 유지하게 합니다.
  권한과 수명 주기는 Codewhale이 담당하며, dsh 고유의 세션·프로필·자격
  증명은 그대로 유지됩니다.
  [docs/INTEGRATIONS_DSH.md](docs/INTEGRATIONS_DSH.md) 참조.
- **VS Code.** 공식 확장 스캐폴드(`extensions/vscode`)는 통합 터미널에서
  Codewhale을 열고 로컬 런타임 기반의 읽기 전용 Agent View를 제공합니다.
  현재는 로컬 개발 프리뷰이며 마켓플레이스 릴리스가 아닙니다.

## 더 알아보기

- [docs/PROVIDERS.md](docs/PROVIDERS.md) — 호스팅·게이트웨이·로컬까지 모든
  프로바이더 라우트
- [docs/FLEET.md](docs/FLEET.md) — Fleet, 원장, 재개
- [docs/WORKFLOW_EXPERIMENTAL_SEARCH.md](docs/WORKFLOW_EXPERIMENTAL_SEARCH.md) — Workflow
  안의 동결된, 프로바이더 중립 실험 검색
- [docs/CONFIGURATION.md](docs/CONFIGURATION.md) — `config.toml`, 훅,
  constitution
- [docs/AUTHORIZATION_ORDER.md](docs/AUTHORIZATION_ORDER.md) — 모드, 훅, 권한
  규칙, 안전 기준선, 저장소 규칙, 승인, 샌드박스가 함께 적용되는 방식
- [docs/HOOKS.md](docs/HOOKS.md) — 11개의 TUI 수명 주기 훅 이벤트, 해당
  페이로드, 턴을 조정할 수 있는 3개 이벤트 (`codewhale exec`와 CLI 하위
  명령은 훅을 실행하지 않음)
- [docs/WEB.md](docs/WEB.md) — 루프백 전용 내장 브라우저 클라이언트와 일회성
  인증 경계

나머지 — 모드, 키 바인딩, 샌드박스 세부 사항, MCP, 런타임 API, 아키텍처 —
는 [docs](docs)와 [codewhale.net](https://codewhale.net/)에 있습니다.

## 기여

이슈, PR, 재현 절차, 로그, 기능 요청은 모두 이곳에서 실제 프로젝트 작업이며, 첫 기여도 환영합니다. PR을 그대로 병합할 수 없을 때는 메인테이너가 작동하는 부분을 거두어 반영하고, 작성자의 크레딧은 커밋, 변경 로그, [docs/CONTRIBUTORS.md](docs/CONTRIBUTORS.md)에 그대로 남습니다.

- [열려 있는 이슈](https://github.com/Hmbown/CodeWhale/issues) — 처음
  기여하기 좋은 작업이 여기에 있습니다
- [CONTRIBUTING.md](CONTRIBUTING.md) — 개발 환경 설정과 PR 흐름
- [docs/CONTRIBUTORS.md](docs/CONTRIBUTORS.md) — 이 프로젝트를 빚어 온
  모든 사람
- [Buy me a coffee](https://www.buymeacoffee.com/hmbown)

프로젝트를 시작하게 해 준 모델과 지원을 제공한 [DeepSeek](https://github.com/deepseek-ai), Whale Brother family로 맞이해 준 [DataWhale](https://github.com/datawhalechina) 🐋, 그리고 터미널 에이전트 경험에 함께 협력해 준 [OpenWarp](https://github.com/zerx-lab/warp)와 [Open Design](https://github.com/nexu-io/open-design)에 감사드립니다.

## 라이선스

[MIT](LICENSE). 독립 커뮤니티 프로젝트이며, 어떤 모델 프로바이더와도 제휴 관계가 없습니다.

![터미널에서 읽기 전용 scout 하위 에이전트 세 개를 병렬로 펼치는 Codewhale](assets/fanout.gif)

[![Star History Chart](https://star-history.dera.page/svg?repos=Hmbown/CodeWhale&type=date&legend=top-left)](https://star-history.dera.page/#Hmbown/CodeWhale&type=date)
