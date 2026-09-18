# Gerçek cihaz kabul protokolü

Hiçbir satır doldurulmadan geçti sayılmaz. Ham raporlar, OS/build, CPU/GPU, ekran,
codec, ağ rotası ve kullanılan sürüm/hash birlikte tutulmalı.

| Senaryo | Gerekli ortam | Bu teslimattaki durum |
|---|---|---|
| Mac → Mac gerçek uzak ekran/input | İki Mac, kimlik + signaling | Henüz uygulanmadı |
| Win → Mac | Win11 x64 + Mac | Henüz uygulanmadı |
| Mac → Win | Mac + Win11 x64 | Henüz uygulanmadı |
| Win → Win | İki Win11 x64 | Henüz uygulanmadı |
| Intel Mac gerçek çalışma | Intel Mac macOS14+ | Yalnızca helper çapraz derleme |
| macOS14 alt sınırı | Gerçek macOS14 sistemi | Deployment target; çalışma testi yok |
| Farklı ev ağı/hotspot/UDP kapalı | İki uç + erişilebilir TURN | Yapılmadı |
| Zorunlu TURN/TLS | Gerçek TLS sertifikası/ayrı listener | Yapılmadı |
| Basılı key/button kopmada bırakılması | Gerçek remote input dispatcher | Eksik |
| Erişilebilirlik/Ekran Kaydı iptali | TCC iznini kullanıcı değiştirir | İlk izin sorgusu doğrulandı, oturum içi iptal bekliyor |
| UAC/secure desktop/FileVault/kilit ekranı | Yetkili native kurulum ve test | Destekli ilan edilmiyor |
| TR Q/TR F/EN fiziksel key mapping | Her iki OS/layout | Sadece kendi Mac penceresinde Unicode metin deneyi |
| Retina/negatif koordinat/rotation/hotplug | Çoklu monitör | Bekliyor; Windows rotation açık hata verir |
| Büyük dosya/symlink/reparse/disk dolu | Gerçek transfer engine | Engine yok; filename parser testleri ayrı |
| 60 dakika oturum | İki gerçek uç, ölçüm harness'i | Yapılmadı |
| 100 cihaz/10 aktif oturum | Tam kontrol düzlemi + TURN | Yapılmadı |
| p95 first frame <=5s | Tekrarlanan gerçek destek oturumu | Ölçülmedi |
| p95 input-to-photon <=200ms | Kamera/uygun zaman ölçümü | Ölçülmedi |
| İmzalı güncelleme/değiştirilmiş paket | Gerçek updater/signing keys | Updater yok |

## Tekrar üretilebilir mevcut deneyler

```bash
source scripts/env.sh
bash scripts/build-macos.sh
cargo build --locked -p dengex-transport
# Ekran yakalamaz:
target/debug/dx-probe transport --report .artifacts/transport.json
target/debug/dx-probe native --helper .artifacts/native/dx-macos-probe --synthetic --seconds 8 --report .artifacts/synthetic.json
# Görünür native pencerede yerel Başlat düğmesini bekler:
target/debug/dx-probe native --helper .artifacts/native/dx-macos-probe --seconds 15 --report .artifacts/screen.json
# CLI'dan açıkça başlatılan yerel capture deneyi:
target/debug/dx-probe native --helper .artifacts/native/dx-macos-probe --seconds 15 --start-local-test --report .artifacts/screen-cli.json
# Sadece kendi penceresine unicode/key ve mouse click gönderir:
.artifacts/native/dx-macos-probe --input-test --report .artifacts/input.json
# Yalnızca localhost'a yayımlanan geçici, kimlik doğrulamalı coturn fixture:
python3 scripts/test-local-turn.py
# Aynı yerel TURN üzerinden gerçek ekran deneyi:
python3 scripts/test-local-turn.py --video
```

Bir `.app` ile terminalden başlatılan helper'ın TCC izin bağlamları farklıdır. Bu
çalışmada terminal bağlamı izinliydi; paket ilk açılışında izinler gerekli olarak
göründü. Uygulamanın kendi capture/input testi için sistem ayarlarından izin verip
yeniden açın. İzin verilmediyse başarısız/bloke sonuç doğrudur, örnek video açılmaz.

Başlatmadan önce hassas pencereleri kapatmak kullanıcının tercihidir. Standart
test raporu ekran/tuş/pano/file içeriği kaydetmez. Görüntü yalnızca native test
penceresinde sunulur; örnek ekran çıktısı rapora gömülmez.

## Aşama 0'dan çıkış kapısı

Windows capture→Media Foundation encode/decode→WebRTC hattını native ortamda tamamla;
Mac ve Windows'ta aynı protokol/sürümle gerçek iki cihazlı peer akışını kur; cihaz
anahtarı/fingerprint bağlamasını ve görünür kabul/kesme akışını bağla. Dört yön,
zorunlu relay, uzak input ve native render performansı raporlanmadan Aşama 0
'geçti' işaretlenmez. Ayrıntılı yönetim paneli bu kapıdan sonra geliştirilir.
