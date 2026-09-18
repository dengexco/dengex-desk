# Laboratuvar işletimi ve sonraki yayın kapıları

Bu dosya production kurulumu yapmaz; alan adı, mevcut servis ve firewall değiştirilmedi.

## Yerel bağımlılıklar

```bash
python3 scripts/init-lab-secrets.py
docker compose -f infra/compose.yaml up -d --wait postgres redis
# İlk boş laboratuvar veritabanında yalnızca bir kez:
docker compose -f infra/compose.yaml exec -T postgres psql -v ON_ERROR_STOP=1 -U dengex_migrator -d dengex < services/api/migrations/001_initial.sql
bash scripts/test-postgres.sh
```

Sırlar rastgele üretilir, var olanlar üzerine yazılmaz, 0600 dosyalarda tutulur ve git'e dahil edilmez. PostgreSQL/Redis host portu yayımlanmaz. API geliştirme servisi 127.0.0.1:8080'e bağlanır; port zaten kullanılıyorsa mevcut servisi durdurmayın, `DX_LISTEN=127.0.0.1:18432` seçin. `/healthz=200` yalnızca süreç canlılığıdır. `/readyz` ve `/v1/*` henüz 503 döner.

`identity` profili, loopback'e bağlı Keycloak `start-dev` deneyidir. PKCE public client, realm, zorunlu MFA, kullanıcı/tenant bağlama ve token iptali henüz yapılandırılmamıştır. Bu profili production identity kurulumu diye kullanmayın. Yönetici parolası oluşturulan `infra/.env` dosyasındadır; hiçbir varsayılan ortak parola yoktur.

`relay` profili ayrı bir sunucuya taşınabilecek coturn taslağıdır. Açmadan önce kendi realm/external-ip/firewall ve kısa ömürlü credential servisinizi tamamlayın. Paylaşılan secret'ı istemciye dağıtmayın. Bu profil varsayılan çalıştırmada açılmaz. `scripts/test-local-turn.py` bunun yerine yalnızca 127.0.0.1'e yayımlanan, rastgele sırlı ve test bitince kapanan ayrı bir fixture oluşturur; yalnızca bu fixture loopback peer'a izin verir. Bu test WAN veya TLS testi değildir.

## Portlar

| Port | Kaynak / amaç | Durum |
|---|---|---|
| 127.0.0.1:1420 TCP | Vite geliştirici UI | Sadece geliştirme |
| 127.0.0.1:8080 TCP | Sağlık API'si | Yerel / ürün API'si hazır değil |
| 127.0.0.1:8180 TCP | Keycloak dev | İsteğe bağlı identity profili |
| Docker içi 5432 TCP | PostgreSQL | Host'a yayımlanmaz |
| Docker içi 6379 TCP | Redis | Host'a yayımlanmaz |
| 3478 UDP/TCP | TURN istemci bağlantısı | Relay profili açıkça etkinleştirilirse |
| 49160–49200 UDP | TURN relay allocation | Relay profili; NAT public mapping gerekli |
| Ayrı IP:443 TCP veya 5349 TCP | TURN TLS | Henüz yapılandırılmadı/test edilmedi |
| HTTPS/WSS 443 TCP | Panel/API/IdP | Production reverse proxy henüz yok |

Normal HTTPS ile TURN TLS'yi aynı IP:443'e gelişigüzel koymayın. Standart Cloudflare public-hostname Tunnel genel TURN/UDP yerine geçmez. TCP istemci bağlantısı relay'nin UDP port gereksinimini ortadan kaldırmaz. Kurumsal proxy uyumluluğu ayrıca ölçülecek.

## Yedek ve geri yükleme

```bash
docker compose -f infra/compose.yaml exec -T postgres pg_dump -U dengex_migrator -d dengex -Fc > .artifacts/lab-backup.dump
docker compose -f infra/compose.yaml exec -T postgres createdb -U dengex_migrator dengex_restore_test
docker compose -f infra/compose.yaml exec -T postgres pg_restore -U dengex_migrator -d dengex_restore_test --exit-on-error < .artifacts/lab-backup.dump
```

Bu çalışmada boş ürün şeması ayrı test veritabanına geri yüklendi; gerçek müşteri verisi ve üretim felaket kurtarma testi değildir. Production yedekler şifreli, ayrı erişim yetkili ve tenant saklama/silme takvimine bağlı olmalı. Audit retention varsayımı 90 gündür; hukuki uyumluluk iddiası değildir.

## Migration ve erişim

Migration rolü ile runtime rolü ayrı olacak. Runtime rolü tablo sahibi/superuser/BYPASSRLS olmayacak. Her yetkili sorgu `BEGIN → set_config(..., true) → sorgular → COMMIT/ROLLBACK` içinde çalışacak. RLS testi transaction bağlamını doğrular; gerçek pgx pool entegrasyonu hâlâ bekler. `002_atomic_enrollment.sql` bir transaction sözleşmesi örneğidir, yeni tablo migration'ı değildir. Üretim migration runner/checksum ve geri alma politikası sonraki aşamada yazılacak.

Portainer için yerel Compose servis tanımları başlangıç girdisidir; dosya bind mount'larının Portainer host'unda bulunması ve secret yönetimi gerekir. Panel, IdP production kurulumu, TLS ve imzalı ajan hazır olmadan Stack'i müşteri ortamına yayınlamayın.

## İmzalama, güncelleme ve kaldırma

Mevcut `.app` geliştirme paketi; Developer ID yok, notarization/stapling yok. Kullanıcı tarafında Node/Rust/Go gerekmez. İndirilen paketi açma konusunda macOS güvenlik kontrollerini otomatik atlayan komut kullanılmaz.

İleride yayın kapısı: her executable/helper'ı Developer ID ile imzala → hardened runtime → notarize → staple → temiz Mac'te kurulum/kaldırma. Windows için Authenticode + MSI/NSIS temiz makine testi. İmzalama tek başına SmartScreen'in kalkmasını garanti etmez.

Şu anda otomatik güncelleme/servis/helper kurulumu yok. İmzalı manifest ve paket doğrulaması, minimum security version/anti-rollback, kanallar/rollout/bakım penceresi ve aktif oturumda erteleme Stage 3 işidir. Lab uygulaması kalıcı erişim kaydı veya gizli başlangıç girdisi oluşturmaz. Kaldırmak için `.app` kaldırılabilir; raporlar `~/Library/Application Support/com.dengex.remote.lab/reports` altında yalnızca test metadatasıdır.

Olay müdahale tasarımı: tenant/device grant üretimini durdur → çevrimiçi revoke gönder → en geç lease sonunda kanalları kapat → authority/device key rotasyonu → audit ve yedek bütünlüğünü incele. Bu workflow henüz sunucuya bağlı değildir; offline cihazın anında iptal edildiği iddia edilmez.
