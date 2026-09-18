# Teknisyen için Aşama 0 kılavuzu

Teknisyen oturumu ve müşteri cihaz listesi henüz bağlı değildir. Bu paket müşteri cihazına gönderilip tamamlanmış destek ürünü gibi tanıtılmamalı. Yerel native motor deneyleri için README'deki komutları kullanın; JSON raporları `.artifacts` altında, paket içinden başlayan testler OS uygulama veri klasöründe tutulur.

Raporu değerlendirirken `experiment`, `syntheticSource`, `twoDeviceTest` alanlarını okuyun. Sentetik codec testi ekran yakalama kanıtı değildir; `transport` yanıt süresi input-to-photon veya ilk kullanılabilir kare gecikmesi değildir. Yerel native input testi global tuş düzeni veya uzaktaki masaüstü kontrol testi değildir.

Gerçek müşteri pilotuna geçiş için `docs/testing/manual-matrix.md` kapısı uygulanır. Windows/macOS cihaz erişimi, gerekli OS izinleri ve test ağları sağlandığında her yön için ayrı kayıt tutulur. İmza sertifikası/Developer ID ve gerçek TURN/TLS/IdP ortamı olmadan production yayın işaretlenmez.
