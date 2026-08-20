<!-- source: README.md sha256:1f5bf984e975 -->
# Codewhale

Một coding agent mã nguồn mở cho terminal của bạn — mang theo model của riêng bạn.

Codewhale khởi đầu là một trải nghiệm gốc (native) cho DeepSeek. Từ đó, nó đã
phát triển thành một dự án do cộng đồng dẫn dắt: một coding harness hợp với một
cộng đồng quốc tế đang lớn dần và hỗ trợ càng nhiều model cùng provider càng
tốt — model mở trước tiên, hosted hay local, không cái nào được ưu ái hơn cái
nào.

Đưa cho nó một provider, một model và một nhiệm vụ. Nó đọc code của bạn, sửa
file, chạy lệnh, kiểm tra công việc của mình, rồi dừng lại khi nhiệm vụ hoàn
thành hoặc cần đến bạn. Đổi model giữa chừng bằng `/model`. Làm việc tương tác
trong TUI, hoặc chạy `codewhale exec` trong script và CI. Viết bằng Rust, giấy
phép MIT, và chạy trên máy của bạn.

Điều mà các harness khác không có: **bạn chọn model cho từng vai — và chúng
không cần giống nhau** — và **các agent trong Codewhale nói chuyện với nhau,
xuyên qua các model.** Một fleet ghim provider, model và mức suy luận cho
từng vai, nên một model rẻ và nhanh có thể chỉ huy một model đắt vốn suy
luận, hoặc một builder GLM có thể làm cùng nhiệm vụ với một reviewer Kimi.
Trong lúc chúng chạy, bạn có thể gửi ghi chú cho bất kỳ agent nào giữa
chừng, xem transcript của nó hoặc ngắt nó — và không chỉ theo quan hệ
cha-con: các task Codewhale riêng biệt trong cùng workspace trao đổi Agent
Mail bền vững, sống qua khởi động lại, được giao đúng một lần tại ranh giới
an toàn và che giấu thông tin đăng nhập. `/goal` giữ một mục tiêu dài qua
nhiều lượt, cho đến khi thật sự hoàn thành. Vai là file bạn chỉnh sửa, và cả
harness vẫn là của bạn.

