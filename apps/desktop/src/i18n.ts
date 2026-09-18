export const tr = {
  product:'Uzaktan destek, sizin kontrolünüzde.',
  subtitle:'Bağlantınızın her adımı görünür. Paylaşımı siz başlatır, siz bitirirsiniz.',
  support:'Destek al', technician:'Teknisyen konsolu', permissions:'İzinler', lab:'Teknik doğrulama',
  development:'Geliştirme sürümü', notConnected:'Bağlı değil', identity:'Bağlantı kimliğim', code:'Geçici kod',
  pendingCode:'Destek servisi bağlı değil',
  supportUnavailable:'Cihaz kimliği bu bilgisayarda kalıcıdır. Destek servisi henüz bağlı olmadığı için bu kimlikle uzaktan bağlantı kurulamaz ve geçici kod oluşturulamaz.',
  screen:'Ekran paylaşımı', input:'Klavye ve fare', allowed:'Sistem izni açık', required:'Sistem izni gerekli', unknown:'Native uygulamada kontrol edilir',
  refresh:'İzinleri kontrol et', testTitle:'Gerçek bileşenleri ayrı ayrı doğrulayın.',
  testHint:'Testler bu bilgisayarda çalışır. Sonuçlar iki cihaz arasında uzaktan destek doğrulaması sayılmaz.',
  transport:'WebRTC bağlantısı', transportHint:'İki yerel uç noktada şifreli veri kanalı ve yanıt testi.',
  synthetic:'Video codec testi', syntheticHint:'Sentetik kare → H.264 → WebRTC → native çözümleme. Ekranı yakalamaz.',
  capture:'Gerçek ekran testi', captureHint:'Düğmeye bastığınızda ekranınız bu bilgisayarda 15 saniye yakalanır ve görünür native pencerede gösterilir.',
  inputTest:'Yerel giriş testi', inputHint:'Türkçe metin ve fare tıklamasını yalnızca kendi test penceresine gönderir.',
  start:'Testi çalıştır', running:'Test çalışıyor…', noResult:'Henüz test çalıştırılmadı.',
  nativeOnly:'Tarayıcı görünümü. Gerçek testler kurulu masaüstü uygulamasında çalışır.',
  passed:'Geçti', failed:'Başarısız', theme:'Temayı değiştir', permanent:'Kalıcı erişim', disabled:'Kapalı',
};
export type TranslationKey = keyof typeof tr;
