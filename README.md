# Search Tool

Ultra-hafif, disk-first Windows arama ve güvenli bakım motoru.

## Proje durumu ve devam noktası

Bu repository tek başına geliştirmeye devam etmek için yeterli olacak şekilde tutulur:

- `docs/HANDOFF.md` — sohbet geçmişine ihtiyaç duymadan güncel durum ve devam sırası
- `docs/STATUS.md` — uygulanmış özellikler ve mevcut release durumu
- `docs/TEST_MATRIX.md` — hangi testlerin PASS/PARTIAL/BLOCKED/TODO olduğu
- `docs/ROADMAP.md` — önceliklendirilmiş kalan işler
- `docs/VALIDATION.md` — doğrulama komutları ve fiziksel Windows sonuçları
- `docs/ARCHITECTURE.md` — mimari özet

## Hedef

**Ürün hedefi:** Windows Search kadar doğal hissettiren, fakat yerel dosya aramasında çok daha hızlı, düzenli ve düşük kaynak tüketimli native Windows araması.

- Aramada diski yeniden taramaz.
- NTFS MFT ile ilk index, USN Journal ile incremental güncelleme yapar.
- Ana runtime Rust'tır; Node/Python/JVM/Electron yoktur.
- AI yalnız gerektiğinde yüklenen yaklaşık 1 MiB INT8 intent modelidir.
- Ağ erişimi yalnız açıkça gereken web lookup yolunda kullanılır.
- Ağır background işler Resource Governor tarafından throttle/pause edilir.
- Background service/maintenance Windows scheduler seviyesinde de düşük CPU/I/O önceliğinde çalışır.
- Temizlik doğrudan delete etmez; güvenli öğeler önce quarantine'e gider.

## Bileşenler

- `search-tool.exe` — CLI / index / search / bakım
- `search-tool-gui.exe` — Windows Search tarzı native Win32 resident arayüz; tema, Mica/Acrylic, renk ve transparanlık desteği
- `search-tool-service.exe` — USN sync + düşük öncelikli compaction
- `search-tool-worker.exe` — izole içerik/IFilter worker
- `search-tool-bench.exe` — sentetik/fresh-process performans ölçümü

## Ana özellikler

- Multi-volume search across all fixed NTFS drives
- Exact / prefix / ranked / fuzzy arama
- `ext:`, `size:`, `path:`, `date:`, `type:`, `app:` filtreleri
- Alias/relationship araması (`node js` → npm/npx/package.json vb.)
- Metin ve kod içerik indexi; Windows IFilter ile zengin doküman worker'ı
- Disk-first sparse checkpoint indexleri
- USN delta overlay ve bounded-memory external compaction; yoğun foreground kullanımda 2 MiB canlı-delta tavanı
- Sync/compact/repair mutasyonları OS-level per-index lock ile serialize edilir
- USN append-only delta okuyucusu eşzamanlı yazım/crash sırasında görülen partial final delta kaydını commit edilmemiş tail olarak yok sayar; tam okunabilen bozuk kayıtlar fail-closed kalır
- USN-generation freshness markers for metadata/content sidecars
- Duplicate analizi: size → sample → full fingerprint → byte doğrulama; volume'lar arası da çalışır
- Knowledge/classification + cleanup safety policy
- Quarantine / restore / purge
- Tiny-AI natural-language route
- Privacy-sanitized, cache'li isteğe bağlı web resolver
- Index verify / deep verify / sidecar repair
- Windows Search tarzı resident panel: Tümü / Dosyalar / Klasörler / İçerik sekmeleri, sonuç yolu, çift tık/Enter ile açma ve single-instance query IPC
- Tema sistemi: native canlı Tema menüsünden system/dark/light, Acrylic/Mica/none, accent renk seçici ve %60/%75/%90/%100 transparanlık; gelişmiş background/surface/text/muted/panel boyutu ayarları `%APPDATA%\SearchTool\ui.conf` içinde kalır
- Desteklenen Windows entegrasyonu: özel `searchtool:` protokolü ve Default Apps içinde belgelenmiş `search:` protokolü için Search Tool adayı; Explorer `search:` çağrılarındaki `crumb=location:` klasör kapsamını korur ve klasör/sürücü/klasör-arka-plan menüsüne scoped “Search with Search Tool” komutu ekler; Tema menüsü doğrudan Windows Varsayılan Uygulamalar sayfasını açar, installer varsayılanı zorla ele geçirmez

## Filtre örnekleri

```text
node ext:exe
report size:>10mb
config path:projects type:file
node app:"node js"
backup date:2026-01-01..2026-09-23
```

Boyut/tarih filtreleri için önce düşük öncelikli metadata sidecar oluşturulur:

```powershell
search-tool metadata-build C: C:\ProgramData\SearchTool\index\C.stidx
```

## Derleme ve doğrulama