Chúng tôi luôn tìm kiếm người đóng góp và cách cải thiện. Nếu một model hay
provider bạn dùng còn thiếu, hoặc có gì đó hỏng, báo cho chúng tôi biết là một
trong những điều hữu ích nhất bạn có thể làm — xem [Đóng góp](#đóng-góp).

[English](README.md) · [简体中文](README.zh-CN.md) · [日本語](README.ja-JP.md) · [한국어](README.ko-KR.md) · [Español](README.es-419.md) · [Português](README.pt-BR.md) · [Русский](README.ru.md) · [Українська](README.uk.md) · [Bahasa Indonesia](README.id.md) · [Français](README.fr.md) · [Deutsch](README.de.md) · [繁體中文](README.zh-TW.md) · [हिन्दी](README.hi.md) · [Türkçe](README.tr.md) · [Italiano](README.it.md) · [Polski](README.pl.md) · [العربية](README.ar.md) · [Català](README.ca.md) · [codewhale.net](https://codewhale.net/) · [Docs](docs) · [Changelog](CHANGELOG.md) · [Discord](https://discord.gg/37gfS3ksug)

[![CI](https://github.com/Hmbown/CodeWhale/actions/workflows/ci.yml/badge.svg)](https://github.com/Hmbown/CodeWhale/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/codewhale-cli?label=crates.io)](https://crates.io/crates/codewhale-cli)
[![npm](https://img.shields.io/npm/v/codewhale?label=npm)](https://www.npmjs.com/package/codewhale)
[![Discord](https://img.shields.io/badge/Discord-join%20the%20community-5865F2?logo=discord&logoColor=white)](https://discord.gg/37gfS3ksug)

![Codewhale chạy trong terminal](assets/screenshot.webp)

## Cài đặt

```bash
npm install -g codewhale
```

Cargo, Docker, Nix, Scoop, archive dựng sẵn, Android/Termux, và một mirror CNB
cho người dùng không truy cập được GitHub đều được hướng dẫn trong
[docs/INSTALL.md](docs/INSTALL.md). Chuyển từ `deepseek-tui` sang? Cấu hình và
session của bạn được giữ nguyên — xem [docs/REBRAND.md](docs/REBRAND.md).

## Sử dụng

```bash
codewhale auth set --provider deepseek   # or export ANTHROPIC_API_KEY, etc.
codewhale                                # open the TUI
codewhale exec "fix the failing test"    # headless
codewhale web                            # local browser client on 127.0.0.1
```


Trong TUI: `/model` đổi provider và model cùng lúc, `/fleet` chạy một đội
worker, `/undo` hoàn tác lượt gần nhất, và `/restore <N>` đưa workspace về một
ảnh chụp trước đó (`/restore` không tham số chỉ liệt kê chúng). Khi vùng soạn
thảo trống, `Tab` chuyển vòng qua Plan / Work / Operate; khi vùng soạn thảo có
chữ, `Tab` lại hoàn tất lệnh slash và nhắc `@`. `Shift+Tab` chuyển vòng qua tư
thế quyền Ask / Auto-Review / Full Access bất cứ lúc nào. `!` chạy một lệnh
shell qua đường phê duyệt bình thường.

## Tính năng

- **Model nào cũng được, provider nào cũng được.** DeepSeek, Claude, GPT, Kimi,
  GLM, hơn 30 provider, và vLLM, SGLang hay Ollama của riêng bạn — không cần
  key — đều chạy qua một runtime và một bộ công cụ. Danh mục theo dõi đội hình trực tiếp của từng provider — backend V4 Pro của DeepSeek (nhãn `DeepSeek-V4-Pro-0813`) vẫn gọi được bằng `deepseek-v4-pro`, Grok 4.6 là mặc định trực tiếp của xAI, còn OrcaRouter định tuyến qua `orcarouter/auto`. Ngân sách ngữ cảnh và giá
  lấy từ route thật; giá chưa rõ hiển thị là chưa rõ, chứ không phải $0.
- **Các agent nói chuyện với nhau — xuyên qua các model.** Mọi agent trong
  Codewhale đều với được trong lúc nó làm việc: `message` xếp một ghi chú
  vào hàng đợi cho sub-agent đang chạy, `followup` đánh thức nó với ghi chú
  của bạn tại ranh giới an toàn kế tiếp, `peek` đọc transcript của nó, và
  một lệnh ngắt chỉ dừng đúng lượt của nó. Điều này vượt ra ngoài cây
  cha-con: các task riêng biệt trong cùng workspace trao đổi **Agent Mail**
  bền vững — một bản tóm tắt bàn giao xếp hàng, sống qua khởi động lại, được
  giao đúng một lần tại ranh giới an toàn của người nhận và che giấu thông
  tin đăng nhập lẫn đường dẫn — nhờ đó một phiên GLM và một phiên Kimi có
  thể phối hợp trong hai terminal mà bạn không phải làm trung chuyển. Mỗi
  bên có thể là model khác nhau; Codewhale chuyên chở cuộc hội thoại.
- **Một harness do bạn viết.** Vai trò là những tệp bạn có thể đọc và sửa — mỗi
  vai trò một model, một tư thế công cụ và các chỉ dẫn thường trực — đặt trong dự
  án để cả nhóm dùng chung, hoặc cạnh các thiết lập cá nhân để đi theo bạn giữa
  các repo. Constitution ghi lại cách bạn muốn agent hành xử trong mọi phiên, để
  harness khớp với cách làm của bạn thay vì của chúng tôi.
- **Công việc có thể nối lại.** Một fleet ghi lại mỗi bước vào một sổ
  append-only, nên `fleet resume` nối lại từ chỗ bạn dừng. `/goal` giữ một
  mục tiêu bền bỉ mà agent theo đuổi lượt này qua lượt khác — có thể tạm
  dừng, tiếp tục và được khôi phục cùng phiên sau khi khởi động lại — còn
  `/workflows` mở một bảng điều khiển trực tiếp trên mọi lần chạy mà journal
  của workspace này giữ.
- **Công việc bạn có thể tiếp tục.** Fleet ghi lại từng bước vào sổ cái chỉ ghi
  thêm, nên `fleet resume` tiếp tục từ chỗ bạn dừng.

## Tích hợp

- **DeepSeek Harness (dsh) — kết nối qua Codewhale.**
  `codewhale integrations dsh connect` liên kết bản cài `@deepseek-ai/dsh`
  hiện có với tuyến provider, quyền và workspace Codewhale của bạn;
  `integrations dsh install-bundle` thêm gói plugin DSH tùy chọn để
  `dsh --profile codewhale` tự mang danh tính đó. Codewhale nắm quyền và
  vòng đời; dsh giữ nguyên phiên, profile và thông tin xác thực của riêng nó.
  Xem [docs/INTEGRATIONS_DSH.md](docs/INTEGRATIONS_DSH.md).
- **VS Code.** Bộ khung extension chính thức (`extensions/vscode`) mở
  Codewhale trong terminal tích hợp và cung cấp Agent View chỉ đọc qua
  runtime cục bộ. Đây là bản xem trước phát triển cục bộ, chưa phải bản phát
  hành marketplace.

## Tìm hiểu thêm

- [docs/PROVIDERS.md](docs/PROVIDERS.md) — mọi route provider: dịch vụ,
  gateway và cục bộ
- [docs/FLEET.md](docs/FLEET.md) — fleet, sổ cái và resume
- [docs/WORKFLOW_EXPERIMENTAL_SEARCH.md](docs/WORKFLOW_EXPERIMENTAL_SEARCH.md) — tìm kiếm thử nghiệm trong Workflow, đã đóng băng và trung lập với provider
- [docs/CONFIGURATION.md](docs/CONFIGURATION.md) — `config.toml`, hook và
  constitution
- [docs/AUTHORIZATION_ORDER.md](docs/AUTHORIZATION_ORDER.md) — cách các chế độ,
  hook, quy tắc quyền, mức an toàn tối thiểu, luật của repo, quy trình phê duyệt
  và sandbox phối hợp với nhau
- [docs/HOOKS.md](docs/HOOKS.md) — mười một sự kiện hook trong vòng đời TUI,
  payload của chúng và ba sự kiện có thể điều hướng một lượt (`codewhale exec`
  và các lệnh con CLI không kích hoạt hook)
- [docs/WEB.md](docs/WEB.md) — trình duyệt nhúng chỉ chạy trên loopback và
  ranh giới xác thực dùng một lần

Mọi thứ còn lại — chế độ, phím tắt, chi tiết sandbox, MCP, runtime API, kiến
trúc — nằm trong [docs](docs) và trên [codewhale.net](https://codewhale.net/).

## Đóng góp

Issue, PR, các bước tái hiện lỗi, log và yêu cầu tính năng đều là công việc
thực sự của dự án ở đây, và những đóng góp đầu tiên luôn được chào đón. Khi một
PR không thể merge nguyên trạng, maintainer sẽ harvest phần dùng được và tác
giả vẫn được ghi công — trong commit, trong changelog và trong
[docs/CONTRIBUTORS.md](docs/CONTRIBUTORS.md).

- [Issue đang mở](https://github.com/Hmbown/CodeWhale/issues) — những đóng góp
  đầu tiên phù hợp nằm ở đây
- [CONTRIBUTING.md](CONTRIBUTING.md) — thiết lập môi trường dev và quy trình PR
- [docs/CONTRIBUTORS.md](docs/CONTRIBUTORS.md) — tất cả những người đã góp
  phần định hình dự án
- [Buy me a coffee](https://www.buymeacoffee.com/hmbown)

Cảm ơn [DeepSeek](https://github.com/deepseek-ai) vì các model và sự hỗ trợ đã
khởi đầu dự án, [DataWhale](https://github.com/datawhalechina) 🐋 vì đã chào
đón chúng tôi vào đại gia đình Whale Brother, và
[OpenWarp](https://github.com/zerx-lab/warp) cùng
[Open Design](https://github.com/nexu-io/open-design) vì đã hợp tác xây dựng
trải nghiệm agent trên terminal.

## Giấy phép

[MIT](LICENSE). Dự án cộng đồng độc lập; không trực thuộc bất kỳ nhà cung cấp
model nào.

![Codewhale phân nhánh ba subagent scout chỉ đọc trong terminal](assets/fanout.gif)

[![Star History Chart](https://star-history.dera.page/svg?repos=Hmbown/CodeWhale&type=date&legend=top-left)](https://star-history.dera.page/#Hmbown/CodeWhale&type=date)
