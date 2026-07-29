# Matriks Lokalisasi (Localization Matrix)

Dokumen pelacakan kanonik untuk setiap bahasa yang didukung, sedang dibangun, direncanakan, atau ditunda oleh Codewhale.

> **Catatan Cakupan (2026-07-12):** Matriks ini mencakup tiga permukaan utama — paket bahasa TUI (`crates/tui/locales/`), README terjemahan (root repositori), dan situs web (`web/`). Ketiganya rilis pada ritme yang berbeda, sehingga suatu bahasa bisa berstatus **shipped** di satu permukaan dan **planned** di permukaan lain.

---

## Keterangan Status

| Status | Arti |
|--------|------|
| **shipped** | Aktif di codewhale.net dan/atau diterbitkan sebagai README mandiri / paket TUI |
| **partial** | Rilis tetapi belum mencakup seluruh bagian; dalam proses pengisian |
| **planned** | Diprioritaskan secara eksplisit untuk gelombang rilis berikutnya |
| **deferred** | Diakui tetapi belum dijadwalkan; memerlukan pengujian tata letak, dukungan jembatan, atau kontributor komunitas |

---

## Paket Bahasa TUI

Paket TUI di bawah `crates/tui/locales/` adalah permukaan terjemahan terbesar di repositori. `en.json` adalah acuan utama; sebuah paket dianggap **lengkap** (complete) jika memiliki paritas kunci persis dengannya, yang ditegakkan oleh `scripts/check-tui-locale-parity.py` (CI) dan pengujian paritas di `crates/tui/src/localization.rs`.

| Bahasa | Berkas | Kunci vs `en.json` (1134) | Status | Catatan |
|--------|------|--------------------------|--------|-------|
| Bahasa Inggris | `en.json` | 1134/1134 | **shipped** | Paket acuan utama. |
| Bahasa Indonesia | `id.json` | 1134/1134 | **shipped** | Lengkap (100% paritas kunci). |
| Bahasa Jepang | `ja.json` | 1134/1134 | **shipped** | Lengkap. |
| Bahasa Mandarin Sederhana | `zh-Hans.json` | 1134/1134 | **shipped** | Lengkap. |
| Bahasa Mandarin Tradisional | `zh-Hant.json` | 480/1134 | **partial** | Hanya inti pengaturan; kunci yang hilang menggunakan fallback Bahasa Inggris. |
| Bahasa Portugis Brasil | `pt-BR.json` | 1134/1134 | **shipped** | Lengkap. |
| Bahasa Spanyol Amerika Latin | `es-419.json` | 1134/1134 | **shipped** | Lengkap. |
| Bahasa Vietnam | `vi.json` | 1134/1134 | **shipped** | Lengkap. |
| Bahasa Korea | `ko.json` | 1134/1134 | **shipped** | Lengkap. |

---

## README Terjemahan

| Bahasa | Berkas | Status | Catatan |
|--------|------|--------|---------|
| Bahasa Inggris | `README.md` | **shipped** | Sumber utama kanonik |
| Bahasa Indonesia | `README.id.md` | **shipped** | Ditinjau per rilis (#4789) |
| Bahasa Mandarin Sederhana | `README.zh-CN.md` | **shipped** | Ditinjau per rilis |
| Bahasa Jepang | `README.ja-JP.md` | **shipped** | Ditinjau per rilis |
| Bahasa Vietnam | `README.vi.md` | **shipped** | Ditinjau per rilis |
| Bahasa Korea | `README.ko-KR.md` | **shipped** | Ditinjau per rilis |
| Bahasa Spanyol | `README.es-419.md` | **shipped** | Ditinjau per rilis |
| Bahasa Portugis Brasil | `README.pt-BR.md` | **shipped** | Ditinjau per rilis |

---

## Cara Menambahkan Paket Bahasa Baru

1. **Paket TUI**:
   - Buat berkas `crates/tui/locales/<tag>.json` berisi seluruh kunci di `en.json`.
   - Tambahkan varian `Locale` pada `crates/tui/src/localization.rs` dan daftarkan di `config_ui.rs`.
   - Jalankan `python3 scripts/check-tui-locale-parity.py` dan `cargo test -p codewhale-tui localization`.

2. **README**:
   - Terjemahkan `README.md` menjadi `README.<tag>.md`.
   - Perbarui stempel sumber sha256 dan jalankan `python3 scripts/check-readme-translations.py`.
