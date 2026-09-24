// Gerçek uçtan uca: Windows'ta uygulama açılır, gerçek Rust motoru, gerçek ekran,
// aynı makinede gerçek QUIC bağlantısı. Arayüzden kod alınır, ikinci bir motorla
// bağlanılır, istek arayüzde kabul edilir, gerçek ekran karesi gelir.
import 'dart:async';

import 'package:afudesk/main.dart';
import 'package:afudesk/motor.dart';
import 'package:afudesk/src/rust/frb_generated.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';

Future<void> bekle(WidgetTester t, bool Function() kosul, {int sn = 20, String ne = ''}) async {
  final son = DateTime.now().add(Duration(seconds: sn));
  while (!kosul()) {
    if (DateTime.now().isAfter(son)) fail('Zaman aşımı: $ne');
    await t.pump(const Duration(milliseconds: 100));
  }
}

String metin(WidgetTester t, Key k) {
  final w = t.widget(find.byKey(k));
  if (w is SelectableText) return w.data ?? '';
  if (w is Text) return w.data ?? '';
  return '';
}

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();
  setUpAll(() async => RustLib.init());

  testWidgets('gerçek motor: kod üret → bağlan → onayla → gerçek ekran → kes', (t) async {
    final motor = RustMotor();
    await t.pumpWidget(AfuDeskUygulama(motor: motor));
    await t.tap(find.byKey(const Key('secenek_ver')));
    await bekle(t, () => find.byKey(const Key('ver_kod')).evaluate().isNotEmpty, ne: 'kod üretilmedi');

    final kod = metin(t, const Key('ver_kod'));
    final parola = metin(t, const Key('ver_parola'));
    expect(kod, startsWith('AFU2.'));
    expect(parola, matches(RegExp(r'^\d{6}$')));
    expect(metin(t, const Key('ver_erisim')), isNotEmpty);
    debugPrint('KOD uzunluğu=${kod.length} erişim="${metin(t, const Key('ver_erisim'))}"');

    // Yanlış parola: bağlanmadan reddedilmeli.
    late IzleyiciOlay yanlis;
    await t.runAsync(() async {
      yanlis = await RustMotor().baglan(kod: kod, parola: '000000', ad: 'Yanlis').first;
    });
    expect(yanlis.tur, 'hata');
    expect(yanlis.metin, 'Parola yanlış.');

    // Doğru parola: ikinci motorla bağlan (aynı süreç, gerçek ağ).
    final olaylar = <IzleyiciOlay>[];
    late StreamSubscription<IzleyiciOlay> abone;
    await t.runAsync(() async {
      abone = RustMotor().baglan(kod: kod, parola: parola, ad: 'Deneme').listen((o) {
        olaylar.add(o);
        if (o.tur == 'kare') RustMotor().kareCizildi();
      });
    });
    await bekle(t, () => find.text('Deneme bağlanmak istiyor').evaluate().isNotEmpty, ne: 'istek penceresi açılmadı');
    expect(olaylar.any((o) => o.tur == 'bekliyor'), isTrue);

    // Kontrol iznini kaldırıp kabul et (testte gerçek fareyi oynatmayalım).
    await t.tap(find.byKey(const Key('istek_kontrol')));
    await t.pump();
    await t.tap(find.byKey(const Key('istek_kabul')));
    await bekle(t, () => find.byKey(const Key('ver_bagli')).evaluate().isNotEmpty, ne: 'bağlı durumuna geçmedi');
    expect(find.text('Yalnız izliyor; kontrol edemez.'), findsOneWidget);

    await bekle(t, () => olaylar.any((o) => o.tur == 'kare'), ne: 'gerçek ekran karesi gelmedi');
    final kabul = olaylar.firstWhere((o) => o.tur == 'kabul');
    final kare = olaylar.firstWhere((o) => o.tur == 'kare');
    expect(kabul.kontrol, isFalse);
    expect(kare.genislik, greaterThan(300));
    expect(kare.rgba.length, kare.genislik * kare.yukseklik * 4);
    final dolu = kare.rgba.where((b) => b != 0 && b != 255).length;
    expect(dolu, greaterThan(kare.rgba.length ~/ 20), reason: 'kare boş/siyah olmamalı');
    debugPrint('KARE ${kare.genislik}x${kare.yukseklik}, olay sayısı=${olaylar.length}');

    // Host keser → izleyici sebebi öğrenir, arayüz yeni koda döner.
    await t.tap(find.byKey(const Key('ver_kes')));
    await bekle(t, () => olaylar.any((o) => o.tur == 'koptu'), ne: 'izleyici kopmayı öğrenmedi');
    expect(olaylar.lastWhere((o) => o.tur == 'koptu').metin, contains('kesti'));
    await bekle(t, () => find.byKey(const Key('ver_kod')).evaluate().isNotEmpty, ne: 'yeni kod gelmedi');
    expect(metin(t, const Key('ver_kod')), isNot(kod), reason: 'bilet tek kullanımlık: yeni kod');

    // Eski kodla tekrar bağlanılamaz (bilet yakıldı).
    final eski = <IzleyiciOlay>[];
    await t.runAsync(() async {
      final s = RustMotor().baglan(kod: kod, parola: parola, ad: 'Tekrar').listen(eski.add, onError: (_) {});
      await Future<void>.delayed(const Duration(seconds: 4));
      await s.cancel();
    });
    expect(find.text('Tekrar bağlanmak istiyor'), findsNothing);
    await abone.cancel();
  });

  testWidgets('gerçek motor: izleyici arayüzü koda bağlanır ve gerçek ekranı çizer', (t) async {
    final motor = RustMotor();
    // Host'u arayüzsüz, motorla başlat; isteği otomatik kabul et.
    String kod = '', parola = '';
    late StreamSubscription<HostOlay> host;
    await t.runAsync(() async {
      final hazir = Completer<void>();
      host = RustMotor().hostBaslat(ad: 'Uzak PC').listen((o) {
        if (o.tur == 'hazir' && !hazir.isCompleted) {
          kod = o.kod;
          parola = o.parola;
          hazir.complete();
        }
        if (o.tur == 'istek') RustMotor().hostKabul(kontrol: true, oyunKolu: true);
      });
      await hazir.future.timeout(const Duration(seconds: 20));
    });

    await t.pumpWidget(AfuDeskUygulama(motor: motor));
    await t.tap(find.byKey(const Key('secenek_baglan')));
    await t.pump(const Duration(milliseconds: 500));
    await t.enterText(find.byKey(const Key('baglan_kod')), kod);
    await t.enterText(find.byKey(const Key('baglan_parola')), parola);
    await t.tap(find.byKey(const Key('baglan_dugme')));
    await bekle(t, () => find.byKey(const Key('oturum_kare')).evaluate().isNotEmpty, ne: 'oturum ekranında kare çizilmedi');
    expect(find.text('Uzak PC'), findsOneWidget, reason: 'başlıkta karşı tarafın adı');
    expect(find.text('Kontrol açık'), findsOneWidget);
    final resim = t.widget<RawImage>(find.byKey(const Key('oturum_kare'))).image!;
    expect(resim.width, greaterThan(300));
    debugPrint('ÇİZİLEN ${resim.width}x${resim.height}');

    // Saniyede bir istatistik gelmeli (gerçek RTT ve FPS).
    await bekle(t, () => find.byKey(const Key('oturum_istatistik')).evaluate().isNotEmpty, ne: 'istatistik gelmedi');
    await t.pump(const Duration(seconds: 2));
    debugPrint('ISTATISTIK ${t.widget<Text>(find.byKey(const Key('oturum_istatistik'))).data}');
    expect(find.byKey(const Key('oturum_kare')), findsOneWidget);

    // Kes → ana akışa dönüş.
    await t.tap(find.byKey(const Key('oturum_kes')));
    await t.pump(const Duration(milliseconds: 600));
    expect(find.byKey(const Key('baglan_dugme')), findsOneWidget);
    await host.cancel();
    await RustMotor().hostDurdur();
  });
}
