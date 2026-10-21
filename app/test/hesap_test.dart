import 'dart:async';

import 'sahte_hesap_servisi.dart';
import 'package:afudesk/hesap/gercek_hesap_servisi.dart';
import 'package:afudesk/hesap/hesap_servisi.dart';
import 'package:afudesk/main.dart';
import 'package:afudesk/motor.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'sahte_motor.dart';

void main() {
  testWidgets('Google Terminal bağlantısı şeridi ve Kes düğmesini korur', (
    t,
  ) async {
    t.view.physicalSize = const Size(390, 844);
    t.view.devicePixelRatio = 1;
    addTearDown(t.view.resetPhysicalSize);
    addTearDown(t.view.resetDevicePixelRatio);
    final motor = SahteMotor();
    var acilis = 0;
    var kesildi = 0;
    final servis = GercekHesapServisi(
      islem: (op, _) async => switch (op) {
        'baslat' => '{"url":"https://accounts.google.com/"}',
        'tamamla' => '{"ad":"Hesabım"}',
        'liste' => '[{"id":"ev","ad":"Ev bilgisayarı","cevrimici":true}]',
        _ => '{}',
      },
      googleBaglan: (id) {
        expect(id, 'ev');
        acilis++;
        return motor.izleyici.stream;
      },
      kes: () async {
        kesildi++;
        motor.izleyici.add(IzleyiciOlay('koptu', metin: 'Bağlantı kapandı.'));
      },
      hostOlaylari: motor.hostOlaylari,
      platformGirisi: (_) async => {},
      tarayiciAc: (_) async => true,
    );
    addTearDown(servis.dispose);
    addTearDown(() async => t.pumpWidget(const SizedBox.shrink()));
    await t.pumpWidget(AfuDeskUygulama(motor: motor, hesapServisi: servis));
    await t.tap(find.text('Google ile giriş yap'));
    await t.pumpAndSettle();
    await t.tap(find.widgetWithText(TextButton, 'Terminal'));
    await t.pump();
    await t.pump(const Duration(milliseconds: 300));
    motor.izleyici.add(IzleyiciOlay('kabul', kontrol: true));
    await t.pump();
    expect(motor.girdiler.last.tur, 'terminal_liste');
    motor.izleyici.add(
      IzleyiciOlay(
        'terminal_liste',
        metin: '[{"id":42,"baslik":"Proje","uygulama":"PowerShell"}]',
      ),
    );
    await t.pump();
    expect(motor.girdiler.last.tur, 'terminal_ac');
    motor.izleyici.add(IzleyiciOlay('terminal_durum', metin: 'acik'));
    await t.pump();
    expect(find.byKey(const Key('terminal_modu')), findsOneWidget);
    final serit = find.text('Uzaktan bağlı: Ev bilgisayarı');
    expect(serit, findsOneWidget);
    expect(
      t.getBottomLeft(serit).dy,
      lessThan(t.getTopLeft(find.byKey(const Key('terminal_modu'))).dy),
    );
    expect(acilis, 1);
    expect(motor.cagrilar.where((s) => s == 'baglan'), isEmpty);
    await t.tap(find.text('Kes'));
    await t.pump();
    await t.pump();
    expect(kesildi, 1);
    expect(serit, findsNothing);
    expect(find.byKey(const Key('terminal_modu')), findsNothing);
    await servis.cikisYap();
    await t.pump();
    expect(t.takeException(), isNull);
  });

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
    expect(servis.sonTerminalleBasla, isFalse);
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
    await t.tap(find.byTooltip('Hesap cihazları'));
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
    expect(
      t
          .widget<TextButton>(find.widgetWithText(TextButton, 'Terminal'))
          .onPressed,
      isNull,
    );
  });

  testWidgets('her bilgisayar satırında Terminal düğmesi bulunur', (t) async {
    final servis = SahteHesapServisi(
      cihazlar: [const Cihaz(id: 'ev', ad: 'Ev bilgisayarı', cevrimici: true)],
    );
    await ac(t, servis);
    expect(find.widgetWithText(TextButton, 'Terminal'), findsOneWidget);
    await t.tap(find.widgetWithText(TextButton, 'Terminal'));
    await t.pumpAndSettle();
    expect(servis.sonTerminalleBasla, isTrue);
    expect(servis.baglantilar.single.id, 'ev');
  });

  testWidgets('başarısız gerçek giriş kodla akışı korur', (t) async {
    final servis = GercekHesapServisi(
      islem: (_, _) async =>
          throw const HesapHatasi('Google girişi tamamlanamadı, yeniden dene.'),
      googleBaglan: (_) => const Stream.empty(),
      kes: () async {},
      hostOlaylari: const Stream.empty(),
      platformGirisi: (_) async => {},
      tarayiciAc: (_) async => false,
    );
    addTearDown(servis.dispose);
    await t.pumpWidget(
      AfuDeskUygulama(motor: SahteMotor(), hesapServisi: servis),
    );
    await t.tap(find.text('Google ile giriş yap'));
    await t.pumpAndSettle();
    expect(servis.hesap, isNull);
    expect(
      find.text('Google girişi tamamlanamadı, yeniden dene.'),
      findsOneWidget,
    );
    await t.tap(find.text('Kodla bağlantı'));
    await t.pumpAndSettle();
    expect(find.byKey(const Key('secenek_ver')), findsOneWidget);
    await t.pumpWidget(const SizedBox.shrink());
  });

  testWidgets('bekleyen tanı kaydı hata ve yeniden denemeyi engellemez', (
    t,
  ) async {
    final kayit = Completer<void>();
    var girisSayisi = 0;
    final servis = GercekHesapServisi(
      islem: (op, _) async {
        if (op == 'baslat') {
          girisSayisi++;
          throw StateError('Google (G3)');
        }
        return '{}';
      },
      googleBaglan: (_) => const Stream.empty(),
      kes: () async {},
      hostOlaylari: const Stream.empty(),
      platformGirisi: (_) async => {},
      tarayiciAc: (_) async => false,
      taniYaz: (_) => kayit.future,
    );
    addTearDown(servis.dispose);
    addTearDown(() async => t.pumpWidget(const SizedBox.shrink()));
    await t.pumpWidget(
      AfuDeskUygulama(motor: SahteMotor(), hesapServisi: servis),
    );
    await t.tap(find.text('Google ile giriş yap'));
    await t.pumpAndSettle();
    expect(kayit.isCompleted, isFalse);
    expect(servis.hesap, isNull);
    expect(find.byType(LinearProgressIndicator), findsNothing);
    expect(
      find.text(
        'Drive erişimi tamamlanamadı (G3), yeniden giriş yapıp izin ver.',
      ),
      findsOneWidget,
    );
    await t.tap(find.text('Ayrıntı'));
    await t.pumpAndSettle();
    expect(find.text('G3 | baslat | StateError |  | '), findsOneWidget);
    await t.tap(find.text('Google ile giriş yap'));
    await t.pumpAndSettle();
    expect(girisSayisi, 2);
    expect(find.byType(LinearProgressIndicator), findsNothing);
    // A late storage failure must not escape into the UI either.
    kayit.completeError(StateError('disk full'));
    await t.pumpAndSettle();
    expect(t.takeException(), isNull);
    await t.tap(find.text('Kodla bağlantı'));
    await t.pumpAndSettle();
    expect(find.byKey(const Key('secenek_ver')), findsOneWidget);
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
