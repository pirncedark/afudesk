import 'package:afudesk/hesap/hesap_servisi.dart';
import 'package:afudesk/main.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'sahte_motor.dart';

void main() {
  Future<void> ac(WidgetTester t, SahteHesapServisi servis) async {
    addTearDown(servis.dispose);
    addTearDown(() async => t.pumpWidget(const SizedBox.shrink()));
    await t.pumpWidget(
      AfuDeskUygulama(motor: SahteMotor(), hesapServisi: servis),
    );
    await t.tap(find.text('Google ile giriş yap'));
    await t.pumpAndSettle();
  }

  testWidgets('giriş, liste, bağlantı şeridi ve kesme', (t) async {
    final servis = SahteHesapServisi(
      cihazlar: [const Cihaz(id: 'ev', ad: 'Ev bilgisayarı', cevrimici: true)],
    );
    await ac(t, servis);
    expect(find.text('Bilgisayarlarım'), findsOneWidget);
    await t.tap(find.text('Ev bilgisayarı'));
    await t.pumpAndSettle();
    expect(find.text('Uzaktan bağlı: Ev bilgisayarı'), findsOneWidget);
    await t.tap(find.text('Kes'));
    await t.pumpAndSettle();
    expect(find.text('Uzaktan bağlı: Ev bilgisayarı'), findsNothing);
  });

  testWidgets('aynı hesap tek dokunuşla PIN ve onay olmadan bağlanır', (
    t,
  ) async {
    final servis = SahteHesapServisi(
      cihazlar: [const Cihaz(id: 'ev', ad: 'Ev bilgisayarı', cevrimici: true)],
    );
    await ac(t, servis);
    expect(servis.uzaktanErisim, isTrue);
    await t.tap(find.text('Ev bilgisayarı'));
    await t.pumpAndSettle();
    expect(find.byType(AlertDialog), findsNothing);
    expect(servis.baglantilar.single.id, 'ev');
    expect(find.text('Uzaktan bağlı: Ev bilgisayarı'), findsOneWidget);
  });

  testWidgets(
    'şerit eski bağlantı ekranında da görünür ve oturum akışını izler',
    (t) async {
      final servis = SahteHesapServisi(
        cihazlar: [
          const Cihaz(id: 'ev', ad: 'Ev bilgisayarı', cevrimici: true),
        ],
      );
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
    },
  );

  testWidgets('ayarlar cihazı çıkarır ve çıkış yapar', (t) async {
    final servis = SahteHesapServisi(
      cihazlar: [const Cihaz(id: 'ev', ad: 'Ev bilgisayarı', cevrimici: true)],
    );
    await ac(t, servis);
    await t.tap(find.byTooltip('Ayarlar'));
    await t.pumpAndSettle();
    await t.tap(find.text('Çıkar'));
    await t.pumpAndSettle();
    expect(find.text('Ev bilgisayarı'), findsNothing);
    await t.tap(find.text('Hesaptan çık'));
    await t.pumpAndSettle();
    expect(find.text('Google ile giriş yap'), findsOneWidget);
    expect(servis.baglantilar, isEmpty);
  });

  testWidgets('çevrimdışı cihaz bağlantı başlatmaz', (t) async {
    final servis = SahteHesapServisi(
      cihazlar: [const Cihaz(id: 'ev', ad: 'Ev bilgisayarı', cevrimici: false)],
    );
    await ac(t, servis);
    await t.tap(find.text('Ev bilgisayarı'));
    await t.pumpAndSettle();
    expect(servis.baglantilar, isEmpty);
    expect(find.text('Çevrimdışı'), findsOneWidget);
  });

  testWidgets('üretim yer tutucusu sahte giriş yapmaz ve kodla akışı korur', (
    t,
  ) async {
    final servis = HazirDegilHesapServisi();
    addTearDown(servis.dispose);
    await t.pumpWidget(
      AfuDeskUygulama(motor: SahteMotor(), hesapServisi: servis),
    );
    await t.tap(find.text('Google ile giriş yap'));
    await t.pumpAndSettle();
    expect(servis.hesap, isNull);
    expect(
      find.text('Google ile giriş henüz hazır değil, kodla bağlantıyı kullan.'),
      findsOneWidget,
    );
    await t.tap(find.text('Kodla bağlantı'));
    await t.pumpAndSettle();
    expect(find.byKey(const Key('secenek_ver')), findsOneWidget);
    await t.pumpWidget(const SizedBox.shrink());
  });

  test('çıkış açık bağlantıyı ve hesap listesini temizler', () async {
    final servis = SahteHesapServisi(
      cihazlar: [const Cihaz(id: 'ev', ad: 'Ev bilgisayarı', cevrimici: true)],
    );
    addTearDown(servis.dispose);
    await servis.girisYap();
    await servis.baglan('ev');
    await servis.cikisYap();
    expect(servis.uzaktanErisim, isFalse);
    expect(servis.baglantilar, isEmpty);
    await expectLater(servis.baglan('ev'), throwsA(isA<HesapHatasi>()));
    await servis.girisYap();
    expect(await servis.bilgisayarlarim(), isEmpty);
  });
}
