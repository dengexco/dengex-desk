# Gerçek test raporu — 17 Eylül 2026

**Durum: çalışan native laboratuvar teslimatı; Aşama 0'ın dört platform/gerçek uzak bağlantı kabulü tamamlanmadı. Üretime hazır değildir.**

Ortam: Apple M4 / Mac16,10 / 16 GiB RAM; macOS 26.6.2 (25G83), arm64. Swift 6.3.1 (Swift 5 dil modu); Rust 1.98.1; Go 1.27.1; Node 25.9.0; Docker 29.5.3. Minimum macOS deployment target 14.0; macOS 14 çalıştırma kanıtı yok.

## Gerçekten çalıştırılan kontroller

| Kontrol | Sonuç | Kanıt / kapsam |
|---|---|---|
| Rust güvenlik ve property testleri | **13 geçti** | [Ham çıktı](evidence/rust-tests.txt): imza, tenant, replay, izin kesişimi, OS izni, lease, eski input epoch, filename/parser |
| Go domain race testleri | **Geçti** | [Ham çıktı](evidence/go-tests.txt): 6 birim test + fuzz seed; 100 eşzamanlı token tüketiminden biri başarılı; kabul/iptal yarışı |
| Rust clippy `-D warnings` | **Geçti** | [Ham çıktı](evidence/clippy.txt) |
| React/TypeScript/Vite production build | **Geçti** | `.artifacts/frontend-build.log`, son paketleme de build'i tekrar çalıştırdı |
| macOS arm64 helper ve Tauri .app | **Derlendi, açıldı** | Native WKWebView açık/koyu tema ve Tab odağı incelendi; son ad-hoc/hardened runtime paketinde [transport](evidence/packaged-transport.json) ve [codec](evidence/packaged-synthetic.json) geçti |
| Native ekran → H.264 → WebRTC → native decode | **Geçti** | [JSON](evidence/native-screen.json): güncel 0.20.5, 1920×1080, 271 encode / 269 decode, yaklaşık 10.47s, hardware encoder=true |
| Sentetik codec/WebRTC | **Geçti** | [JSON](evidence/synthetic-codec.json): 320×180, 90 encode / 89 decode. Ekran testi değildir |
| Yerel WebRTC veri kanalı | **Geçti** | [JSON](evidence/transport.json): 0.20.5 ile handshake+echo 242ms. İki uç aynı süreçte |
| Native Türkçe metin + mouse click | **Geçti** | [JSON](evidence/native-input.json): CoreGraphics/NSEvent üzerinden yalnızca kendi test penceresine. Uzak input veya TR Q/F fiziksel mapping testi değildir |
| Zorunlu coturn UDP | **Geçti** | [JSON](evidence/turn-udp.json): kısa ömürlü HMAC TURN credential; relay-only, yerel Docker fixture |
| Gerçek 1080p ekranın zorunlu TURN/UDP'den geçmesi | **Geçti, kalite kabulü değil** | [JSON](evidence/turn-native-screen.json): 226 encode / 195 decode; 8.56s; aynı Mac/Docker, WAN değil |
| TURN/TCP | **Desteklenmiyor** | [JSON](evidence/turn-tcp-unsupported.json): açık hata; doğrudan bağlantıya düşmez |
| TURN/TLS | **Desteklenmiyor / gerçek TLS testi yok** | 0.20.5 adapter kaynağı secure/non-UDP TURN atlıyor; CLI açık hata verir |
| Windows x64 adapter | **Çapraz derleyici kontrolü geçti** | `.artifacts/windows-cross-check.log`; DXGI/SendInput gerçek kod, Windows'ta çalıştırılmadı |
| Intel macOS helper | **x86_64 derlendi** | `.artifacts/native/dx-macos-probe-x86_64`; Intel makinede çalıştırılmadı; Intel tam Tauri paketi yok |
| PostgreSQL migration | **Geçti** | Lab PostgreSQL 17 üzerinde 18 tablo |
| Gerçek DB tenant RLS, scoped FK, audit immutability | **Geçti** | [SQL test çıktısı](evidence/postgres-security.txt); uygulama rolünü taklit eden NOBYPASSRLS test rolü, fixture transaction rollback |
| DB transaction bağlamı temizliği | **Geçti** | Aynı bağlantıda SET LOCAL sonrası transaction bitişinde boş bağlam. Gerçek pgx pool henüz bağlı değil |
| Yedek/restore | **Geçti** | [Restore çıktısı](evidence/postgres-restore.txt): boş ürün şeması ayrı test DB'ye, 18 tablo. Müşteri verisi/DR kabulü değil |
| API sağlık/ürün hazır değil davranışı | **Geçti** | [HTTP çıktısı](evidence/api-smoke.json): health 200; ready ve `/v1/sessions` 503 |
| npm audit | **0 vulnerability** | `.artifacts/npm-audit.json` |
| RustSec taraması | **0 vulnerability, 7 uyarı** | [Özet](evidence/dependency-audit-summary.json): altı bakım uyarısı, bir glib unsound uyarısı. Yayın incelemesi açık |
| SBOM | **Üretildi** | `.artifacts/sbom-rust.json`, `.artifacts/sbom-npm.json`; [Rust lisans dökümü](../dependency-licenses.md) |
| Ad-hoc paket imzası | **Yerel bütünlük doğrulandı** | [codesign](evidence/codesign-verify.txt); Developer ID, Team ID, notarization yok |
| CI / gitleaks | **Workflow yazıldı; uzakta çalıştırılmadı** | `.github/workflows/verify.yml`; CI geçmiştir iddiası yok |

