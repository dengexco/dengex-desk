# 0.2.0 beta doğrulama kapsamı

Bu dosya mevcut test kapsamını açıklar. Geçmiş pilot sonuçları `results.md` ve `evidence/` altında kendi tarihleriyle saklanır; yeni sürüm için tekrar çalıştırılmış sayılmaz.

## Otomatik yayın kapıları

- Rust yetki/imza, sona erme, tekrar kullanım ve protokol testleri.
- Go race detector ile API: tek kullanımlık davet, açık host onayı, yetki kapsamı, rate limit, yavaş istemci, olay gizliliği.
- TURN ayarlarının reddedilmesi, onay öncesi kimlik bilgisi verilmemesi, rol/oda bazlı HMAC ve oturum sonunda yeni kimlik bilgisi kesilmesi.
- Gerçek coturn fixture üzerinde relay zorunlu WebRTC data-channel trafiği; masaüstünün kullandığı Host::prepare ile sadece relay adaylarının toplanması. TCP'nin desteklenmediği durumda doğrudan bağlantıya düşmeden hata vermesi.
- Docker içindeki gerçek API için readiness, davet, tekrar kullanım, onay, ICE erişimi ve stop HTTP smoke testi.
- Windows'ta gerçek DLL'nin yüklenmesi, ABI, eksik/değişmiş DLL reddi ve Common Controls manifest regresyonu. Windows Media Foundation encoder testi; ekran yakalama testi değildir.
- Frontend derlemesi ve bağımlılık taraması. macOS yardımcılarının iki mimari için derlenmesi, macOS/Windows paketlerinin üretilmesi.

Sonuçlar [Verify](https://github.com/dengexco/dengex-desk/actions/workflows/verify.yml) ve [Release](https://github.com/dengexco/dengex-desk/actions/workflows/release.yml) çalıştırmalarında commit kimliğiyle bulunur. Yayın iş akışı tüm kapılar geçmeden paketleri yayımlamaz.

## Fiziksel cihaz kabulü

Bu sürüm için Windows↔Mac ve diğer iki cihaz çiftlerinde gerçek kullanıcı ekranı kabulü otomatik CI kapsamında değildir. Her ortamda aşağıdakiler ayrıca doğrulanmalıdır:

1. İki cihazda aynı HTTPS adresi; sunucu kontrolü başarılı.
2. Davet öncesi ve yerel onay öncesi görüntü gönderilmemesi.
3. Onaydan sonra gerçek ekranın en az 30 saniye görüntülenmesi.
4. Paylaşımı bitir, pencereyi kapat, ağ/sunucu kaybı ve kilitleme durumlarında yakalamanın durması.
5. Farklı ağlarda `DX_RELAY_ONLY=1` ile gerçek video; UDP kapalı ağda bağlantının başarısız olduğunun açık gösterilmesi.
6. macOS izin reddi/yeniden açma ve Windows WebView2/Media Feature Pack eksikliği için anlaşılır hata.

CI'nin yeşil olması donanım/izin/NAT uyumluluğunun tamamını veya üretim güvenlik denetimini kanıtlamaz.
