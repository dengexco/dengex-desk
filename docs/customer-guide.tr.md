# Müşteri için geliştirme paketi kılavuzu

Bu paket henüz dengeX teknisyenine uzaktan bağlanmaz. Ana ekranda bu bilgisayarın kalıcı yerel kimliği gösterilir; sunucu kaydı yapılmadığı için geçici kod ve destek oluşturma henüz kullanılamaz. Teknik doğrulama bölümünde kendi bilgisayarınızda gerçek bileşenleri deneyebilirsiniz.

“Ekranımı 15 sn test et” düğmesine bastığınızda sistem izni kontrol edilir; izin varsa ayrı, görünür native pencerede test başlar. Açık olan ekranın görüntüsü aynı bilgisayardaki native test yüzeyine döner; kayıt dosyası oluşturulmaz. Durdur düğmesi veya pencereyi kapatmak testi bitirir; olağan deney 15 saniyede biter.

macOS Sistem Ayarları → Gizlilik ve Güvenlik → Ekran ve Sistem Sesi Kaydı: uygulamaya ekran izni verin. Klavye/fare testi için Erişilebilirlik izni ayrıca gerekir. İzin değişikliğinde yeniden açma gerekebilir. Sistem izni tek başına herhangi bir teknisyene erişim vermez.

Kalıcı Erişim bu pakette kapalıdır ve açılamaz. Kurulum başlangıç servisi eklemez. Dosya/pano/kamera/mikrofon paylaşımı veya gizli kayıt yapmaz. Sistem engeli varsa örnek görüntü ile başarılı gibi davranmaz.

## Ekran testi ve cihaz kimliği — 18 Eylül güncellemesi

“Teknik doğrulama” bölümünde “Ekran iznini aç” düğmesini kullanın. macOS Sistem Ayarları’nda “dengeX Remote Lab” için ekran kaydı iznini açın. macOS isterse uygulamayı kapatıp yeniden açın. “Ekranımı 15 sn test et” düğmesine bastığınızda gerçek ekranınız native pencerede gösterilir; Durdur testi sonlandırır. Test raporu izin engelini de kaydeder, yeni test aynı raporu yeniler.

Ana ekrandaki DX kimliği bu bilgisayarda oluşturulur ve uygulama tekrar açıldığında korunur. Henüz destek sunucusuna kayıt yapılmadığı için uzaktan bağlantı adresi olarak kullanılamaz. Geçici kod bu sürümde verilmez. Özel cihaz anahtarı macOS Anahtar Zinciri’nde saklanır. Kilitli veya erişilemeyen kasada yeni kimlik üretmek yerine hata gösterilir.
