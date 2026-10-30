# Kendi sunucunuzda dengeX Remote

Bu kılavuz 0.2.0 beta'nın onaylı ekran izleme API'sini kurar. PostgreSQL/Redis/Keycloak gerekmez. Linux sunucu, Docker Engine + Compose v2, Python 3.9+, public IPv4 ve bir DNS adı kullanın. Sunucu yeniden başlatıldığında bellek içindeki davetler ve oturumlar sona erer.

## HTTPS ve isteğe bağlı TURN

1. DNS'te `destek.example.com` A kaydını sunucunun IPv4 adresine yönlendirin. Çalışan IPv6'nız yoksa AAAA kaydı eklemeyin. Cloudflare DNS kullanıyorsanız ilk kurulumda DNS only kullanın.
2. Güvenlik duvarında TCP 80/443 açın. TURN kullanacaksanız UDP 3478 ve UDP 49160–49200 açın. Sunucunun Internet'e çıkışı olmalı.
3. Kaynağı alın ve kendi değerlerinizle yapılandırın:

```sh
git clone https://github.com/dengexco/dengex-desk.git
cd dengex-desk
python3 scripts/setup-server.py --domain destek.example.com --turn-ip PUBLIC_IPV4
docker compose --env-file infra/self-host/.env -f infra/self-host/compose.yaml --profile turn up -d --build --wait
curl --fail https://destek.example.com/readyz
python3 scripts/test-server.py https://destek.example.com
```

`PUBLIC_IPV4` bir yer tutucudur. TURN istemiyorsanız `--turn-ip ...` ve `--profile turn` seçeneklerini kaldırın. Caddy alan adınız için sertifika alır ve yeniler; gerçek DNS ve erişilebilir portlar gerekir. `/readyz` içinde `ready:true`, `guestHosts:true` ve TURN etkinse `turn:true` görünmelidir. Bu kontrol TURN'e UDP erişimini veya ekran yakalamayı kanıtlamaz.

İki istemcide sunucu adresi olarak `https://destek.example.com` girin. Native istemci yalnızca geçerli sertifikalı HTTPS kök adresini kabul eder; parola, yol veya sorgu parametresi içeren adresleri reddeder.

## Cloudflare Tunnel seçeneği

Zaten Cloudflare Tunnel kullanıyorsanız HTTPS servisi yerine yalnızca API'yi başlatabilirsiniz:

```sh
docker compose --env-file infra/self-host/.env -f infra/self-host/compose.yaml up -d --build --wait api
cloudflared tunnel --url http://127.0.0.1:8080
```

Bu komutun ürettiği `https://...trycloudflare.com` adresini iki uygulamaya girin. Quick Tunnel geçicidir; süreç kapanınca adres kaybolur. Kalıcı kullanımda kendi Cloudflare hesabınızda named tunnel + alan adı kurup origin'i `http://127.0.0.1:8080` olarak yapılandırın. Tünel token'ını istemciye veya depoya koymayın.

**Cloudflare HTTP tüneli TURN veya görüntü aktarım sunucusu değildir.** Görüntü doğrudan WebRTC veya ayrıca erişilebilir TURN/UDP üzerinden gider. Mac'te Docker Desktop altında TURN profilinin Linux host networking ayarını üretim yapılandırması olarak kullanmayın; TURN için Linux public sunucu kullanın. Quick Tunnel denemeleri doğrudan bağlantı kurulabilen ağlarla sınırlı olabilir.

## Yapılandırma ve güvenlik sınırları

Kurulum `infra/self-host/.env` ve `infra/self-host/secrets/` üretir. Bunlar git tarafından yok sayılır. Sırları yazdırmaz ve mevcut yapılandırmanın üzerine yazmaz. Sır dosyaları container UID'sinin okuyabilmesi için salt okunur, ana dizin yalnızca dosya sahibine açık oluşturulur. Bunları kaynak kodla veya istemci paketiyle dağıtmayın.