## Sonuçların sınırları

Kare sayıları ve elapsed süre bir p95 fps/latency benchmark değildir. ScreenCaptureKit durağan görüntüde daha az frame üretebilir. Nominal RTP zamanlama 30 fps'dir; gerçek capture PTS üzerinden pace/adaptation tamamlanmadı. Encode/decode sayıları farklıdır; son sample buffer'ı ve paket/kare kaybı etkileri olabilir. Relay denemesinde 31 karelik fark var; PLI/keyframe ve backpressure iyileştirmesi, ağ kaybı ve uzun oturum testi gerekir. Testin geçtiği iddiası gerçek karelerin şifreli hattan çözülmesiyle sınırlıdır.

İlk 1080p deneyinde 85 kare kodlandı, sıfır kare çözüldü: [başarısız kanıt](evidence/initial-native-failure.json). RTP yeniden birleştirme toleransı 32'den 1024 pakete çıkarıldı ve test tekrarlandı. Başarısız test saklandı. İlk TURN fixture da Docker relay adreslemesi nedeniyle başarısız oldu; loopback'e bağlı, kimlik doğrulamalı fixture düzeltildi. Video fixture'ında kullanıcı/kota sınırları ayrı deneyler için düzeltildi. Bunlar production TURN/tenant quota testi değildir.

İlk webrtc-rs 0.17.2 deneyi güncel 0.20.5 ile değiştirildi. Yukarıdaki medya/transport/relay JSON'larında motor sürümü bulunur; eski motorun sonuçları yeni sürüme mal edilmez. 0.20.5 de TURN/TCP/TLS sağlamadığı için ürün motoru kabulü verilmedi. [ADR](../adr/0001-phase-zero-engine.md).

Terminal bağlamında capture/Accessibility izinleri açıktı. Yeni `.app` ilk açılışında bu izinler yoktu ve UI doğru biçimde izin gerektiğini gösterdi. Paket içinde izin gerektirmeyen codec/transport yolu doğrulandı; paket adına kullanıcı izin onayı verilmedi.

Windows cihazı, ikinci Mac ve farklı ağ ortamına erişim sağlanmadı. Developer ID/Windows imza sertifikası ve üretim DNS/TLS/IdP bilgileri verilmedi. Dört yön, gerçek uzak mouse/keyboard, 60 dakika stabilite, 100 cihaz/10 oturum, ilk kare/etkileşim p95, OS14/Intel çalışma, kurulum/güncelleme/kaldırma matrisi doğrulanmış değildir.

## Tamamlanmayan ürün işlevleri

