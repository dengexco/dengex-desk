# dengeX Remote

Windows ve macOS için Tauri, Rust, Swift ve Go ile geliştirilen uzaktan destek projesi. [MIT lisanslıdır](LICENSE).

**Geliştirme aşamasındadır; üretime hazır değildir.** Native laboratuvar araçları ve davet/onay ile Windows'tan Mac ekranını izlemeyi hedefleyen pilot akış mevcuttur. Windows'ta çalıştırma ve iki fiziksel cihaz arasında canlı görüntü kabulü henüz tamamlanmadı.

Gerçek macOS ekran yakalama, VideoToolbox H.264 encode/decode, yerel WebRTC taşıma, native görüntü sunumu, Türkçe metin/fare deneyi ve native güvenlik çekirdeği geliştirildi. Sahte uzaktan bağlantı, örnek ekran videosu veya uydurma cihaz listesi yoktur. Ürün rotaları entegrasyon tamamlanana kadar 503 döner.

- [Gerçek doğrulama sonuçları ve sınırları](docs/testing/results.md)
- [Mimari ve mevcut modüller](docs/architecture.md)
- [Motor/codec kararı](docs/adr/0001-phase-zero-engine.md)
- [Tehdit modeli](docs/threat-model.md)
- [Kurulum ve işletim](docs/operations.md)
- [Gerçek cihaz kabul listesi](docs/testing/manual-matrix.md)
- [Ürün gereksinimleri](docs/requirements.tr.txt)
- [Katkıda bulunma](CONTRIBUTING.md)

Bu depo kaynak kodu ve belgeleri içerir. Derleme çıktıları, çalışma zamanı anahtarları ve canlı pilot sunucu adresi Git'e eklenmez. Paketler yerelde `.artifacts/releases/` altında üretilir.

## Cloudflare ve Windows görüntüleme pilotu

18 Eylül güncellemesi: Windows x64 için portable Tauri uygulaması, Mac ekranını izlemek üzere davet/onay akışı ve ayrı HTTPS pilot API eklendi. **Windows → Mac yalnızca ekran izleme**; Windows ekranını paylaşma ve uzak klavye/fare henüz yok. Windows paketi derlendi; gerçek Windows cihazı üzerinde çalışma kabulü bekleniyor.

- [Pilot kurulum ve güven sınırları](docs/cloudflare-pilot.tr.md)
- [Windows kullanım adımları](docs/windows-pilot.tr.txt)
- Geçerli adres: `python3 scripts/pilot-server.py status`. Cloudflare Quick Tunnel test içindir; bu Mac uyanık ve süreçler açık kalmalıdır.
- Mac: **Cihaza bağlan → Davet oluştur**. Windows: sunucu adresi/davet ile istek. Mac: gelen isteği kabul.
- Public HTTPS üzerinden davet, tekrar kullanımın reddi, host onayı ve sonlandırma kontrolleri [geçti](docs/testing/evidence/cloudflare-api.json). Bu sonuç Windows runtime veya iki fiziksel cihaz kabulü değildir.

## Paketi açma

ZIP'i açıp `dengeX Remote Lab.app` uygulamasını çalıştırın. Son kullanıcıda Node.js, Rust veya Go gerekmez. Paket **ad-hoc geliştirme imzalıdır; Developer ID/notarization yoktur**, production dağıtımı değildir. Sistem güvenlik uyarısını otomatik atlayan kurulum betiği kullanılmaz.

Ana ekran destek servisine henüz bağlı değildir. **Teknik doğrulama** bölümündeki testler gerçekten çalışır:

1. WebRTC bağlantısı: iki yerel uç arasında şifreli veri kanalı.
2. Video codec: açıkça sentetik karelerden H.264/WebRTC/native decode.
3. Gerçek ekran: “Ekranımı 15 sn test et” düğmesiyle başlayan görünür native deney; Durdur düğmesi. İzin eksikse hemen nedeni ve “Ekran iznini aç” eylemi gösterilir.
4. Yerel giriş: sadece test uygulamasının kendi penceresinde Türkçe metin ve fare tıklaması.

Ana ekranda gösterilen kalıcı cihaz kimliği, rastgele UUID ve Ed25519 anahtarıyla oluşturulur. Özel anahtar macOS Anahtar Zinciri’nde tutulur; arayüze yalnızca public key ve kimlik bilgisi çıkar. Kimlik henüz sunucuya kaydedilmediği için uzaktan bağlanılabilir bir adres değildir. Geçici kod, destek servisi bağlanana kadar kullanılamaz. Anahtar kasası hatasında kimlik sessizce yenilenmez.

