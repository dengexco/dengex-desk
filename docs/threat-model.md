# Güven sınırları

Bu dosya bir tehdit modeli ve entegrasyon kapısıdır; bitmiş güvenlik denetimi değildir.

| Sınır / tehdit | Mevcut denetim | Üretim öncesi açık iş |
|---|---|---|
| Renderer → native | Sabit, kapalı test komutu listesi; shell/fs eklentisi yok; yerel origin CSP; native ekran onayı | Gerçek grant'i capture/input dispatcher'a bağla, XSS/IPC fuzz |
| Başka tenant cihaz ID'si | Rust imzalı grant scope + Go assignment kesişimi testleri; SQL bileşik FK/RLS taslağı | Gerçek OIDC + query/WS authorization + RLS havuz temizliği entegrasyon testi |
| Grant değişikliği / replay | Ed25519 verify_strict, <=60s, tenant/device/user/session/policy/iki fingerprint, bounded replay cache | Güvenilen authority provisioning/rotation; kalıcı replay veya cihaz boot nonce |
| Signaling ele geçirilmesi | Şu an dış signaling yok, SDP aynı süreçte | Kayıtlı cihaz anahtarına bağlı endpoint imzası; merkezi imzalama otoritesi ihlali artık riskini açıkla |
| TURN yöneticisi | Standart DTLS/SRTP; relay-only config fallback yapmaz | Gerçek relay testi, kısa ömürlü credential mint, tenant/account quotas, abuse/egress ölçümü |
| Reddedilmiş/iptal edilen oturum | Native gate fail-closed, Go consent/cancel yarış testi | Network lease watchdog tüm medya/input/clipboard/file kanallarını durdurmalı |
| Yetkisiz OS giriş | Quick-support elevasyon yok; macOS OS izin sorgusu; Windows UIPI aşılmaz | Basılı tuş/buton takibi ve disconnect release; fiziksel key mapping; OS izin iptali testi |
| Dosya kaçışı | Filename parser traversal/ADS/device adlarını reddeder | Açılmış directory handle'a göre no-follow/reparse-safe akış; tüm transfer henüz yok |
| Parola/anahtar sızıntısı | Ürün parolası saklanmaz, log'a içerik yazılmaz, lab sırları 0600/ignore | Keychain/DPAPI, cihaz yerel Argon2id, cihaz anahtarı enrollment, PKCE/MFA |
| Audit değişikliği | İçerik içermeyen schema + append-only trigger taslağı | Ayrı DB roller, harici yedek, checkpoint, retention/deletion restore denemesi |
| Güncelleme ele geçirilmesi | Otomatik güncelleme yok | İmzalı manifest/paket, anti-rollback, OS imza/noter, anahtar rotasyonu |

Yerel laboratuvar CLI'si kullanıcı tarafından çalıştırılan bir geliştirme aracıdır. `--start-local-test` yalnızca bu bilgisayarda en fazla 60 saniyelik görünür capture testidir; ağ veya renderer'dan bu seçenek alınmaz. Helper IPC'si ebeveynden miras kalan pipe'larla sınırlıdır. Makinede aynı kullanıcı yetkisindeki zararlı bir süreçten mutlak izolasyon iddia edilmez.

UI'da işletim sistemi izninin açık olması, bir teknisyene cihaz yetkisi vermez. Cihaz kaydı/kalıcı ajan/katılımsız erişim birbirinden ayrılacak; mevcut paketin hiçbiri otomatik açılmaz. Standart geliştirici teknisyenine gizli parola sıfırlama yolu tasarlanmamıştır.