Gerçek kullanıcı hesabı/PKCE/MFA, sunucu cihaz kaydı/Windows anahtar kasası, WSS signaling ve iki cihazlı oturum; grant/consent/lease'in canlı capture/input dispatcher'a bağlanması; Next.js paneli; katılımsız erişim/Argon2id/parola rotasyonu; dosya/pano/sohbet; tam monitör/key mapping; imzalı güncelleme; production Compose/Portainer/TLS ve kapasite testi. Bunlar sahte başarılı sonuçlarla doldurulmadı.

## 18 Eylül: ekran izni ve cihaz kimliği düzeltmesi

- Paket içindeki ekran testinde izin eksikliği yeniden üretildi. Önceki akış bekleme/hata yolunda ayrıntıyı kaybediyor, UI izin yenilemesi genel hatayı siliyordu. Yeni akış izin eksikliğini hemen `blocked` olarak gösterir; neden raporda ve kartta kalır. [İzin engeli kanıtı](evidence/packaged-screen-permission-blocked.json).
- “Ekran iznini aç” doğru macOS Sistem Ayarları sayfasını açtı. Kullanıcı izin değişikliği sonrasında uygulama yeniden açıldı; TCC veritabanına müdahale edilmedi. İzin sonrası bir kez boş WebView görüldü, tam kapatıp yeniden açınca düzeldi; otomatik OS yeniden başlatma davranışı için uzun süreli regresyon testi henüz yok.
- Güncel `.app` içinden “Ekranımı 15 sn test et” çalıştırıldı ve UI **Geçti** gösterdi: **1920×1080, 425 decoded frame, handshake+echo 480 ms**. [Paketlenmiş gerçek ekran raporu](evidence/packaged-screen-permission-fixed.json). Bu yine aynı bilgisayardaki iki WebRTC ucudur; uzak bağlantı veya performans kabulü değildir.
- Yeni yerel cihaz kimliği Anahtar Zinciri helper’ından UI’a geldi; uygulama kapatılıp yeniden açıldığında aynı `[cihaz kimliği çıkarıldı]` kimliği korundu. Özel anahtar UI/loga çıkarılmadı. Sunucu enrollment ve geçici destek kodu halen bağlı değil.
- İki Rust masaüstü regresyon testi geçti: eksik/bilinmeyen izinler yalnızca ilgili testi engeller; başarısız alt süreç raporunun özel nedeni korunur ve sahte passed sonucu reddedilir. React/TypeScript/Vite build ve masaüstü/transport clippy `-D warnings` geçti.
- macOS sistem Bash sürümünde boş dizi + nounset nedeniyle `package-macos.sh --debug` hatası da giderildi. Yeni app/ZIP oluşturuldu; derin/strict codesign doğrulaması paket betiğinde geçti. Developer ID ve notarization halen yok.

Tam ürün için kalan işler: [öncelikli liste](../remaining-work.tr.md).

## 18 Eylül: Cloudflare Tunnel ve Windows pilot paketi

