import 'package:afudesk/hesap/hesap_servisi.dart';
import 'package:afudesk/main.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'sahte_motor.dart';

void main() {
  Future<void> ac(WidgetTester t, SahteHesapServisi servis) async {
    addTearDown(servis.dispose);
    addTearDown(() async => t.pumpWidget(const SizedBox.shrink()));
    await t.pumpWidget(AfuDeskUygulama(motor: SahteMotor(), hesapServisi: servis));
    await t.tap(find.text('Google ile giriş yap'));
    await t.pumpAndSettle();
  }

  testWidgets('giriş, liste, bağlantı şeridi ve kesme', (t) async {
    final servis = SahteHesapServisi(cihazlar: [
      const Cihaz(id: 'ev', ad: 'Ev bilgisayarı', cevrimici: true),
    ]);
    await ac(t, servis);
    expect(find.text('Bilgisayarlarım'), findsOneWidget);
    await t.tap(find.text('Ev bilgisayarı'));
    await t.pumpAndSettle();
    expect(find.text('Uzaktan bağlı: Ev bilgisayarı'), findsOneWidget);
    await t.tap(find.text('Kes'));
    await t.pumpAndSettle();
    expect(find.text('Uzaktan bağlı: Ev bilgisayarı'), findsNothing);
  });

  testWidgets('yanlış PIN tek cümle hata gösterir ve bağlantı açmaz', (t) async {
    final servis = SahteHesapServisi(cihazlar: [
      const Cihaz(id: 'ev', ad: 'Ev bilgisayarı', cevrimici: true, pinGerekli: true),
    ], cihazPinleri: {'ev': '123456'});
    await ac(t, servis);
    await t.tap(find.text('Ev bilgisayarı'));
    await t.pumpAndSettle();
    await t.enterText(find.byKey(const Key('hesap_pin')), '999999');
    await t.tap(find.text('Bağlan'));
    await t.pumpAndSettle();
    expect(find.text('PIN yanlış, yeniden dene.'), findsOneWidget);
    expect(servis.baglantilar, isEmpty);
  });

  testWidgets('PIN kurulumu rakam, uzunluk ve tekrar denetler', (t) async {
    final servis = SahteHesapServisi();
    await ac(t, servis);
    await t.tap(find.text('Uzaktan erişime izin ver'));
    await t.pumpAndSettle();
    await t.enterText(find.byKey(const Key('hesap_pin')), '123');
    await t.tap(find.text('Kaydet'));
    await t.pumpAndSettle();
    expect(find.text('PIN için en az 6 rakam gir.'), findsOneWidget);
    expect(servis.uzaktanErisim, isFalse);
    await t.enterText(find.byKey(const Key('hesap_pin')), '123456');
    await t.enterText(find.byKey(const Key('hesap_pin_tekrar')), '654321');
    await t.tap(find.text('Kaydet'));
    await t.pumpAndSettle();
    expect(find.text('PIN eşleşmiyor, iki alana aynı PIN’i gir.'), findsOneWidget);
    await t.enterText(find.byKey(const Key('hesap_pin_tekrar')), '123456');
    await t.tap(find.text('Kaydet'));
    await t.pumpAndSettle();
    expect(servis.pinKurulu, isTrue);
    expect(servis.uzaktanErisim, isTrue);
  });

  testWidgets('hatırlanan PIN ikinci bağlantıda sorulmaz', (t) async {
    final servis = SahteHesapServisi(cihazlar: [
      const Cihaz(id: 'ev', ad: 'Ev bilgisayarı', cevrimici: true, pinGerekli: true),
    ], cihazPinleri: {'ev': '123456'});
    await ac(t, servis);
    await t.tap(find.text('Ev bilgisayarı'));
    await t.pumpAndSettle();
    await t.enterText(find.byKey(const Key('hesap_pin')), '123456');
    await t.tap(find.text('Bu cihazda hatırla'));
    await t.tap(find.text('Bağlan'));
    await t.pumpAndSettle();
    await t.tap(find.text('Kes'));
    await t.pumpAndSettle();
    await t.tap(find.text('Ev bilgisayarı'));
    await t.pumpAndSettle();
    expect(find.byKey(const Key('hesap_pin')), findsNothing);
    expect(find.text('Uzaktan bağlı: Ev bilgisayarı'), findsOneWidget);
  });

  testWidgets('şerit eski bağlantı ekranında da görünür ve oturum akışını izler', (t) async {
    final servis = SahteHesapServisi(cihazlar: [
      const Cihaz(id: 'ev', ad: 'Ev bilgisayarı', cevrimici: true),
    ]);
    await ac(t, servis);
    await t.tap(find.text('Kodla bağlantı'));
    await t.pumpAndSettle();
    expect(find.byKey(const Key('secenek_baglan')), findsOneWidget);
    await servis.baglan('ev');
    await t.pumpAndSettle();
    expect(find.text('Uzaktan bağlı: Ev bilgisayarı'), findsOneWidget);
    await t.tap(find.text('Kes'));
    await t.pumpAndSettle();
    expect(servis.baglantilar, isEmpty);
    // AnaSayfa'nın periyodik yenileme zamanlayıcısını kapat.
    await t.pumpWidget(const SizedBox.shrink());
  });

  testWidgets('ayarlar onaysız erişimi kapatır, cihazı çıkarır ve çıkış yapar', (t) async {
    final servis = SahteHesapServisi(cihazlar: [
      const Cihaz(id: 'ev', ad: 'Ev bilgisayarı', cevrimici: true),
    ]);
    await ac(t, servis);
    await t.tap(find.byTooltip('Ayarlar'));
    await t.pumpAndSettle();
    expect(servis.onaysizBaglanti, isTrue);
    await t.tap(find.text('Onaysız bağlantı'));
    await t.pumpAndSettle();
    expect(servis.onaysizBaglanti, isFalse);
    await t.tap(find.text('Çıkar'));
    await t.pumpAndSettle();
    expect(find.text('Ev bilgisayarı'), findsNothing);
    await t.tap(find.text('Hesaptan çık'));
    await t.pumpAndSettle();
    expect(find.text('Google ile giriş yap'), findsOneWidget);
    expect(servis.baglantilar, isEmpty);
  });

  testWidgets('çevrimdışı cihaz bağlantı başlatmaz', (t) async {
    final servis = SahteHesapServisi(cihazlar: [
      const Cihaz(id: 'ev', ad: 'Ev bilgisayarı', cevrimici: false),
    ]);
    await ac(t, servis);
    await t.tap(find.text('Ev bilgisayarı'));
    await t.pumpAndSettle();
    expect(servis.baglantilar, isEmpty);
    expect(find.text('Çevrimdışı'), findsOneWidget);
  });

  testWidgets('üretim yer tutucusu sahte giriş yapmaz ve kodla akışı korur', (t) async {
    final servis = HazirDegilHesapServisi();
    addTearDown(servis.dispose);
    await t.pumpWidget(AfuDeskUygulama(motor: SahteMotor(), hesapServisi: servis));
    await t.tap(find.text('Google ile giriş yap'));
    await t.pumpAndSettle();
    expect(servis.hesap, isNull);
    expect(find.text('Google ile giriş henüz hazır değil, kodla bağlantıyı kullan.'), findsOneWidget);
    await t.tap(find.text('Kodla bağlantı'));
    await t.pumpAndSettle();
    expect(find.byKey(const Key('secenek_ver')), findsOneWidget);
    await t.pumpWidget(const SizedBox.shrink());
  });

  test('servis PIN atlamayı reddeder ve çıkışta kayıtları siler', () async {
    final servis = SahteHesapServisi(cihazlar: [
      const Cihaz(id: 'ev', ad: 'Ev bilgisayarı', cevrimici: true, pinGerekli: true),
    ], cihazPinleri: {'ev': '123456'});
    addTearDown(servis.dispose);
    await servis.girisYap();
    await expectLater(servis.baglan('ev'), throwsA(isA<HesapHatasi>()));
    expect(await servis.pinDogrula('ev', '123456', false), isTrue);
    await servis.baglan('ev');
    await servis.baglantiyiKes();
    await expectLater(servis.baglan('ev'), throwsA(isA<HesapHatasi>()));
    await servis.pinKur('123456');
    await servis.uzaktanErisimAyarla(true);
    await servis.cikisYap();
    expect(servis.pinKurulu, isFalse);
    expect(servis.uzaktanErisim, isFalse);
    await servis.girisYap();
    expect(await servis.bilgisayarlarim(), isEmpty);
  });
}
