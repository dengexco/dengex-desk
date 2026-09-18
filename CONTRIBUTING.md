# Katkıda bulunma

dengeX Remote, geliştirme aşamasında bir uzaktan destek projesidir. Güncel kapsam
ve doğrulama sınırları README ile `docs/testing/results.md` dosyasında açıklanır.

1. Depoyu fork edin ve değişikliğiniz için ayrı bir dal açın.
2. Küçük, incelenebilir değişiklikler hazırlayın; davranış değişiyorsa ilgili
   dokümantasyonu ve anlamlı testleri güncelleyin.
3. Araçlar PATH üzerinde hazırken `bash scripts/check.sh` çalıştırın. Yerel
   PostgreSQL, ekran/giriş izinleri ve iki fiziksel cihaz testleri ayrıca yapılır.
4. Pull request açıklamasında değişikliği, çalıştırılan kontrolleri ve test
   edemediğiniz platformları belirtin. Derlemeyi gerçek cihaz testi olarak sunmayın.

Oturum davetlerini, kimlik doğrulama anahtarlarını, `.env` dosyalarını, gerçek
cihaz kimliklerini veya ekran kayıtlarını eklemeyin. Hata raporlarında hassas
bilgileri çıkarın; güvenlik açığına ilişkin özel verileri herkese açık issue'ya
yazmayın. Testleri yalnızca sahibi olduğunuz veya açıkça izin verilen cihazlarda
çalıştırın.

Katkılar projenin [MIT lisansı](LICENSE) kapsamında yayımlanır. Üçüncü taraf
kodlarının lisans ve atıflarını koruyun.
