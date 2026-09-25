# Pilot 3 — Uygulama güvenliği ve Windows DLL paketi

Bu katman dengeX Remote bağlantılarını ve paket yüklemeyi korur; bilgisayar
genelinde antivirüs, EDR veya güvenlik duvarı hizmeti değildir.

## Bağlantı korumaları

- Davet 256 bit rastgele, tek kullanımlık ve **5 dakika** geçerli. Oturumun
  toplam üst sınırı oluşturulduğu andan itibaren 30 dakika.
- Ekran paylaşımı yalnızca ekranı paylaşan kişinin açık onayıyla başlar.
  HTTPS, DTLS/SRTP, Ed25519 imzalı kısa süreli grant ve native süre denetimi korunur.
- Sunucu genelinde dakikada 30 katılma denemesi ve 20 başarısız kimlik
  doğrulama denemesi sınırı vardır. Geçerli token ile heartbeat/sonlandırma bu
  bütçeyi tüketmez. Yeni misafir odası sınırı 6/dakika; açık oda sınırı 8.
- Limitler proxy başlıklarına göre ayrılmaz. `X-Forwarded-For` değiştirerek
  atlanamaz; fakat genel bütçeyi tüketen biri yeni katılımları geçici olarak
  engelleyebilir. Üretim için proxy güveni, hesap bazlı limitler ve DDoS
  koruması ayrıca gerekir.
- En fazla 96 KiB istek gövdesi, aynı anda 16 gövde okuma, API sürecinde 10 sn
  okuma süresi vardır. Gövde okuma ve ağ yanıtı oda kilidini tutmaz.
- Authorization yalnızca tek, doğru biçimde `Bearer` başlığıyla kabul edilir.
- Oturum ekranındaki **Oturum güvenliği**, davet/onay/başlama/bitme olaylarını
  gösterir. Oda başına son 16 olay; yönetici için `GET /pilot/security` ile
  son 128 olay sunucu belleğinde tutulur. Bu uç sadece yönetici token'ını kabul eder.
  Olaylar yalnızca zaman ve sabit olay türüdür; ad, IP, token, SDP ve görüntü içermez.
  Yeniden başlatmada silinir; kalıcı veya değiştirilemez denetim günlüğü değildir.

## Windows dağıtımı

```text
dengeX Remote/
  dengeX Remote.exe       # küçük, bütünlük kontrolü yapan başlatıcı
  dengex_desktop.dll      # Tauri arayüzü, Rust oturumu, WebRTC ve Windows ekran motoru
  resources/pilot-server.json
  BENI-OKU.txt
  LICENSE.txt
```

**EXE ve DLL aynı sürümden ve aynı klasörde olmalıdır.** ZIP'in tamamını çıkartın.
DLL çift tıklanarak çalıştırılmaz. Sunucu tarafındaki Go API ayrı süreçtir;
macOS `.app` biçimini korur. Rust bileşenleri aynı güvenli oturum yaşam döngüsünü
paylaşan tek uygulama DLL'inde bağlanır; bağımsız eklenti/SDK sözleşmesi verilmez.

Başlatıcı derlenirken DLL SHA-256 özeti EXE'ye gömülür. Başlatıcı yalnızca kendi
klasöründeki DLL'i doğrular; farklı, eksik, kesilmiş veya reparse-point dosyayı
reddeder. Dosya doğrulamadan süreç kapanana kadar yazma/silme paylaşımı kapalı
açılır. DLL tam yoluyla yüklenir, bağımlılık araması System32 ile sınırlandırılır;
çalışma klasörü ve PATH taranmaz. `DxAppAbiVersion` (1) ve `DxAppRun` Win64 ABI
girişleri kontrol edilir. Uygulama ana iş parçacığında, DllMain dışında başlatılır.

Bu, bozulmuş veya değiştirilmiş DLL'in mevcut başlatıcıyla açılmasını engeller.
**Authenticode imzası değildir**; EXE ve DLL'i birlikte değiştirebilen yerel
saldırgana veya yöneticiye karşı güven sınırı oluşturmaz. Paket hâlâ imzasız
geliştirme paketidir. Üretim dağıtımı için imzalı EXE/DLL/kurucu gerekir.

## Kaynaktan derleme ve doğrulama

Windows x64, MSVC Developer PowerShell, Node, Python ve Rust gerekir:

```powershell
npm ci
npm run build
# pilot-server.json yoksa örnek yapılandırmayı kopyalayın.
$env:RUSTFLAGS = '-C target-feature=+crt-static'
cargo build --locked -p dengex-desktop --lib --features tauri/custom-protocol
python scripts/build-windows-launcher.py target/debug/dengex_desktop.dll target/debug/dengeX-Remote.exe
python scripts/test-windows-launcher.py
```

`dengeX-Remote.exe --verify-package` yalnızca dosya bütünlüğünü denetler.
`--load-check` ayrıca gerçek DLL yüklemesini ve ABI sürümünü denetler; pencere,
capture veya bağlantı başlatmaz. Başarı 0; eksik DLL 12, bozulmuş DLL 13,
yüklenemeyen DLL 14, ABI uyuşmazlığı 15 çıkış kodudur.

GitHub Actions, Windows'ta değişmiş/eksik DLL, farklı çalışma klasörü, Türkçe
dosya yolu ve gerçek uygulama DLL'i yükleme kontrollerini çalıştırır. Bu kontroller
iki fiziksel cihazla ekran aktarımı kabul testinin yerine geçmez.

Kaynaklar: [Rust cdylib](https://doc.rust-lang.org/reference/linkage.html),
[Microsoft DLL yükleme](https://learn.microsoft.com/en-us/windows/win32/api/libloaderapi/nf-libloaderapi-loadlibraryexw),
[DLL arama sınırı](https://learn.microsoft.com/en-us/windows/win32/api/libloaderapi/nf-libloaderapi-setdefaultdlldirectories).
