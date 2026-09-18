# Cloudflare Tunnel ve Windows görüntüleme pilotu

Pilot 2, Windows veya Mac ekranını karşı bilgisayardan izlemek içindir. Bağlanılacak bilgisayar davet oluşturur ve paylaşımı ayrıca kabul eder. Uzak klavye/fare kontrolü, gerçek teknisyen hesabı/MFA, müşteri tenant kaydı, katılımsız erişim ve TURN bu pilotta yoktur. Tam ürünün tamamlandığı anlamına gelmez.

## Çalıştırma

Sunucu ve tunnel bu Mac'te çalışır; Mac'in uyanık ve internete bağlı olması gerekir. Mevcut dengeX servislerinin tünel/DNS yapılandırmaları değiştirilmez.

Temiz klonda Go, Rust, Node ve Python araçlarını PATH üzerinde hazırlayın.
`cloudflared` ikilisini Cloudflare'ın resmi dağıtımından edinip
`.tools/cloudflared/cloudflared` yoluna koyun; bu araç depoya dahil değildir.
`scripts/env.sh`, yalnızca ilk geliştirme ortamındaki `.tools` araçlarını
kullanan çalışma alanları içindir; genel kurulum betiği değildir.

```bash
mkdir -p .artifacts/pilot
(cd services/api && go build -o ../../.artifacts/pilot/dx-pilot-api ./cmd/api)
python3 scripts/pilot-server.py start
python3 scripts/pilot-server.py status
python3 scripts/pilot-server.py stop
```

`start` benzersiz Quick Tunnel adresini verir. Adres uygulamanın `resources/pilot-server.json` dosyasına ve bu Mac'in özel pilot yapılandırmasına yazılır. Yeni adresten sonra paketleri yeniden oluşturun veya Windows ekranındaki sunucu adresini elle güncelleyin. Kullanıcıya özel credential veya davet ZIP'e gömülmez.

```bash
bash scripts/package-macos.sh --debug
bash scripts/package-windows-cross.sh
```

Windows çapraz derleme için proje içindeki cargo-xwin, LLVM/Windows SDK araçları ve Rust x86_64-pc-windows-msvc target'ı gerekir. Windows SDK lisansı kurulumda kabul edilir. Linux/macOS çapraz derleme çıktısı Windows'ta çalıştırma kanıtı değildir. Kalıcı Windows paketleme için gerçek Windows CI runner'ı tercih edin.

Web adresi Windows paketinin indirme sayfasını sunar. `/download/windows` yalnızca hazırlanan ZIP dosyasını döndürür; repo, loglar, sırlar ve diğer çalışma alanı dosyaları sunulmaz. ZIP, imzasız geliştirme `.exe` ve public sunucu adresi içerir. CRT statik bağlanır; WebView2 çalışma zamanı gerekir. Windows kullanım adımları: [kılavuz](windows-pilot.tr.txt).

## Oturum akışı

1. Bağlanılacak Windows'ta **Cihaza bağlan → Bu bilgisayarın ekranını paylaş → Davet oluştur**.
2. Mac'te aynı sunucu adresi, cihaz adı ve Windows'taki daveti girip istek gönderin.
3. Windows'ta cihaz adını ve yalnızca izleme iznini kontrol edip kabul edin.
4. Windows: DXGI → NV12 → Media Foundation H.264 → native WebRTC. İlk monitör, en fazla 1280×720; performans kabulü henüz yok. Alıcının WebView WebRTC yüzeyi görüntüyü çözer; kareler React state/JSON/base64 üzerinden aktarılmaz.
5. **Denemeyi bitir** veya uygulamayı kapatma paylaşımı durdurur. Paylaşan uygulamanın başlığı ve oturum ekranı paylaşım durumunu gösterir.

Ters yönde Mac davet oluşturur ve kendi ekranı için macOS izni gerekir. Mac yalnızca Windows ekranını izliyorsa ekran kaydı izni gerekmez. Mac gönderici ScreenCaptureKit/VideoToolbox ve görünür native Durdur penceresini kullanır. Windows'ta döndürülmüş monitör, imleç birleştirme, secure desktop/RDP ve ekran değişimi sonrası otomatik toparlanma henüz yoktur.

