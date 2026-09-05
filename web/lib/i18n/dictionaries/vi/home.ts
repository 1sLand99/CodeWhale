import type { HomeDict } from "../types";

/**
 * Vietnamese home dictionary — native copy for the Tidal Folio landing page,
 * in the current direction: your models, more capable together; agents
 * and control on your own machine; availability stated per surface as it
 * is today. Product vocabulary stays literal (Plan / Work / Operate, Ask /
 * Auto-Review / Full Access, Codewhale, TUI, codewhale exec, Fleet).
 */

export const home: HomeDict = {
  metaTitle: "Codewhale — Mô hình của bạn. Mạnh hơn khi làm việc cùng nhau.",
  metaDescription:
    "Codewhale là hệ thống điện toán tác tử mã nguồn mở. Mang những mô hình bạn đang dùng — hosted, qua gateway, hoặc cục bộ — vào terminal và để chúng làm việc cùng nhau trên máy của bạn, dưới sự kiểm soát của bạn. Rust, MIT.",
  kicker: "Điện toán tác tử, theo điều kiện của bạn",
  heroTitleA: "Mô hình của bạn.",
  heroTitleB: "Mạnh hơn khi làm việc cùng nhau.",
  heroIntro:
    "{brand} gom các mô hình bạn đang dùng vào một terminal và để chúng làm việc như một thủy thủ đoàn — đọc mã, sửa tệp, chạy kiểm tra — trong khi bạn quyết định mỗi mô hình được phép làm gì. Mã nguồn mở, chạy trên máy của bạn.",
  getCodewhale: "Tải Codewhale",
  exploreProduct: "Khám phá sản phẩm",
  shotPreview: "Xem trước terminal",
  shotBuild: "bản phát triển v{version}",
  screenshotAlt:
    "Bản phát triển Codewhale v0.9.12 trong terminal: biểu tượng cá voi bằng chữ nổi, phiên mới chưa có lịch sử, ô soạn tin, và thanh chân trang hiển thị Full Access, chế độ Work, hai tác vụ đã lên lịch, máy chủ MCP đang kết nối và mô hình GLM-5.3 ở mức tối đa",
  latestRelease: "Bản phát hành mới nhất {tag}",
  releaseUnavailable: "Không có trạng thái phát hành",
  currentSource: "Mã nguồn",
  sourceCandidate: "Chưa phát hành",
  providerRoutes: "{count} nhà cung cấp",
  publishedRelease: "đã phát hành",
  figcaptionSourceCandidate: "chưa phát hành",
  chapterTerminal: "Terminal của bạn",
  chapterTerminalTitle: "Một nơi quen thuộc để bắt đầu.",
  gainHeading:
    "Thứ bạn nhận được không phải chatbot, mà là đòn bẩy cho những mô hình bạn đã trả tiền.",
  gainLede:
    "Một phiên có thể giữ nhiều mô hình cùng lúc, mỗi mô hình một vai trò bạn giao, tất cả cùng làm việc trong một kho mã theo cùng một bộ quy tắc.",
  gain: [
    ["Mô hình của bạn", "Khóa hosted, một gateway, hoặc runtime cục bộ không cần khóa. Ghim mỗi vai trò một mô hình khác nhau và giữ nguyên nhà cung cấp bạn chọn — tên mô hình không bao giờ tự đổi nhà cung cấp."],
    ["Tác tử có năng lực", "Các chế độ Plan, Work và Operate; một fleet tác tử con cho một việc; công cụ cho tệp, shell, web và MCP; phiên lưu được, tiếp tục được và quay lui được."],
    ["Kiểm soát trên máy của bạn", "Ask, Auto-Review hoặc Full Access — bạn đặt mức nó được làm trước khi hỏi. Chạy cục bộ, có sandbox khi hệ điều hành cho phép, kèm nhật ký kiểm toán bạn đọc được."],
  ],
  chapterModels: "Mô hình của bạn",
  modelsHeading: "Mang theo những gì bạn có. Không đổi gì bạn chưa chọn.",
  modelsBody:
    "Codewhale đi kèm {count} nhà cung cấp và đối xử bình đẳng với tất cả. Lưu khóa một lần, đặt tên mô hình, và tuyến đường giữ nguyên như bạn đặt. Mô hình cục bộ qua vLLM, SGLang hoặc Ollama không cần khóa.",
  modelsFacts: [
    ["Hosted", "Khóa API của bạn, lưu bằng codewhale auth set"],
    ["Gateway", "Một endpoint cho nhiều mô hình, nhà cung cấp vẫn do bạn chọn"],
    ["Cục bộ", "vLLM, SGLang, Ollama trên localhost — thường không cần khóa"],
  ],
  modelsLink: "Xem mọi nhà cung cấp",
  startHeading: "Bốn bước tới phiên đầu tiên.",
  startLede:
    "Cài đặt, mở phiên không cần khóa, kết nối nhà cung cấp, rồi lập fleet khi một mô hình là chưa đủ.",
  startGuideLink: "Đọc hướng dẫn bắt đầu",
  startVocabularyLink: "Xem thuật ngữ sản phẩm",
  chapterAccount: "Nơi nó chạy hôm nay",
  availabilityHeading: "Đã có, đang phát triển, và chưa có — nói thẳng.",
  availabilityLede:
    "Terminal là sản phẩm đã phát hành. Mọi thứ khác được liệt kê đúng trạng thái thực tế.",
  availability: [
    ["Terminal", "Đã phát hành", "npm, Cargo và binary dựng sẵn cho Linux, macOS và Windows. Android trên Termux là bản xem trước."],
    ["Ứng dụng web", "Đăng nhập và điều khiển từ xa đã có", "Đăng nhập hoặc tạo tài khoản, rồi gõ /rc trong một phiên cục bộ đang chạy để tiếp tục chính phiên đó từ trình duyệt. Phần còn lại của bàn làm việc trên trình duyệt vẫn là bản xem trước phát triển."],
    ["Máy tính để bàn", "Bản phát triển", "Có bản alpha cho macOS, Linux và Windows. Chưa có ứng dụng desktop phát hành chính thức."],
    ["Máy tính đám mây", "Chưa có", "Việc chạy công việc trên máy tính được host đang được phát triển. Trang này sẽ nói khi nó hoạt động."],
  ],
  availabilityNote:
    "Terminal không cần tài khoản. Tài khoản tự nó không bao giờ là gói trả phí, và không gì trên trang này có thể tính phí bạn.",
  accountLink: "Tạo tài khoản",
  surfacesHeading: "Dùng runtime ngay nơi công việc diễn ra.",
  surfaces: [
    ["TUI", "Làm việc tương tác trong terminal"],
    ["codewhale exec", "Script và CI"],
    ["Ứng dụng web", "Chạy trong trình duyệt, chỉ qua loopback"],
    ["Runtime API + MCP", "Tích hợp cục bộ"],
    ["fleet", "Công việc nhiều tác tử, bền vững"],
  ],
  runtimeLink: "Xem các giao diện runtime và ghi chú về độ ổn định",
  installBandHeading: "Bắt đầu chỉ bằng một lệnh.",
  copy: "Sao chép",
  copied: "Đã sao chép ✓",
  binaries: "Bản nhị phân",
  chinaMirrors: "Mirror Trung Quốc",
  installGuideLink: "Đọc hướng dẫn cài đặt",
  communityHeading: "Xây dựng công khai",
  communityBody:
    "Giấy phép MIT, được định hình bởi những người đóng góp trên khắp runtime, nhà cung cấp, nền tảng, tài liệu và kiểm thử.",
  communityLinksAria: "Liên kết cộng đồng",
  contribute: "Đóng góp",
};
