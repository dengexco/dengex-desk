# dengeX Remote — başlama kılavuzu

Bu beta onaylı ekran izleme içindir. Klavye/fare, dosya, ses ve katılımsız erişim içermez.

## Windows

`dengeX-Remote-Windows-x64.zip` dosyasını tamamen çıkarın. `dengeX Remote.exe`, `dengex_desktop.dll` ve `resources` aynı klasörde kalmalıdır. EXE'yi tek başına taşımayın. Windows 11 x64 ve güncel Microsoft Edge WebView2 Runtime gerekir. Windows N kullanıyorsanız Media Feature Pack'i kurun.

Paket Authenticode ile imzalanmamıştır; Windows yayımlayıcıyı doğrulayamaz. Güvenlik yazılımını kapatmayın. Kaynağı doğrulayamadığınız dosyayı çalıştırmayın; kuruluşunuz izin vermiyorsa yöneticinizden yardım alın. ZIP özetini PowerShell `Get-FileHash .\dengeX-Remote-Windows-x64.zip -Algorithm SHA256` ile sürümün SHA256SUMS.txt dosyasına karşı kontrol edebilirsiniz.

## macOS

İşlemcinize uygun ZIP'i çıkarıp `dengeX Remote Lab.app` uygulamasını Applications'a taşıyın. Uygulama ad-hoc imzalıdır; Apple Developer ID/noter onayı yoktur. macOS açmayı engellerse yalnızca kaynağını doğruladığınız paket için Sistem Ayarları → Gizlilik ve Güvenlik ekranındaki uygulamaya özel açma seçeneğini kullanın. Sistem korumalarını genel olarak kapatmayın.

Ekranı paylaşacak Mac'te İzinler → Ekran iznini aç yolunu izleyin. Sistem Ayarları → Gizlilik ve Güvenlik → Ekran ve Sistem Sesi Kaydı altında dengeX Remote Lab iznini açın. macOS isterse uygulamayı kapatıp açın. İzleyen Mac için ekran kaydı izni gerekmez. Erişilebilirlik izni yalnızca geliştirici bölümündeki yerel giriş testi içindir.

## Bağlantı

1. İki bilgisayarda aynı `https://...` dengeX sunucu adresini girin. Ortak bir sunucu pakete dahil değildir. Adres kontrolünün başarılı olması ekranın bağlandığı anlamına gelmez.
2. Bağlanılacak bilgisayarda Davet oluştur'a basın. 5 dakika geçerli, tek kullanımlık daveti yalnızca güvendiğiniz kişiye iletin.
3. İzleyen bilgisayarda daveti ve görünen adı girin. Paylaşan bilgisayarda gelen isteği kabul edin. Görünen ad hesap doğrulaması değildir.
4. İlk görüntüyü bekleyin. Gerekirse Görüntüyü aç'a basın. Paylaşımı bitir düğmesi veya uygulamayı kapatma paylaşımı sonlandırır.

Sunucu adresi bu cihazda hatırlanır; davet ve oturum token'ları kalıcı olarak kaydedilmez. Önceki pilot paketlerinin geçici Cloudflare adresleri geçersiz olabilir; yeni sunucu adresinizi girin.

## Sorun giderme

- **Sunucu hazır değil:** sunucuda `docker compose --env-file infra/self-host/.env -f infra/self-host/compose.yaml ps` ve `/readyz` yanıtını kontrol edin. HTTPS sertifikası geçerli olmalı; uygulama HTTP'yi veya sertifika doğrulamasını atlamayı desteklemez.
- **Davet geçersiz:** davet bir kez kullanılabilir; iki cihazda aynı sunucuyu kullanıp yeni davet oluşturun.
- **Davet oluşturma kapalı:** yöneticiniz `DX_PILOT_GUEST_HOSTS=1` ayarını kontrol etsin.
- **Görüntü yok:** paylaşan cihazın ekran iznini ve yerel oturumunun açık olduğunu kontrol edin. TURN kullanılıyorsa 3478 ve 49160–49200 UDP açık olmalı. Cloudflare Tunnel görüntüyü taşımaz. UDP tamamen engelleniyorsa bu sürüm o ağı desteklemez.
- **DLL eksik/değişmiş:** aynı sürüm ZIP'ini tekrar çıkarın. Eski EXE ile yeni DLL'yi karıştırmayın. Hash kontrolü devre dışı bırakılamaz.
- **Windows ekranı kapandı:** kilit, çözünürlük/ekran değişikliği veya güvenli masaüstü yakalamayı durdurabilir. Normal masaüstüne dönüp yeniden davet oluşturun.

Kaynak ve hata bildirimleri: https://github.com/dengexco/dengex-desk
Sunucu kurulumu: https://github.com/dengexco/dengex-desk/blob/main/docs/self-hosting.md
Hata bildirirken davet, token, ekran görüntüsü veya özel sunucu sırlarını paylaşmayın.