Mac helper, parent pipe kapanışında kendini sonlandırır. Windows native capture/encoder işçisi, yerel iptal ve monoton grant süresini her capture döngüsünde denetler; görüntü kuyruğu sınırlıdır. Sunucu yetkisi yenilenemezse native gönderici en fazla grant süresinde durur. Renderer sayacına güvenilmez.

## Pilot güven sınırı

- Yönetim anahtarı Mac Application Support ve `.artifacts/pilot/create-token` içinde yalnızca kullanıcı tarafından okunur (0600). Dağıtım paketinde veya URL'de bulunmaz.
- Pilot 2: `/pilot/create-guest`, yalnızca `DX_PILOT_GUEST_HOSTS=1` açıldığında hesap olmadan cihazın kendi odasını oluşturur. `pilot-server.py start` bunu pilot için açar; varsayılan API yapılandırmasında kapalıdır. Genel kabul bütçesi 6 oluşturma/dakika, toplam 8 açık odadır. İstemcinin gönderdiği IP header’larına güvenilmez. Bu bir hesap/MFA doğrulaması veya üretim DoS koruması değildir. `/pilot/create` yönetim anahtarını gerektirmeye devam eder.
- Davet 256 bit rastgele, tek kullanımlık, oda ile sınırlı ve 30 dakika geçerlidir. Aynı anda katılma yarışında yalnızca biri kazanır. Sunucuda erişim token'larının yalnızca SHA-256 özeti tutulur.
- Host ve viewer farklı token'lar kullanır. Alıcı host adına onay veremez. Tek alıcı, yalnızca view_screen. Genel input/file/shell komutu yoktur.
- API, TLS sertifikasını doğrulayan native istemciden kullanılır; yönlendirmeler takip edilmez. Token Authorization header veya JSON body'dedir, sorgu parametrelerinde değildir. Browser Origin taşıyan doğrudan API istekleri reddedilir.
- SDP fingerprint'leri sunucunun Ed25519 imzalı 45 saniyelik grant'ine; room, pilot kapsamı ve view_screen yetkisine bağlanır. Native uç, beklenen iki fingerprint'i ve imzayı mevcut SessionGate ile denetler. Yerel lease en fazla 40 saniye ve grant bitişinden kısadır.
- Sunucu bu pilotun güvenilen otoritesidir. Üretim tenant/OIDC/cihaz enrollment otoritesi yerine geçmez. Davetli cihaz adı doğrulanmış hesap adı değildir ve UI'da böyle belirtilir.
- Token/oda bilgileri bellektedir; API yeniden başlayınca bütün odalar geçersizleşir. En fazla 8 açık oda; süresi bitenler silinir, ended odalar yeni davet oluşturulurken temizlenir. Aktif olmayan uç 60 saniyede ended olur; grant yenilemesi durur.

## Ağ ve sınırlar

Cloudflare Quick Tunnel yalnızca HTTPS kontrol/eşleştirme ve paket indirmeyi taşır. Bu pilot REST polling kullanır; websocket veya SSE'ye bağlı değildir. WebRTC ekran trafiği DTLS/SRTP ile doğrudan uçlar arasındadır. Cloudflare public STUN adresi ICE keşfi için kullanılır. TURN yapılandırılmadı: simetrik NAT, UDP engeli veya kurumsal ağda görüntü bağlantısı kurulamayabilir. Tünelin açık olması medya yolunun çalıştığını kanıtlamaz.

Kalıcı kullanım için Cloudflare hesabında ayrı bir named tunnel ve alan adı; farklı ağlarda güvenilir medya için ayrı erişilebilir TURN servisi gerekir. Quick Tunnel adresi yeniden başlatmada değişir ve test amaçlıdır. Bu makinenin uykusu/yeniden başlaması hizmeti kapatır; otomatik boot servisi kurulmadı.

Kaynaklar: [Cloudflare Quick Tunnels](https://developers.cloudflare.com/cloudflare-one/networks/connectors/cloudflare-tunnel/do-more-with-tunnels/trycloudflare/), [public hostname protokolleri](https://developers.cloudflare.com/cloudflare-one/networks/connectors/cloudflare-tunnel/routing-to-tunnel/protocols/), [Cloudflare STUN/TURN](https://developers.cloudflare.com/realtime/turn/), [Tauri Windows derleme](https://v2.tauri.app/distribute/windows-installer/).