Ekran/giriş için uygulamadaki **İzinler** bölümünden ilgili sistem ayarını açın, **dengeX Remote Lab** iznini etkinleştirin. macOS isterse uygulamayı kapatıp yeniden açın. İzin sorguları arka planda yürür ve bir testin hata mesajını silmez. Terminalden çalışan helper ile `.app` izin bağlamı aynı değildir. Katılımsız erişim, otomatik başlangıç, sistem servisi ve kayıt açılmaz.

## Kaynaktan geliştirme

Geliştirici gereksinimleri: Node 24+, Rust 1.98.1, Go 1.27.1, Python 3; macOS native için Command Line Tools/Swift, Windows için Microsoft C++ Build Tools ve WebView2 Runtime. Araçları PATH üzerinde hazırlayın. `.tools` klasörü kişisel geliştirme ortamıdır ve depoda dağıtılmaz; `scripts/env.sh` yalnızca bu klasördeki araçları kullanan mevcut çalışma alanları içindir.

```bash
git clone https://github.com/dengexco/dengex-desk.git
cd dengex-desk
npm ci
npm run build
```

macOS masaüstü geliştirme:

```bash
bash scripts/build-macos.sh
cargo build --locked -p dengex-transport
mkdir -p apps/desktop/src-tauri/resources
cp target/debug/dx-probe apps/desktop/src-tauri/resources/dx-probe
cp .artifacts/native/dx-macos-probe apps/desktop/src-tauri/resources/dx-macos-probe
cp .artifacts/native/dx-device-identity apps/desktop/src-tauri/resources/dx-device-identity
# Yalnızca henüz yerel sunucu yapılandırması yoksa:
test -f apps/desktop/src-tauri/resources/pilot-server.json || cp apps/desktop/src-tauri/resources/pilot-server.example.json apps/desktop/src-tauri/resources/pilot-server.json
npm run desktop -- dev
```

Yalnızca UI geliştirme görünümü: `npm run dev` → `http://127.0.0.1:1420`. Tarayıcı native testleri simüle etmez; test düğmeleri devre dışıdır.

```bash
bash scripts/check.sh
bash scripts/test-postgres.sh         # önce operations.md içindeki lab DB kurulumu
python3 scripts/test-local-turn.py    # Docker gerekli, geçici localhost fixture
bash scripts/package-macos.sh        # release-mode development .app; production imzası değil
```

Go süreç denemesi: `cd services/api && DX_LISTEN=127.0.0.1:18432 go run ./cmd/api`. Çalışma zamanı hazır olmayan ürün işlevlerini başarılı gibi döndürmez.

Windows üzerinde kaynak derlemesi için Node/Rust/MSVC araçları hazırken önce `apps/desktop/src-tauri/resources/pilot-server.example.json` dosyasını aynı klasörde `pilot-server.json` adıyla kopyalayın; var olan yerel yapılandırmayı değiştirmeyin. Ardından `npm ci`, `npm run build` ve `cargo build --locked --target x86_64-pc-windows-msvc -p dengex-desktop --features tauri/custom-protocol` çalıştırın. `rustup target add x86_64-pc-windows-msvc` ile hedef kurulabilir. Sunucu adresi boşken uygulama açılır; pilot ekranında kendi HTTPS sunucunuzun adresi girilir. `scripts/package-windows-cross.sh`, ilk geliştirme ortamındaki yerel cargo-xwin/LLVM/SDK araçlarına bağlıdır; temiz klon için otomatik araç kurucusu değildir.

Cloudflare pilotunu çalıştırmak için [ayrı kurulum belgesini](docs/cloudflare-pilot.tr.md) izleyin. Canlı test adresi ve oturum davetleri bu depodan dağıtılmaz.

## Sıradaki kabul kapısı

Windows encode/decode/input ve gerçek iki uçlu signaling/cihaz kimliği entegrasyonu; dört platform eşleşmesi; farklı ağ ve TURN/TLS; consent/revoke/lease'in canlı medya ve input'a bağlanması. Bunlar bitmeden Aşama 0 ve Aşama 1 tamamlandı sayılmaz. Ayrıntılı Next.js paneli, OIDC/MFA/PKCE, managed agent, cihaz parolası, dosya/pano/sohbet, imzalı güncelleme ve production işletim sonraki aşamalardır.

## Lisans

Projenin kendi kaynak kodu [MIT](LICENSE) lisansı altındadır. Üçüncü taraf bağımlılıklar kendi lisanslarına tabidir; [Rust bağımlılık dökümü](docs/dependency-licenses.md) başlangıç envanteridir.
