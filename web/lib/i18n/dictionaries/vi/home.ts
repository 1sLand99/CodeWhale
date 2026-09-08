import type { HomeDict } from "../types";

/**
 * Vietnamese home dictionary — native copy for the Tidal Folio landing page,
 * in the current direction: your models, more capable together; agents
 * and control on your own machine; availability stated per surface as it
 * is today. Product vocabulary stays literal (Plan / Work / Operate, Ask /
 * Auto-Review / Full Access, Codewhale, TUI, codewhale exec, Fleet).
 */

export const home: HomeDict = {
  metaTitle: "Codewhale — Tạo những gì bạn muốn.",
  metaDescription:
    "Phát triển phần mềm, làm việc với tệp và tự động hóa tác vụ cùng Codewhale. Chọn mô hình từ nhà cung cấp hoặc mô hình cục bộ, rồi đổi nhà cung cấp theo nhu cầu công việc.",
  kicker: "Tác tử AI mã nguồn mở",
  heroTitleA: "Tạo những gì bạn muốn.",
  heroTitleB: "Chọn mô hình bạn muốn dùng.",
  heroIntro:
    "{brand} mang đến các tác tử có thể phát triển phần mềm, làm việc với tệp và tự động hóa tác vụ. Dùng mô hình bạn chọn và đổi nhà cung cấp theo nhu cầu công việc.",
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
  chapterTerminalTitle: "Bắt đầu với điều bạn muốn tạo ra.",
  gainHeading:
    "Biến ý tưởng thành hành động.",
  gainLede:
    "Xây dựng dự án, tìm hiểu một vấn đề hoặc tự động hóa tác vụ. Bắt đầu với một tác tử, rồi chia việc lớn hơn cho nhiều tác tử.",
  gain: [
    [
      "Bắt tay xây dựng",
      "Biến ý tưởng thành phần mềm chạy được. Các tác tử có thể chỉnh sửa tệp, chạy lệnh và kiểm tra kết quả."
    ],
    [
      "Tự động hóa việc lặp lại",
      "Tạo tập lệnh và quy trình cho những tác vụ thường lặp lại, rồi chạy từ terminal."
    ],
    [
      "Chọn mô hình bạn muốn dùng",
      "Kết nối mô hình từ nhà cung cấp hoặc mô hình cục bộ. Chia một công việc lớn cho các tác tử dùng mô hình và đảm nhiệm vai trò khác nhau."
    ]
  ],
  chapterModels: "Mô hình của bạn",
  modelsHeading: "Tìm mô hình phù hợp với tác vụ.",
  modelsBody:
    "Dùng dịch vụ của nhà cung cấp, kết nối qua cổng trung gian hoặc chạy mô hình cục bộ. Chọn nhà cung cấp và mô hình cho từng phiên, rồi thay đổi trong lúc làm việc.",
  modelsFacts: [
    ["Hosted", "Khóa API của bạn, lưu bằng codewhale auth set --provider <id>"],
    ["Gateway", "Một endpoint cho nhiều mô hình, nhà cung cấp vẫn do bạn chọn"],
    ["Cục bộ", "vLLM, SGLang, Ollama trên localhost — thường không cần khóa"],
  ],
  modelsLink: "Khám phá mô hình và nhà cung cấp",
  startHeading: "Bắt đầu tác vụ đầu tiên.",
  startLede:
    "Cài đặt Codewhale, kết nối một mô hình và cho biết bạn muốn làm gì. Thêm Fleet khi muốn nhiều tác tử cùng chia sẻ công việc.",
  startGuideLink: "Đọc hướng dẫn bắt đầu",
  startVocabularyLink: "Xem thuật ngữ sản phẩm",
  chapterAccount: "Tải Codewhale",
  availabilityHeading: "Bạn có thể dùng Codewhale ở đâu.",
  availabilityLede:
    "Bắt đầu từ terminal. Ứng dụng và máy tính đám mây đang được phát triển.",
  availability: [
    [
      "Terminal",
      "Đã phát hành",
      "Các bản nhị phân phát hành trên GitHub dành cho Linux, macOS và Windows; bạn cũng có thể cài qua npm hoặc Cargo. Phiên bản Android trên Termux là bản xem trước."
    ],
    [
      "Ứng dụng web",
      "Bản xem trước đang phát triển",
      "Truy cập tài khoản và ghép nối trình duyệt trong bản xem trước đang phát triển."
    ],
    [
      "Máy tính để bàn",
      "Bản phát triển",
      "Ứng dụng macOS đang được phát triển; bản tải xuống công khai sẽ có sau."
    ],
    [
      "Máy tính đám mây",
      "Đang phát triển",
      "Máy tính do nhà cung cấp vận hành để chạy tác vụ của bạn."
    ]
  ],
  availabilityNote:
    "Terminal không cần tài khoản Codewhale. Nhà cung cấp của bạn tính phí sử dụng các mô hình do họ vận hành.",
  accountLink: "Tạo tài khoản",
  surfacesHeading: "Dùng runtime ngay nơi công việc diễn ra.",
  surfaces: [
    ["TUI", "Làm việc tương tác trong terminal"],
    ["codewhale exec", "Script và CI"],
    ["Trình khách web cục bộ","Giao diện localhost; không gian làm việc trên trình duyệt do máy chủ cung cấp vẫn đang được phát triển"],
    ["Runtime API + MCP", "Tích hợp cục bộ"],
    ["Fleet","Nhiều tác tử cùng làm một việc"],
  ],
  runtimeLink: "Khám phá các tích hợp",
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
