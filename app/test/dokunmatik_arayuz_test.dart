// Telefon (dokunmatik) kipinin arayüz testleri.

import 'package:afudesk/main.dart';
import 'package:afudesk/motor.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';

import 'sahte_motor.dart';

Future<SahteMotor> telefon(WidgetTester t, {bool kontrol = true}) async {
  t.view.physicalSize = const Size(400, 800);
  t.view.devicePixelRatio = 1;
  addTearDown(t.view.reset);
  final m = SahteMotor()
    ..dokunmatik = true
    ..baglantiVerilebilir = false;
  await t.pumpWidget(AfuDeskUygulama(motor: m));
  await t.tap(find.byKey(const Key('secenek_baglan')));
  await gec(t);
  await t.enterText(find.byKey(const Key('baglan_kod')), 'AFU2.abc');
  await t.enterText(find.byKey(const Key('baglan_parola')), '123456');
  await t.tap(find.byKey(const Key('baglan_dugme')));
  await gec(t);
  m.izleyici.add(IzleyiciOlay('kabul', kontrol: kontrol, genislik: 200, yukseklik: 100));
  await t.pump();
  await t.runAsync(() async {
    m.izleyici.add(IzleyiciOlay('kare', genislik: 200, yukseklik: 100, rgba: Uint8List.fromList(List.filled(200 * 100 * 4, 90))));
  });
  for (var i = 0; i < 200 && find.byKey(const Key('oturum_kare')).evaluate().isEmpty; i++) {
    await t.runAsync(() => Future<void>.delayed(const Duration(milliseconds: 20)));
    await t.pump();
  }
  await t.pump();
  return m;
}

Future<void> gec(WidgetTester t) async {
  for (var i = 0; i < 4; i++) {
    await t.pump(const Duration(milliseconds: 250));
  }
}

List<String> fare(SahteMotor m) =>
    m.girdiler.where((g) => g.tur == 'fare').map((g) => '${g.ad}:${g.basili}').toList();

void main() {
  testWidgets('telefonda "Bağlantı ver" devre dışı, "Bağlan" üstte', (t) async {
    t.view.physicalSize = const Size(400, 800);
    t.view.devicePixelRatio = 1;
    addTearDown(t.view.reset);
    final m = SahteMotor()..baglantiVerilebilir = false;
    await t.pumpWidget(AfuDeskUygulama(motor: m));
    expect(find.textContaining('ekran paylaşımı yakında'), findsOneWidget);
    final ver = t.getTopLeft(find.byKey(const Key('secenek_ver')));
    final bag = t.getTopLeft(find.byKey(const Key('secenek_baglan')));
    expect(bag.dy, lessThan(ver.dy));
    await t.tap(find.byKey(const Key('secenek_ver')));
    await gec(t);
    expect(m.cagrilar.where((c) => c.startsWith('hostBaslat')), isEmpty);
  });

  testWidgets('dokunma = sol tık, imleç işareti görünür', (t) async {
    final m = await telefon(t);
    final alan = t.getRect(find.byKey(const Key('oturum_ekran')));
    await t.tapAt(alan.center);
    await t.pump();
    expect(fare(m), ['sol:true', 'sol:false']);
    final k = m.girdiler.firstWhere((g) => g.tur == 'konum');
    expect(k.x, closeTo(0.5, 0.02));
    expect(k.y, closeTo(0.5, 0.02));
    expect(find.byKey(const Key('oturum_imlec')), findsOneWidget);
  });

  testWidgets('uzun bas = sağ tık', (t) async {
    final m = await telefon(t);
    final alan = t.getRect(find.byKey(const Key('oturum_ekran')));
    final g = await t.startGesture(alan.center);
    // Gerçek zamanlı saat kullanıldığı için gerçekten bekle.
    await t.runAsync(() => Future<void>.delayed(const Duration(milliseconds: 750)));
    await t.pump(const Duration(milliseconds: 200));
    await g.up();
    await t.pump();
    expect(fare(m), ['sag:true', 'sag:false']);
  });

  testWidgets('sürükleme = sol basılı hareket', (t) async {
    final m = await telefon(t);
    final alan = t.getRect(find.byKey(const Key('oturum_ekran')));
    final g = await t.startGesture(alan.center);
    await g.moveBy(const Offset(60, 0));
    await g.moveBy(const Offset(40, 0));
    await g.up();
    await t.pump();
    expect(fare(m), ['sol:true', 'sol:false']);
    expect(m.girdiler.where((x) => x.tur == 'konum').length, greaterThan(2));
  });

  testWidgets('özel tuş çubuğu: Enter ve Esc', (t) async {
    final m = await telefon(t);
    await t.tap(find.byKey(const Key('ozel_Enter')));
    await t.tap(find.byKey(const Key('ozel_Escape')));
    final tuslar = m.girdiler.where((g) => g.tur == 'tus').map((g) => '${g.ad}:${g.basili}').toList();
    expect(tuslar, ['Enter:true', 'Enter:false', 'Escape:true', 'Escape:false']);
    expect(fare(m), isEmpty, reason: 'çubuğa dokunmak ekranı tıklamamalı');
  });

  testWidgets('telefon klavyesinden yazı ve silme', (t) async {
    final m = await telefon(t);
    await t.tap(find.byKey(const Key('oturum_klavye')));
    await t.pump();
    await t.enterText(find.byKey(const Key('oturum_yazi')), '​merhaba');
    await t.pump();
    await t.enterText(find.byKey(const Key('oturum_yazi')), '');
    await t.pump();
    expect(m.girdiler.where((g) => g.tur == 'metin').single.metin, 'merhaba');
    final sil = m.girdiler.where((g) => g.tur == 'tus').map((g) => '${g.ad}:${g.basili}').toList();
    expect(sil, ['Backspace:true', 'Backspace:false']);
  });

  testWidgets('dar ekranda başlık kesilmez: çip ve Kes yalnız simge', (t) async {
    await telefon(t);
    expect(find.text('ALI-PC'), findsNothing); // bu testte 'bekliyor' yok, başlık varsayılan
    expect(find.text('Uzak ekran'), findsOneWidget);
    expect(find.text('Kontrol açık'), findsNothing);
    expect(find.byTooltip('Bağlantıyı kes'), findsOneWidget);
  });

  testWidgets('dokunmatik oturum yatay kipe geçer, çıkınca serbest bırakır', (t) async {
    final yonler = <Object?>[];
    t.binding.defaultBinaryMessenger.setMockMethodCallHandler(SystemChannels.platform, (c) async {
      if (c.method == 'SystemChrome.setPreferredOrientations') yonler.add(c.arguments);
      return null;
    });
    await telefon(t);
    expect(yonler.first, ['DeviceOrientation.landscapeLeft', 'DeviceOrientation.landscapeRight']);
    await t.tap(find.byTooltip('Bağlantıyı kes'));
    await gec(t);
    expect((yonler.last as List).length, 4, reason: 'çıkınca tüm yönler serbest');
  });

  testWidgets('yalnız izleme kipinde dokunma ve çubuk yok', (t) async {
    final m = await telefon(t, kontrol: false);
    final alan = t.getRect(find.byKey(const Key('oturum_ekran')));
    await t.tapAt(alan.center);
    await t.pump();
    expect(m.girdiler, isEmpty);
    expect(find.byKey(const Key('oturum_klavye')), findsNothing);
  });

  testWidgets('dokunmatik oturumda dosya gönder düğmesi yok (yalnız masaüstü)', (t) async {
    await telefon(t);
    expect(find.byKey(const Key('oturum_dosya_gonder')), findsNothing);
  });
}
