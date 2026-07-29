# Agent Fleet (Armada Agen)

Agent Fleet adalah control plane yang mengutamakan lokal (*local-first*) untuk eksekusi banyak pekerja (*multi-worker*) yang tahan lama. Fleet **bukanlah** mesin eksekusi terpisah: seorang pekerja fleet (*fleet worker*) adalah eksekusi `codewhale exec` tanpa antarmuka yang diluncurkan dan dilacak oleh fleet secara permanen.

Gunakan Fleet daripada pembagian tugas agen yang berumur pendek ketika pekerjaan membutuhkan percobaan ulang (*retry*), ketahanan terhadap mode tidur/restart komputer, eksekusi jarak jauh, bukti tanda terima (*receipts*), atau jejak audit ber-ledger.

---

## Perintah Dasar CLI Fleet

```sh
codewhale fleet init
codewhale fleet run tasks.json --max-workers 4
codewhale fleet status
codewhale fleet inspect <worker-id>
codewhale fleet logs <worker-id>
codewhale fleet artifacts <worker-id>
codewhale fleet interrupt <worker-id>
codewhale fleet restart <worker-id>
codewhale fleet resume <run-id>
codewhale fleet stop --all
```

`codewhale fleet resume <run-id>` adalah perintah pemulihan setelah sistem terhenti: perintah ini memutar ulang ledger, merekonsiliasi tugas yang terhenti (Mencoba lagi sesuai anggaran tugas, atau melaporkannya jika gagal), lalu menampilkan status setelah pemulihan. Perintah ini aman dijalankan setelah laptop terbangun dari mode tidur atau setelah restart runtime.

---

## Lokasi Penyimpanan Status

Status Fleet disimpan di dalam ruang kerja di bawah `.codewhale/fleet.jsonl`. Log pekerja dan log adapter disimpan di bawah `.codewhale/fleet/` dan `.codewhale/fleet-host/`.

### Perbedaan Status Interaktif dan Persisten

- Di dalam TUI: Perintah `/fleet status` (atau `/subagents`) menampilkan sub-agen yang terhubung ke sesi interaktif saat ini.
- Di dalam Shell: Perintah `codewhale fleet status` membaca riwayat eksekusi Fleet yang tersimpan di ledger `.codewhale/fleet.jsonl`.
