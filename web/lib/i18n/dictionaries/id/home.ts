import type { HomeDict } from "../types";

/**
 * Indonesian home dictionary — native copy for the Tidal Folio landing page,
 * in the current direction: your models, more capable together; agents
 * and control on your own machine; availability stated per surface as it
 * is today. Product vocabulary stays literal (Plan / Work / Operate, Ask /
 * Auto-Review / Full Access, Codewhale, TUI, codewhale exec, Fleet).
 */

export const home: HomeDict = {
  metaTitle: "Codewhale — Model Anda. Lebih mampu bersama.",
  metaDescription:
    "Codewhale adalah sistem komputasi agentik sumber terbuka. Bawa model yang sudah Anda pakai — hosted, lewat gateway, atau lokal — dan biarkan mereka bekerja bersama di terminal Anda, di mesin Anda, di bawah kendali Anda. Rust, MIT.",
  kicker: "Komputasi agentik, dengan syarat Anda",
  heroTitleA: "Model Anda.",
  heroTitleB: "Lebih mampu bersama.",
  heroIntro:
    "{brand} mengumpulkan model yang sudah Anda pakai ke dalam satu terminal dan membuat mereka bekerja seperti satu awak — membaca kode, mengedit, menjalankan pemeriksaan — sementara Anda yang menentukan apa yang boleh dilakukan tiap model. Sumber terbuka, di mesin Anda.",
  getCodewhale: "Dapatkan Codewhale",
  exploreProduct: "Jelajahi produk",
  shotPreview: "Pratinjau terminal",
  shotBuild: "build pengembangan v{version}",
  screenshotAlt:
    "Build pengembangan Codewhale v0.9.12 di terminal: tanda paus dari titik braille, sesi baru tanpa riwayat, kotak pesan, dan footer yang menampilkan Full Access, mode Work, dua tugas terjadwal, server MCP yang sedang terhubung, dan model GLM-5.3 pada upaya maksimal",
  latestRelease: "Rilis terbaru {tag}",
  releaseUnavailable: "Status rilis tidak tersedia",
  currentSource: "Sumber",
  sourceCandidate: "Belum dirilis",
  providerRoutes: "{count} penyedia",
  publishedRelease: "dirilis",
  figcaptionSourceCandidate: "belum dirilis",
  chapterTerminal: "Terminal Anda",
  chapterTerminalTitle: "Tempat yang akrab untuk memulai.",
  gainHeading:
    "Yang Anda dapat bukan chatbot, melainkan daya ungkit atas model yang sudah Anda bayar.",
  gainLede:
    "Satu sesi bisa menampung beberapa model sekaligus, masing-masing dalam peran yang Anda beri, semuanya bekerja di repositori yang sama dengan aturan yang sama.",
  gain: [
    ["Model Anda", "Kunci hosted, sebuah gateway, atau runtime lokal tanpa kunci sama sekali. Sematkan model berbeda untuk tiap peran dan pertahankan penyedia yang Anda pilih — nama model tidak pernah mengganti penyedia untuk Anda."],
    ["Agen yang cakap", "Mode Plan, Work, dan Operate; satu fleet sub-agen untuk satu pekerjaan; alat untuk berkas, shell, web, dan MCP; sesi yang bisa disimpan, dilanjutkan, dan dikembalikan."],
    ["Kendali di mesin Anda", "Ask, Auto-Review, atau Full Access — Anda menentukan seberapa jauh ia bekerja sebelum bertanya. Berjalan lokal, dalam sandbox jika OS mengizinkan, dengan log audit yang bisa Anda baca."],
  ],
  chapterModels: "Model Anda",
  modelsHeading: "Bawa yang Anda punya. Jangan ubah yang tidak Anda pilih.",
  modelsBody:
    "Codewhale hadir dengan {count} penyedia dan memperlakukan semuanya setara. Simpan kunci sekali, sebutkan model, dan rute tetap persis seperti yang Anda atur. Model lokal lewat vLLM, SGLang, atau Ollama tidak butuh kunci.",
  modelsFacts: [
    ["Hosted", "Kunci API Anda sendiri, disimpan dengan codewhale auth set"],
    ["Gateway", "Satu endpoint untuk banyak model, penyedia tetap Anda yang pilih"],
    ["Lokal", "vLLM, SGLang, Ollama di localhost — biasanya tanpa kunci"],
  ],
  modelsLink: "Lihat semua penyedia",
  startHeading: "Empat langkah menuju sesi pertama.",
  startLede:
    "Pasang, buka sesi tanpa kunci, hubungkan penyedia, lalu siapkan fleet saat satu model tidak cukup.",
  startGuideLink: "Baca panduan memulai",
  startVocabularyLink: "Lihat kosakata produk",
  chapterAccount: "Di mana ia berjalan hari ini",
  availabilityHeading:
    "Tersedia sekarang, dalam pengembangan, dan belum — dinyatakan apa adanya.",
  availabilityLede:
    "Terminal adalah produk yang sudah dirilis. Yang lain dicantumkan sesuai keadaannya yang sebenarnya.",
  availability: [
    ["Terminal", "Dirilis", "npm, Cargo, dan biner siap pakai untuk Linux, macOS, dan Windows. Android di Termux masih pratinjau."],
    ["Aplikasi web", "Masuk akun tersedia", "Masuk atau buat akun di app.codewhale.net. Meja kerja di peramban sendiri masih pratinjau pengembangan."],
    ["Desktop", "Build pengembangan", "Ada build alfa untuk macOS, Linux, dan Windows. Belum ada aplikasi desktop yang dirilis."],
    ["Komputer cloud", "Belum tersedia", "Menjalankan pekerjaan di komputer yang di-host masih dalam pengembangan. Halaman ini akan mengatakannya saat sudah berfungsi."],
  ],
  availabilityNote:
    "Terminal tidak memerlukan akun. Akun tidak pernah dengan sendirinya menjadi paket berbayar, dan tidak ada apa pun di situs ini yang bisa menagih Anda.",
  accountLink: "Buat akun",
  surfacesHeading: "Gunakan runtime di tempat pekerjaan berlangsung.",
  surfaces: [
    ["TUI", "Kerja terminal interaktif"],
    ["codewhale exec", "Skrip dan CI"],
    ["Klien Web", "Klien peramban khusus loopback"],
    ["Runtime API + MCP", "Integrasi lokal"],
    ["fleet", "Kerja multi-agen yang tahan lama"],
  ],
  runtimeLink: "Lihat antarmuka runtime dan catatan stabilitas",
  installBandHeading: "Mulai dengan satu perintah.",
  copy: "Salin",
  copied: "Tersalin ✓",
  binaries: "Biner",
  chinaMirrors: "Mirror Tiongkok",
  installGuideLink: "Baca panduan instalasi",
  communityHeading: "Dibangun secara terbuka",
  communityBody:
    "Berlisensi MIT dan dibentuk oleh para kontributor di berbagai runtime, penyedia, platform, dokumentasi, dan pengujian.",
  communityLinksAria: "Tautan komunitas",
  contribute: "Kontribusi",
};