Windows PowerShell (Rust 1.98 önerilir, MSRV 1.89):

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace --release
```

Tam kontrol:

```powershell
.\scripts\verify.ps1
```

## İlk kurulum

Admin PowerShell:

```powershell
.\scripts\install.ps1 -Drive C:
```

Kaldırma varsayılan olarak index/veriyi korur:

```powershell
.\scripts\uninstall.ps1
```

Veriyi de silmek için:

```powershell
.\scripts\uninstall.ps1 -PurgeData
```


## Teşhis ve bakım

Tek index veya tüm index klasörü için:

```powershell
search-tool doctor C:\ProgramData\SearchTool\index
search-tool repair C:\ProgramData\SearchTool\index
search-tool maintain C:\ProgramData\SearchTool\index
```

- `doctor`: open/verify, delta, RAM hint, service state ve metadata/content freshness durumunu gösterir.
- `repair`: recoverable name/id/checkpoint sidecar'larını yeniden üretir; klasör verilirse tüm volume'ları işler.
- `maintain`: crash recovery + gerektiğinde sidecar repair + delta compaction + final verify yapar. Windows service çalışıyorsa güvenlik için mutasyon yapmayı reddeder.

## Sentetik ölçek testi

Bu değerler Linux sandbox depolamasında ölçülmüştür; HDD latency sonucu olarak yorumlanmamalıdır. RAM/index ölçeklenmesini göstermek içindir.

| Kayıt | Index ailesi | Fresh open | Search RSS | Sparse checkpoint heap |
|---:|---:|---:|---:|---:|
| 1M | ~121 MB | ~0.86 ms* | ~1.23 MiB | ~82 KB |
| 5M | ~605 MB | ~3.09 ms | ~1.64 MB | ~410 KB |
| 10M | ~1.21 GB | ~5.87 ms | ~2.07 MB | ~820 KB |

`*` Açılış mikro-benchmarkı sandbox scheduling/cache durumuna göre sub-ms ile birkaç ms arasında oynayabiliyor.

## Güvenlik yaklaşımı

- System/user-data/unknown öğeler cleanup işlem katmanında tekrar korunur.
- Cache/temp gibi yeniden üretilebilir öğeler quarantine adayı olabilir.
- Quarantine aynı volume üzerinde rename kullanır; gereksiz disk kopyası yapmaz.
- Kalıcı purge ayrı komuttur.
- Web resolver tam kullanıcı path'ini veya dosya içeriğini dışarı göndermez.

## Windows saha doğrulaması

Portable pakette tanılama araçları da bulunur:

```powershell
.\low-end-benchmark.ps1 -Index C:\ProgramData\SearchTool\index -IdleSeconds 60
.\windows-soak.ps1 -Drive C: -Index C:\ProgramData\SearchTool\index\C.stidx -DurationMinutes 30 -CrashRestartService
.\foreground-impact.ps1 -Drive C: -Index C:\ProgramData\SearchTool\index -Enforce
.\defender-check.ps1 -Path . -CustomScan -Enforce
.\web-resolver-validation.ps1 -Enforce
.\smartscreen-validation.ps1 -Artifact .\search-tool-gui.exe -RequireEnabled -RequireMotw
.\display-validation.ps1 -Mode Exercise -RequireMixedDpi -Enforce
.\pristine-validation.ps1 -Package .\SearchTool-Windows-x64.zip
.\physical-validation.ps1 -Drive C: -Index C:\ProgramData\SearchTool\index -SoakMinutes 30 -EnforceTargets
```

USN journal reset testi güvenlik nedeniyle yalnız kendi geçici NTFS VHD'sinde çalışır:

```powershell
.\journal-reset-recovery.ps1
```


Kaynak checkout üzerinde tam Windows release adayı kapısı:

```powershell
.\scripts\windows-release-gate.ps1 -SoakMinutes 5
```

Bu kapı build/test, izole NTFS/USN runtime, journal-reset recovery, portable paket SHA-256 doğrulaması, temiz install/uninstall smoke ve Defender kontrolünü tek raporda toplar.
## Search Tool görünüm kimliği ve Windows uyumluluğu

Search Tool artık Windows Search'i görsel olarak taklit eden düz bir panel değildir. Varsayılan `Signature` görünümü koyu lacivert yüzey, camgöbeği vurgu, kart tipi sonuç satırları, dosya/klasör badge'leri ve `LOCAL • INSTANT • PRIVATE` ürün kimliği kullanır. `Görünüm` menüsünden Signature, Midnight, Graphite, Frost ve Windows Native presetleri; system/dark/light tema; vurgu/arka plan/kart/yazı/ikincil yazı renkleri; compact/comfortable/spacious sonuç yoğunluğu; compact/standard/wide panel boyutu; saydamlık; arka plan resmi, fit/fill/stretch ve resim opacity ayarlanabilir. Ayarlar `%APPDATA%\SearchTool\ui.conf` içinde kalıcıdır.

Windows uyumluluğu bilinçli olarak iki katmanlıdır. Windows 11'de desteklenen build'lerde rounded frame ve Mica/system backdrop kullanılır. Windows 10 build 10240-19045 ailesinde Win11-only DWM attribute'ları çağrılmaz; DPI API'leri dinamik yüklenir ve eski build'lerde system-DPI fallback kullanılır. Win10'da Segoe UI, Win11'de Segoe UI Variable tercih edilir. Böylece aynı binary modern Win11 görünümü sunarken eski Windows 10 sürümlerinde güvenli native fallback ile açılmaya devam eder.
