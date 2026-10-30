# dengeX Remote

[![Verify](https://github.com/dengexco/dengex-desk/actions/workflows/verify.yml/badge.svg)](https://github.com/dengexco/dengex-desk/actions/workflows/verify.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)

Windows ve macOS için **kendi sunucunuzda çalışan, kullanıcı onaylı ekran paylaşımı**. MIT lisanslıdır. Görüntü WebRTC ile şifrelenir; her bağlantı için ekranı paylaşan kişinin açık onayı gerekir.

**0.2.0 beta:** gerçek ekran yakalama ve izleme vardır. Uzaktan klavye/fare kontrolü, dosya aktarımı, ses, katılımsız erişim, kalıcı cihaz numarası ve hesap/MFA sistemi henüz yoktur. Ücretsiz ortak bağlantı sunucusu sağlanmaz. Bu sürüm küçük ekiplerin kontrollü denemelerine yöneliktir; tam AnyDesk alternatifi olduğu iddia edilmez.

## İndir ve bağlan

[Windows ve macOS paketleri → Releases](https://github.com/dengexco/dengex-desk/releases)

| Bilgisayar | İndirilecek dosya |
|---|---|
| Windows 11 x64 | `dengeX-Remote-Windows-x64.zip` |
| macOS 14+ Apple Silicon | `dengeX-Remote-macOS-arm64.zip` |
| macOS 14+ Intel | `dengeX-Remote-macOS-x86_64.zip` |

1. Önce [sunucuyu kurun](docs/self-hosting.md) veya yöneticinizden mevcut sunucunun HTTPS adresini alın.
2. ZIP'i tamamen çıkarın. Windows'ta **dengeX Remote.exe** dosyasını DLL ve `resources` klasörüyle birlikte tutun. macOS'ta uygulamayı Applications'a taşıyın.
3. İki bilgisayarda aynı sunucu adresini girip **Sunucuyu kontrol et** düğmesine basın.
4. Ekranı paylaşacak bilgisayarda **Davet oluştur**; izleyecek bilgisayarda daveti girip bağlantı isteği gönderin.
5. Paylaşan bilgisayarda isteği kontrol edip **Ekranımı paylaşmayı kabul et** düğmesine basın. **Paylaşımı bitir** veya uygulamayı kapatma paylaşımı durdurur.

Paketler ticari kod imzalama sertifikası/noter onayı içermez. Windows WebView2 gerekir; Windows N sürümlerinde Media Feature Pack gerekebilir. macOS'ta Ekran Kaydı izni gerekir. [Ayrıntılı kurulum ve sorun giderme](docs/quickstart.md).

## Sunucu

Linux, Docker Compose, bir alan adı ve herkese açık sunucu gerekir. HTTPS için 80/443 TCP; isteğe bağlı TURN için 3478 ve 49160–49200 UDP açılır.

```sh
git clone https://github.com/dengexco/dengex-desk.git
cd dengex-desk
python3 scripts/setup-server.py --domain destek.example.com --turn-ip SUNUCUNUN_PUBLIC_IPV4_ADRESI
docker compose --env-file infra/self-host/.env -f infra/self-host/compose.yaml --profile turn up -d --build --wait
```

Örnek alan adını/IP yer tutucusunu değiştirin. `--turn-ip` kullanılmazsa `--profile turn` seçeneğini de kaldırın; bu durumda yalnızca doğrudan WebRTC bağlantısı denenir. Kurulum mevcut sırların üzerine yazmaz. [HTTPS, Cloudflare Tunnel, TURN, güncelleme ve yedekleme](docs/self-hosting.md).

## Güvenlik ve sınırlar

- Tek kullanımlık 256 bit davet, 5 dakika geçerlilik; en fazla 30 dakikalık oturum.
- Sunucuda atomik davet tüketimi, açık yerel onay, deneme/hacim sınırları ve sınırlı bellek kullanımı.
- İmzalı, DTLS kimliklerine bağlı kısa süreli görüntüleme yetkisi. Yetki yenilenmezse native yakalama durur.
- TURN parolaları onay sonrasında oturum/role göre üretilir; sunucunun ortak sırrı istemciye gönderilmez.
- Windows EXE, birlikte dağıtılan DLL'nin SHA-256 özetini yüklemeden önce doğrular. Bu, Authenticode imzasının yerine geçmez.
- Sunucu ekran görüntülerini kaydetmez. Olay kayıtları bellekte sınırlıdır; davet, token, SDP ve ekran içermez. İzleyen kişi kendi cihazında kayıt yapabilir.

Bu beta en fazla 8 eşzamanlı oda için sınırlandırılmıştır. Misafir davet oluşturma açık olduğunda sunucu adresini bilen herkes kendi odasını oluşturabilir; davet ve yerel onay olmadan mevcut ekranlara erişemez. Çok kullanıcılı kurumsal yetkilendirme yoktur. UDP engelli ağlarda TURN/TCP veya TURN/TLS desteği bulunmadığından bağlantı kurulamayabilir. Windows ana monitörü en fazla 1280×720 ile paylaşır; kilitli masaüstü, UAC güvenli masaüstü ve RDP desteklenmez.

[Security policy](SECURITY.md) · [Güvenlik/DLL ayrıntıları](docs/security-and-dll.md) · [Test kapsamı](docs/testing/beta-release.md)

## Kaynaktan derleme

Geliştirme için Node.js 24 LTS, `rust-toolchain.toml` içindeki Rust sürümü, Go 1.27.2 ve Python 3.9+ gerekir. macOS'ta Xcode Command Line Tools; Windows'ta Visual Studio 2022 C++ Build Tools ve Windows SDK gerekir. Son kullanıcı bunları kurmaz.

```sh
npm ci
npm run build
cargo test --locked -p dengex-agent-core -p dengex-transport --lib
# services/api dizininde:
go test -race ./...
```

macOS paketi: `bash scripts/package-macos.sh`.
Windows paketi: **x64 Native Tools PowerShell** içinde `./scripts/package-windows.ps1`.
Çıktılar `.artifacts/releases/` altındadır. Yerel Tauri geliştirme: `npm run desktop -- dev` (macOS paylaşımı için önce `bash scripts/package-macos.sh --debug` ile native yardımcıları üretin).

[Build and publish desktop release](.github/workflows/release.yml) iş akışı testleri, Windows DLL yükleme kontrolünü ve her iki Mac mimarisinin derlemesini tamamladıktan sonra beta paketlerini ve SHA-256 özetlerini yayımlar. Yayın paketlerinde özel sunucu adresi veya yönetici sırrı bulunmaz.

## Yapı ve katkı

| Dizin | Sorumluluk |
|---|---|
| `apps/desktop` | Tauri 2 / React arayüzü, native oturum yönetimi |
| `native/agent-core` | Yetki doğrulama, yeniden kullanım koruması, protokol kuralları |
| `native/transport` | Rust WebRTC ve H.264 akışı |
| `native/platform-windows` | DXGI ve Media Foundation |
| `native/platform-macos` | ScreenCaptureKit ve VideoToolbox yardımcıları |
| `native/windows-launcher` | DLL bütünlüğünü kontrol eden EXE |
| `services/api` | Go eşleştirme, onay, süreli yetki ve TURN kimlik bilgileri |
| `infra/self-host` | Kullanılabilir HTTPS + API + isteğe bağlı TURN kurulumu |

`infra/compose.yaml`, veritabanı şemaları ve `/v1` taslakları gelecekteki kurumsal sürümün laboratuvarıdır; yukarıdaki kurulum bunlara ihtiyaç duymaz. Eski pilot belgeleri tarihsel test kayıtlarıdır.

[Katkı kılavuzu](CONTRIBUTING.md) · [Planlanan kapsam](docs/requirements.tr.txt) · [MIT lisansı](LICENSE). Üçüncü taraf bileşenlerin lisansları paketlerin `THIRD-PARTY-NOTICES.txt` dosyasında bulunur.
