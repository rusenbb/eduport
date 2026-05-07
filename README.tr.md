# Eduport

[English](README.md) · **Türkçe**

<img src="src-tauri/icons/icon.svg" alt="Eduport" width="80" align="right" />

Üniversite başvurularını, programları, kişileri, laboratuvarları ve bunları birbirine bağlayan belgeler ile e-postaları takip eden tek kullanıcılı bir masaüstü uygulaması. Veriler düz Markdown + YAML olarak diskte saklanır — Dropbox / iCloud / Syncthing ile senkronize edilebilir, uygulamanın yanında doğrudan Obsidian'da düzenlenebilir.

**Durum:** v1 aktif geliştirme aşamasında. Yerel derlemeler uçtan uca çalışıyor; imzalı yükleyiciler ve CI dağıtımı sonraya bırakıldı.

## Öne çıkanlar

- **8 varlık tipi** — University, Lab, Person, Program, Application, Document, Email, Note — her biri YAML frontmatter ile `<slug>-<id>.md` olarak saklanır
- **İlişki grafiği olarak wikilink'ler** — `[[eth-zurich-K9p3]]` referansları id-eki ile çözümlenir, böylece Obsidian içindeki yeniden adlandırmalar bağlantıları kırmaz
- **Üç bölmeli arayüz** — sayım ve etiket çipleri içeren kenar çubuğu, list/kanban geçişi, yapılandırılmış alanlar ve render edilmiş gövde gösteren detay paneli
- **Başvuru kanban'ı** — kartları durum sütunları arasında sürükleyin; gövdedeki onay kutuları uygulama içinde tıklanabilir
- **⌘K komut paleti** — hızlı arama + SQLite FTS5 üzerinden tam metin arama
- **İlk çalıştırma karşılama akışı** — veri klasörü seçer, `attachments/` ve `notes/` alt klasörlerini oluşturur, isteğe bağlı örnek veri ekler
- **Yumuşak silme** — silinen öğeler `<veri klasörü>/.eduport-trash/` altına taşınır, uygulama içi Çöp Kutusu görünümünden geri yüklenebilir
- **Doğruluk kaynağı = sizin dosyalarınız** — SQLite indeksi veri klasörünün dışında (işletim sisteminin önbellek dizininde) yaşar; eksik veya bayatsa kendini sıfırdan yeniden oluşturur

## Mimari

```
┌─────────────────────────────────────────┐
│ Tauri kabuk (Rust)                      │
│  • Python sidecar'ı başlatır + denetler  │
│  • WebView'i sidecar URL'sine yönlendirir│
│  • Yerel diyalog + reveal köprüleri      │
└─────────────┬───────────────────────────┘
              │ HTTP loopback (127.0.0.1:<rastgele port>)
              ▼
┌─────────────────────────────────────────┐
│ Python sidecar (FastAPI + uvicorn)      │
│  • .md varlık dosyaları üzerine REST API │
│  • watchdog dosya izleyici               │
│  • markdown-it-py + YAML ayrıştırma      │
│  • SQLite + FTS5 indeksleyici            │
└─────────────────────────────────────────┘
```

## Teknoloji yığını

| Katman | Tercih |
|---|---|
| Yerel kabuk | Tauri 2 (Rust) |
| Frontend | SvelteKit + Svelte 5, Tailwind CSS v4, CodeMirror 6 |
| Markdown render (UI) | `marked` + özel wikilink/checkbox çıkarımı |
| Sidecar API | FastAPI + uvicorn, Pydantic v2 |
| Dosya izleyici | `watchdog` |
| Depolama / arama | stdlib `sqlite3` ile FTS5 |
| Masaüstü paketleme | Tauri externalBin + sidecar için PyInstaller |
| Geliştirme araçları | `uv` (Python), `npm` (frontend), `cargo` (Rust) |

## Proje yapısı

```
docs/                tasarım belgesi, paketleme notları, uygulama planları
frontend/            SvelteKit uygulaması — UI + API istemcisi
scripts/             derleme yardımcıları (sidecar paketleme, Tauri ön gereksinimleri)
sidecar/             Python (FastAPI) — uv ile yönetilen proje
src-tauri/           Rust kabuk (Tauri 2) — giriş noktası + yerel köprüler
```

## Ön koşullar

Proje macOS, Windows ve Linux'u hedefler. Üçü de şunlara ihtiyaç duyar:

- **Rust** 1.77.2+ (`rustup install stable`)
- **Node.js** 20+ ve npm
- **Python** 3.12+ ve [`uv`](https://docs.astral.sh/uv/)

Ayrıca işletim sistemine özgü Tauri ön koşulları (tam liste: <https://v2.tauri.app/start/prerequisites/>):

- **macOS:** Xcode Command Line Tools — `xcode-select --install`
- **Windows:** [Microsoft C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) ("Desktop development with C++" iş yüküyle). WebView2, Windows 10/11'de önceden yüklüdür.
- **Linux (Debian/Ubuntu):** `libwebkit2gtk-4.1-dev`, `libgtk-3-dev`, `librsvg2-dev`, `build-essential`

## Başlangıç

### Sidecar'ı bağımsız çalıştırın (rastgele bir portta API)

```bash
cd sidecar
uv sync
uv run eduport-sidecar
```

### Frontend'i tarayıcıda çalıştırın (yalnızca arayüz, Tauri köprüleri yok)

```bash
cd frontend
npm install
npm run dev          # http://localhost:5173
```

Yalnızca-tarayıcı modunda Tauri gerektiren diyaloglar (klasör seçici, yerel dosya gösterici) geliştirme yer tutucularına geri düşer.

### Tam masaüstü uygulamasını derleyip kurun

Gerçek bir kurulabilir uygulamaya en hızlı yol paketleme betiğidir — Python sidecar'ı PyInstaller ile paketler, SvelteKit frontend'ini derler ve `tauri build`'i çalıştırır:

```bash
python3 scripts/build_desktop.py
```

Linux'ta `src-tauri/target/release/bundle/` altında `.deb` ve `.rpm` üretir. Kurulum için `sudo apt install ./src-tauri/target/release/bundle/deb/eduport_*.deb` (veya dağıtımınızın eşdeğerini) kullanın. Gelişmiş bundle hedefleri ve CI notları için bkz. [docs/packaging.md](docs/packaging.md).

## Testler ve kontroller

```bash
# Sidecar (Python) — pytest + ruff
cd sidecar
uv run pytest -q
uv run ruff check src/ tests/

# Frontend — tip kontrolü + Svelte tanılaması
cd frontend
npm run check
```

## Belgeler

- **[Tasarım belgesi](docs/superpowers/specs/2026-05-06-eduport-design.md)** — varlık modeli, depolama düzeni, arayüz şekli, senkronizasyon semantiği, ayrıştırma hatası yönetimi — kanonik "neden" ve "ne"
- **[Paketleme](docs/packaging.md)** — yerel derlemeler, GitHub Actions iş akışı, imzalama notları
- **Uygulama planları** — katman bazlı ayrıntılı dökümler [`docs/superpowers/plans/`](docs/superpowers/plans/) altında

## Lisans

MIT (`src-tauri/Cargo.toml` içinde belirtildiği gibi). Bağımsız bir `LICENSE` dosyası yapılacaklar listesinde.