| Ayar | Anlamı |
|---|---|
| `DX_PILOT_GUEST_HOSTS=1` | Adresi bilen kullanıcıların kendi onaylı odalarını oluşturmasına izin verir. Varsayılan kurulum böyle çalışır. |
| `DX_PILOT_TOKEN_FILE` | Yönetici odası oluşturma/olay okuma için 32 rastgele baytın base64url token dosyası. Normal istemciler almaz. |
| `DX_TURN_URL` | `turn:host:port?transport=udp`; TCP/TLS adresleri başlangıçta reddedilir. |
| `DX_TURN_SECRET_FILE` | Coturn ile aynı ortak HMAC sırrı. En az 32 karakter. |
| `DX_RELAY_ONLY=1` | İki uçta yalnızca TURN adayları kullanılır; doğrudan bağlantıya geri dönüş yasaktır. TURN yoksa API başlamaz. |

Misafir oluşturma açık olan Internet sunucuları kota tüketimine açıktır. Bu küçük ekip betasında global davet/kimlik doğrulama sınırları ve en fazla 8 oda vardır; tenant, kullanıcı hesabı, MFA veya kalıcı cihaz kaydı yoktur. Genel halka açık büyük hizmet olarak sunmayın.

TURN HMAC parolaları ancak paylaşan tarafın onayından sonra, rol ve oda başına verilir. Bitişleri oda sonundan bir dakika sonradır (en fazla yaklaşık 31 dakika). Oturumu bitirmek ekran yetkisini iptal eder; daha önce verilmiş TURN parolasını anında iptal etmez. Coturn kullanıcı/toplam kota, bant genişliği sınırı ve özel/loopback IP yasakları uygular. TURN operatörü uçların IP'lerini ve trafik hacmini görebilir; medya DTLS-SRTP ile şifrelenir.

## Doğrulama

- `/readyz`: yapılandırma ve API erişimi.
- `scripts/test-server.py`: tek kullanımlık davet, onay, ICE bilgisi erişimi ve sonlandırma; ekran görüntüsü oluşturmaz.
- İki gerçek cihazda davet/onay sonrası görüntü, 30 saniye akış ve durdurma kontrolü yapın.
- TURN kabul testi için `.env` içinde `DX_RELAY_ONLY=1` yapıp API'yi yeniden oluşturun. Farklı ağlardaki iki cihazdan gerçek görüntü doğrulayın. Ardından isterseniz `0` yapın. UDP engelli ağlar bu sürümde desteklenmez.

## Güncelleme, kapatma ve yedekleme

```sh
git pull --ff-only
docker compose --env-file infra/self-host/.env -f infra/self-host/compose.yaml --profile turn up -d --build --wait
# Kapatma (kalıcı HTTPS sertifikalarını silmez):
docker compose --env-file infra/self-host/.env -f infra/self-host/compose.yaml --profile turn down
```

Güncelleme aktif oturumları sonlandırabilir. `.env`, `secrets/` ve Caddy `caddy_data` volume'unu erişimi kısıtlı yerde yedekleyin. Ekran kaydı veya kalıcı oturum veritabanı yoktur. Token sızarsa ilgili sırları değiştirip API/Coturn'u yeniden başlatın; tüm açık oturumlar sonlanır.

İnceleme için `docker compose ... ps` ve API servis loglarını kullanın. Reverse proxy access logları varsayılan kapalıdır. Authorization, davet, SDP, istek gövdesi veya ekran içeriği kaydetmeyin.

Referanslar: [Caddy automatic HTTPS](https://caddyserver.com/docs/automatic-https), [coturn yapılandırması ve REST kimlik bilgileri](https://github.com/coturn/coturn/blob/master/README.turnserver), [Cloudflare Quick Tunnels](https://developers.cloudflare.com/cloudflare-one/connections/connect-networks/do-more-with-tunnels/trycloudflare/).