- Cloudflare `2026.9.1` Quick Tunnel, yalnızca projenin `127.0.0.1:18433` pilot API'sine bağlandı. Mevcut tüneller/DNS kayıtları değiştirilmedi. Canlı URL `scripts/pilot-server.py status` ile okunur.
- Public HTTPS üzerinden sağlık, yetkisiz oda oluşturmanın reddi, davetin tek tüketimi, viewer'ın onay verememesi, host onayı ve sonlandırma doğrulandı: [API kanıtı](evidence/cloudflare-api.json).
- Go race testleri geçti: atomik davet yarışında 20 istekten biri; onay öncesi SDP reddi, rol ayrımı, grant Ed25519 imzası, iptal, expiry, Origin reddi ve heartbeat bitişi. Masaüstü testleri 3 geçti; son desktop/transport clippy `-D warnings` geçti. UI TypeScript/Vite build geçti.
- Tauri Windows x64 portable `.exe` üretildi. PE GUI x86-64 doğrulandı; CRT statik, import listesinde VCRUNTIME140/140_1 yok. Microsoft SDK debug sembollerinin bulunamamasına ilişkin LNK4099 uyarısı executable üretimini engellemedi.
- Windows ZIP public Cloudflare adresinden yeniden indirildi; SHA-256 yerel paketle aynı, ZIP bütünlüğü geçti. Pakette yalnızca `.exe`, public sunucu adresi ve kullanım kılavuzu var; token/davet/admin anahtarı gömülmedi. [Paket kanıtı](evidence/windows-pilot-package.json).
- Güncel Mac `.app` açıldı; yeni Cihaza bağlan ekranı derlendi. Yeni ad-hoc derleme için macOS ekran izni tekrar gerekli. **Kullanıcı Windows paketini hazırlamamızı, Mac izin adımını daha sonra yapacağını söyledi.** İzin etkinleştirilmedi; güncel paketle gerçek uzak görüntü testi tamamlandı denmiyor.
- Önceki paket içindeki 1080p ekran testinin geçmesi yeni Cloudflare/Windows oturumunun çalıştığı anlamına gelmez. Windows'ta gerçek uygulama çalıştırması, iki fiziksel cihaz görüntüsü, farklı ağ ve TURN doğrulaması halen bekleniyor.
- Bu pilot yalnızca davetli Windows → Mac ekran izleme için hazırlanmıştır. Uzak input, Windows host, tenant/OIDC/MFA, kalıcı cihaz kaydı ve TURN bağlı değildir. Mac uykuya girer veya sunucu/tünel kapanırsa hizmet erişilemez.

Kurulum ve durdurma: [Cloudflare pilot kılavuzu](../cloudflare-pilot.tr.md).

## 18 Eylül: Pilot 2 — Windows ekranına Mac'ten bağlanma

Önceki Pilot 1 paketi yalnızca Windows alıcı / Mac gönderici yönündeydi.
Pilot 2, Windows'un davet oluşturmasını ve kendi ekranını paylaşmasını ekler.
Mac alıcı bu akışta ekran kaydı izni istemez.

- Windows DXGI yakalama, en fazla 1280×720 BGRA→NV12 dönüşümü ve Windows inbox
  Media Foundation H.264 encoder native WebRTC göndericiye bağlandı. Encoder
  Baseline, düşük gecikme ve periyodik keyframe kullanır; Annex B parametre
  setleri sample'a eklenir. İlk monitör; döndürme/imleç birleştirme/RDP/secure
  desktop kabulü yoktur. 20 fps hedefi performans ölçümü değildir.
- Windows hedefi için desktop/platform/transport çapraz derleyici ve Clippy
  `-D warnings` kontrolleri geçti. Mac'te core, desktop ve piksel dönüşüm
  testleri ile desktop/transport Clippy geçti; frontend paketlemede doğrulandı.
- GitHub Windows runner üzerinde `cargo test -p dengex-platform-windows`
  başarılı: sentetik NV12 karelerden H.264 SPS/PPS/IDR çıktısı üretildi.
  Bu test gerçek Windows masaüstünü yakalamaz. [CI çalışması](https://github.com/dengexco/dengex-desk/actions/runs/35335757694).
- Sunucuda opt-in guest host akışı, altı davet/dakika ve sekiz açık oda sınırı;
  host/viewer onay ayrımı ve mevcut odaların izolasyonu Go race testleriyle geçti.
  Yönetim anahtarı istemciye dağıtılmaz. Mevcut admin oluşturma yetkisi korunur.
- Canlı HTTPS üzerinden guest davet, guest'in yönetim erişiminin reddi, viewer'ın
  host adına onay verememesi ve iptal doğrulandı. [Kanıt](evidence/cloudflare-api.json).
- Windows yakalama işçisi native iptal ve monoton yetki süresini denetler.
  Uygulama kapanışı stop işaretini verir; bounded kanal ve drop guard işçiyi
  kapatır. Kareler React/JSON/base64 üzerinden geçirilmez.

Gerçek hedef Windows'ta açılış, ekran yakalama ve Mac'e canlı görüntü aktarımı
henüz kabul edilmedi. İmzalı kurulum, uzak input ve TURN hâlâ bu pilotun dışındadır.
