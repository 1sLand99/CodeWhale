import type { HomeDict } from "../types";

/**
 * Indonesian home dictionary — native copy for the Tidal Folio landing page,
 * in the current direction: your models, more capable together; agents
 * and control on your own machine; availability stated per surface as it
 * is today. Product vocabulary stays literal (Plan / Work / Operate, Ask /
 * Auto-Review / Full Access, Codewhale, TUI, codewhale exec, Fleet).
 */

export const home: HomeDict = {
  metaTitle: "Codewhale — Ciptakan apa yang Anda inginkan.",
  metaDescription:
    "Bangun perangkat lunak, kelola berkas, dan otomatisasi tugas dengan Codewhale. Pilih model yang dihosting atau model lokal, lalu ganti penyedia sesuai kebutuhan pekerjaan Anda.",
  kicker: "Agen AI sumber terbuka",
  heroTitleA: "Ciptakan apa yang Anda inginkan.",
  heroTitleB: "Pilih model Anda.",
  heroIntro:
    "{brand} menyediakan agen yang dapat membangun perangkat lunak, mengelola berkas, dan mengotomatiskan tugas. Gunakan model pilihan Anda dan ganti penyedia sesuai kebutuhan pekerjaan Anda.",
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
  chapterTerminalTitle: "Mulai dengan sesuatu yang ingin Anda buat.",
  gainHeading:
    "Wujudkan ide Anda.",
  gainLede:
    "Bangun proyek, teliti sebuah pertanyaan, atau otomatisasi tugas. Mulai dengan satu agen dan bagi pekerjaan yang lebih besar ke beberapa agen.",
  gain: [
    [
      "Bangun sesuatu",
      "Ubah ide menjadi perangkat lunak yang berfungsi. Agen Anda dapat mengedit berkas, menjalankan perintah, dan memeriksa hasilnya."
    ],
    [
      "Otomatisasi tugas berulang",
      "Buat skrip dan alur kerja untuk tugas yang berulang, lalu jalankan dari terminal."
    ],
    [
      "Pilih model Anda",
      "Hubungkan model yang dihosting atau model lokal. Bagikan bagian-bagian dari pekerjaan besar ke agen dengan model dan peran yang berbeda."
    ]
  ],
  chapterModels: "Model Anda",
  modelsHeading: "Temukan model yang sesuai dengan tugas Anda.",
  modelsBody:
    "Gunakan penyedia layanan model, hubungkan melalui gateway, atau jalankan model secara lokal. Pilih penyedia dan model untuk setiap sesi, lalu ubah saat Anda bekerja.",
  modelsFacts: [
    ["Hosted", "Kunci API Anda sendiri, disimpan dengan codewhale auth set --provider <id>"],
    ["Gateway", "Satu endpoint untuk banyak model, penyedia tetap Anda yang pilih"],
    ["Lokal", "vLLM, SGLang, Ollama di localhost — biasanya tanpa kunci"],
  ],
  modelsLink: "Jelajahi model dan penyedia",
  startHeading: "Mulai tugas pertama Anda.",
  startLede:
    "Pasang Codewhale, hubungkan model, dan sampaikan apa yang ingin Anda lakukan. Tambahkan Fleet saat Anda ingin beberapa agen berbagi pekerjaan.",
  startGuideLink: "Baca panduan memulai",
  startVocabularyLink: "Lihat kosakata produk",
  chapterAccount: "Dapatkan Codewhale",
  availabilityHeading:
    "Di mana Anda dapat menggunakan Codewhale.",
  availabilityLede:
    "Mulai di terminal. Aplikasi dan komputer cloud masih dalam pengembangan.",
  availability: [
    [
      "Terminal",
      "Dirilis",
      "Biner rilis GitHub untuk Linux, macOS, dan Windows; npm dan Cargo tersedia sebagai alternatif. Android di Termux masih dalam tahap pratinjau."
    ],
    [
      "Aplikasi web",
      "Pratinjau pengembangan",
      "Akses akun dan penautan peramban dalam pratinjau pengembangan."
    ],
    [
      "Desktop",
      "Build pengembangan",
      "Aplikasi macOS masih dalam pengembangan; unduhan untuk publik akan tersedia nanti."
    ],
    [
      "Komputer cloud",
      "Dalam pengembangan",
      "Komputer yang dihosting untuk menjalankan tugas Anda."
    ]
  ],
  availabilityNote:
    "Terminal dapat digunakan tanpa akun Codewhale. Biaya penggunaan model yang dihosting ditagih oleh penyedia Anda.",
  accountLink: "Buat akun",
  surfacesHeading: "Gunakan runtime di tempat pekerjaan berlangsung.",
  surfaces: [
    ["TUI", "Kerja terminal interaktif"],
    ["codewhale exec", "Skrip dan CI"],
    ["Klien web lokal","Antarmuka localhost; ruang kerja peramban yang dihosting masih dalam pengembangan"],
    ["Runtime API + MCP", "Integrasi lokal"],
    ["Fleet","Beberapa agen mengerjakan satu tugas"],
  ],
  runtimeLink: "Jelajahi integrasi",
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
