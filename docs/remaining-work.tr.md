# Tam ürün için kalan işler — 18 Eylül 2026

Durum: macOS'ta çalışan native deney paketi. İki ayrı bilgisayar arasında uzaktan destek ürünü henüz tamamlanmadı. Aşama 0 kabul kapısı açık.

## 1. İlk gerçek destek oturumu — en yüksek öncelik

- Yerel cihaz anahtarını kayıt daveti, firma ve sunucu kaydıyla bağla. Destek kimliğini sunucuda ayır; klonlanan anahtar, iptal ve yeniden kayıt yaşam döngüsünü uygula.
- Go API iş rotalarını ve gerçek PostgreSQL erişimini bağla. Şema/RLS testleri var; ürün HTTP rotaları halen hazır değil (503).
- OIDC/PKCE ile teknisyen girişi ve MFA; firma üyeliği, personel ataması ve cihaz kapsamını her istekte denetle.
- Cihaz heartbeat/presence ve WSS signaling; iki ayrı süreç ve cihaz arasında kimliği doğrulanmış oturum kurulması.
- 10 dakikalık tek kullanımlık destek kodu, deneme sınırı ve atomik tüketim; teknisyen bilgisi ve istenen izinlerle gerçek müşteri kabul/ret ekranı.
- İmzalı grant, iki DTLS fingerprint'i, nonce, politika sürümü ve en fazla 60 saniyelik lease denetimlerini canlı medya/input kanallarına bağla. Çekirdek testleri uygulama entegrasyonu yerine geçmez.
- Teknisyen ekranında uzak görüntü, giriş kanalları, yalnızca izleme modu; görünür oturum çubuğu ve müşterinin anında bağlantıyı kesmesi.
- Kesinti, yeniden bağlanma, anahtarların bırakılması, yetki iptali, zaman aşımı ve temel audit olayları.

Kabul: farklı ağlardaki iki cihazda yetkili teknisyen müşteri onayıyla gerçek ekranı görür ve kontrol eder; müşteri kesince bütün kanallar durur. Henüz bu kabul yok.

## 2. Platform ve ağ motorunu tamamla

- Windows DXGI yakalama ve SendInput kodu var; gerçek Windows üzerinde encode/decode, sunum, uygulama paketi ve uzak input entegrasyonu eksik.
- macOS gerçek capture/VideoToolbox var. Giriş deneyi kendi test penceresiyle sınırlı; uzaktaki yetkili oturuma bağlı masaüstü kontrolü eksik.
- Win→Win, Win→Mac, Mac→Win, Mac→Mac cihaz testleri; Apple Silicon ve Intel gerçek çalışma doğrulaması.
- Mevcut webrtc-rs 0.20.5 adaptörü TURN/TCP ve TURN/TLS sağlamıyor. Uygun motor/uyarlama kararı, ardından gerçek UDP kapalı ağ ve 443/TLS denemesi gerekli. Yerel TURN/UDP deneyinin geçmesi bunu karşılamaz.
- Bant genişliğine göre kalite/fps, paket kaybında anahtar kare isteme, uzun oturum stabilitesi, TR Q/TR F/EN, Retina/DPI ve monitör değişimi.

## 3. Yönetim ve yönetilen cihazlar

- Next.js paneli: firmalar, şubeler/gruplar, cihazlar, kullanıcılar, teknisyen atamaları, politikalar, aktif/geçmiş oturumlar ve denetim kayıtları.
- Masaüstü teknisyen konsolu ve güvenli tek kullanımlık referansla uygulama açma akışı.
- Açık onayla yönetilen ajan kurulumu, kullanıcı oturumu/servis ayrımı, başlangıç ve kaldırma yaşam döngüsü.
- Cihaza özel Argon2id doğrulayıcılı katılımsız erişim; parola değiştirme, iptal, politika sürümü, offline bekleme ve yeniden yetkilendirme.

## 4. Oturum özellikleri

- Akış halinde iki yönlü dosya aktarımı; hedef klasör, ayrı izinler, kota, bütünlük, iptal ve güvenli dosya yolu işlemleri.
- Ayrı okuma/yazma yetkileriyle metin panosu ve oturum içi sohbet.
- Monitör seçimi, sığdır/gerçek boyut/yakınlaştır/tam ekran, oturum süresi raporları.

## 5. Dağıtım, işletim ve kabul

- Production DNS/TLS, kimlik sağlayıcısı ve TURN kurulumu; sır yönetimi, firewall, kotalar, izleme, alarm ve audit saklama işleri. Yerel Compose taslağı production kurulumu değildir.
- Windows imzalı paket; macOS Developer ID, notarization/stapling ve doğrulanmış Intel paketi. Mevcut Mac paketi ad-hoc geliştirme imzalıdır.
- İmzalı güncelleme manifesti/paketi, sürüm düşürme koruması, aşamalı dağıtım, güvenli kurtarma ve kaldırma.
- Gerçek yedek/geri yükleme, migration ve sürüm uyumluluk prosedürleri. Boş şemanın yerel geri yükleme testi yapılmıştır.
- Uçtan uca güvenlik/tenant izolasyonu, izin iptali, bozuk paket ve ağ kopması testleri; CI'ın uzakta çalıştırılması ve bağımlılık uyarılarının ele alınması.
- 1080p/30 fps hedefi; ilk kare p95 ≤5 saniye, referans ağda etkileşim p95 ≤200 ms; 60 dakika stabilite; 100 cihaz/10 eşzamanlı oturum yük denemeleri. Henüz ölçülmüş kabul sonuçları yok.

Gerekli dış girdiler: gerçek Windows test cihazı ve ikinci Mac/uygun native ortam, farklı ağ, pilot sunucu ve alan adı, üretim kimlik sağlayıcısı yapılandırması, Windows/Apple imza yetkileri. Geliştirme işleri bu girdilerin tamamını beklemeden sürdürülebilir; ilgili gerçek kabul/dağıtım bunlara bağlıdır.

Oturum kaydı, sistem sesi, tarayıcıdan tam kontrol, Wake-on-LAN, uzaktan yazdırma ve whiteboard ilk sürüm kabulüne dahil değildir; özgün gereksinimde sonraki sürümlere bırakılmıştır.
