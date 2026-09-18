# ADR 0001 — Ölçülebilir native deney, üretim motoru kararı beklemede

Durum: Aşama 0 için kabul, ürün mimarisi için geçici. Tarih: 2026-09-17.

Mac'te ScreenCaptureKit + VideoToolbox H.264 constrained-baseline + AppKit render seçildi. Mevcut Apple Silicon cihazında donanım kodlama ve gerçek ekranın yerel WebRTC üzerinden çözülmesi doğrulandı. Windows tarafında DXGI CPU readback spike vardır; Media Foundation encode/decode, cursor compositing, ekran dönüşü ve access-loss recovery eksiktir. Bu nedenle H.264 iki platform için nihai seçilmiş codec değildir.

Transport spike, tam sürüm olarak kilitlenmiş **webrtc-rs 0.20.5 + rtc 0.20.5** kullanır. İlk 0.17.2 denemesinde TURN/TCP yolunun uygulanmadığı görüldüğü için güncel Sans-I/O sürümüne geçildi; native ekran, codec ve relay-UDP deneyleri tekrarlandı. Güncel sürümün `turn_relayer.rs` kodu da secure/non-UDP TURN URL'lerini atlıyor. Bu nedenle TCP/TLS'de bağlantı zaman aşımı beklemek yerine açık `TURN_TCP_TLS_UNSUPPORTED` hatası verilir; zorunlu relay modunda doğrudan bağlantıya düşülmez.

ICE-TCP, TURN/TCP istemci taşımasıyla aynı yetenek değildir. Bu repo TURN/TCP veya TURN/TLS destekli olduğunu iddia etmez. Aşama 0 sonrası ürün motoru kararı için libwebrtc tercihli alternatif olarak seçilmiştir; tam codec/FFI/build/lisans ve dört platform denemesi gerekir. Mevcut 0.20.5 hattı ölçüm aracı olarak korunur; üretim motoru kabulü verilmez.

Alternatifler: libwebrtc (tam codec/transport ve geniş platform tecrübesi, daha büyük native build/FFI maliyeti); webrtc-rs 0.20.x + ayrı capture/codec köprüleri; lisansı ve dağıtım yükümlülükleri incelenmiş hazır uzak masaüstü motoru. AnyDesk kodu, protokolü veya varlıkları kullanılmadı.

Video çözümü React içine kare kopyalamaz. Native sunum maliyetini ayrı ölçmek için AppKit penceresi kullanılır. Tauri ekranı test yönetimini yapar; native render gömülmesi sonraki adım olabilir. Donanım kodlama desteklenmezse VideoToolbox'ın yazılım seçeneğine izin verilir; gerçek fallback testi yapılmamıştır.

H.264'ün OS encoder'ını kullanmak tüm ürün dağıtım/patent yükümlülüklerinin çözüldüğü anlamına gelmez. İki platform codec deneyi ve dağıtım lisansı incelemesi tamamlanmadan ticari yayın kararı verilmez. macOS 14 deployment target kullanıldı; macOS 14'te çalıştığı iddia edilmiyor. x86_64 helper build'i Intel cihaz testi sayılmaz.

Resmî kaynaklar:
- [Apple ScreenCaptureKit örneği](https://developer.apple.com/documentation/screencapturekit/capturing-screen-content-in-macos)
- [Apple VideoToolbox](https://developer.apple.com/documentation/videotoolbox/vtcompressionsession)
- [Microsoft Desktop Duplication](https://learn.microsoft.com/en-us/windows/win32/direct3ddxgi/desktop-dup-api)
- [Microsoft SendInput ve UIPI sınırı](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-sendinput)
- [webrtc-rs 0.17.2 sürümü](https://github.com/webrtc-rs/webrtc/releases/tag/v0.17.2)
- [webrtc-rs mimari geçişi](https://webrtc.rs/blog/2026/01/31/webrtc-v0.17.0-feature-freeze-sansio-shift.html)
- [webrtc-rs güncel kaynak ve lisans](https://github.com/webrtc-rs/webrtc)
- [Tauri süreç modeli](https://v2.tauri.app/concept/process-model/)

- [webrtc-rs 0.20.5 kaynak](https://docs.rs/crate/webrtc/0.20.5/source/src/peer_connection/transports/turn_relayer.rs)
