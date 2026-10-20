import 'dart:convert';
import 'package:afudesk/guncelleme.dart';
import 'package:afudesk/bilesenler/guncelle_karti.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

class SahteGuncelleyici extends Guncelleyici {
  Guncelleme? yayin;
  int kontroller = 0;
  Object? hata;
  @override
  Future<Guncelleme?> kontrol() async {
    kontroller++;
    return yayin;
  }
  @override
  Future<void> kur(Guncelleme g, void Function(double) ilerleme) async {
    if (hata != null) throw hata!;
  }
}

void main() {
  setUp(() => GuncelleKarti.ertelenen.clear());
  final yeni = Guncelleme('v1.7.0', Uri.parse('https://github.com/paket'),
      Uri.parse('https://github.com/ozet'), 100);
  testWidgets('Acilista ve gunde bir kontrol eder, geri donuste tekrarlamaz', (
    tester,
  ) async {
    final servis = SahteGuncelleyici();
    await tester.pumpWidget(MaterialApp(home: GuncelleKarti(servis: servis)));
    await tester.pumpAndSettle();
    expect(servis.kontroller, 1);
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
    await tester.pumpAndSettle();
    expect(servis.kontroller, 1);
    await tester.pump(const Duration(hours: 23));
    expect(servis.kontroller, 1);
    await tester.pump(const Duration(hours: 1));
    await tester.pumpAndSettle();
    expect(servis.kontroller, 2);
    guncellemeKontrolIstegi.value++;
    await tester.pumpAndSettle();
    expect(servis.kontroller, 3);
    await tester.pumpWidget(const SizedBox.shrink());
    await tester.pump(const Duration(days: 1));
    expect(servis.kontroller, 3);
  });
  test('1.8.0 surumu yalniz daha eski surumlere onerilir', () {
    expect(yeniSurum('v1.8.0', '1.7.0+8'), isTrue);
    expect(yeniSurum('v1.8.0', '1.8.0+9'), isFalse);
    expect(yeniSurum('v1.8.0', '1.9.0'), isFalse);
    expect(yeniSurum('v1.8.0-rc.1', '1.7.0'), isFalse);
  });
  test('releases/latest JSON Android paketini ve ozetini ayristirir', () {
    final json =
        jsonDecode('''{
      "tag_name": "v1.8.0", "draft": false, "prerelease": false,
      "assets": [
        {"name": "AfuDesk-windows-x64.zip", "size": 200,
         "browser_download_url": "https://github.com/pirncedark/afudesk/releases/download/v1.8.0/AfuDesk-windows-x64.zip"},
        {"name": "AfuDesk-android.apk", "size": 123456,
         "browser_download_url": "https://github.com/pirncedark/afudesk/releases/download/v1.8.0/AfuDesk-android.apk"},
        {"name": "AfuDesk-android.apk.sha256", "size": 86,
         "browser_download_url": "https://github.com/pirncedark/afudesk/releases/download/v1.8.0/AfuDesk-android.apk.sha256"}
      ]
    }''')
            as Map<String, dynamic>;
    final g = Guncelleme.yayindan(json, '1.7.0+8', 'AfuDesk-android.apk')!;
    expect(g.surum, 'v1.8.0');
    expect(g.boyut, 123456);
    expect(g.paket.path, endsWith('/v1.8.0/AfuDesk-android.apk'));
    expect(g.ozet.toString(), '${g.paket}.sha256');
  });
  test('Windows kaynak secimi her iki asamada yapilir', () {
    expect(windowsKurulum, contains("Join-Path \$stage 'afudesk.exe'"));
    expect(windowsKurulum, contains("Join-Path \$stage 'AfuDesk'"));
    expect(windowsKurulum, contains(r'return $stage'));
    expect(windowsKurulum, contains("Join-Path \$nested 'afudesk.exe'"));
    expect(windowsKurulum, contains(r'return $nested'));
    expect(windowsKurulum, contains("throw 'Paket eksik'"));
    expect(RegExp(r'\$source = Paket-Kaynagi').allMatches(windowsKurulum).length, 2);
    expect(windowsKurulum, contains('UnauthorizedAccessException'));
    expect(windowsKurulum, contains('exit 3'));
  });
  test('Prepare gizli pencere argumanlarini kullanir', () {
    expect(windowsPrepareArgumanlari('kur.ps1'), [
      '-NoProfile', '-NonInteractive', '-WindowStyle', 'Hidden',
      '-ExecutionPolicy', 'Bypass', '-File', 'kur.ps1', '-Prepare',
    ]);
  });
  testWidgets('Yeni surum gorunur, Sonra ayni surumu bu acilista gizler', (tester) async {
    final servis = SahteGuncelleyici()..yayin = yeni;
    await tester.pumpWidget(MaterialApp(home: GuncelleKarti(servis: servis)));
    await tester.pumpAndSettle();
    expect(find.byKey(const Key('guncelle_karti')), findsOneWidget);
    expect(find.byKey(const Key('guncelle_dugme')), findsOneWidget);
    await tester.tap(find.byKey(const Key('guncelle_sonra')));
    await tester.pumpAndSettle();
    expect(find.byKey(const Key('guncelle_karti')), findsNothing);
    guncellemeKontrolIstegi.value++;
    await tester.pumpAndSettle();
    expect(servis.kontroller, 2);
    expect(find.byKey(const Key('guncelle_karti')), findsNothing);
    await tester.pumpWidget(const SizedBox.shrink());
    await tester.pumpWidget(MaterialApp(home: GuncelleKarti(servis: servis)));
    await tester.pumpAndSettle();
    expect(find.byKey(const Key('guncelle_karti')), findsNothing);
    servis.yayin = Guncelleme('v1.8.0', yeni.paket, yeni.ozet, 100);
    guncellemeKontrolIstegi.value++;
    await tester.pumpAndSettle();
    expect(find.byKey(const Key('guncelle_karti')), findsOneWidget);
    await tester.pumpWidget(const SizedBox.shrink());
    final sayi = servis.kontroller;
    guncellemeKontrolIstegi.value++;
    expect(servis.kontroller, sayi);
  });
  testWidgets('Yeni surum yoksa kart yer kaplamaz', (tester) async {
    await tester.pumpWidget(MaterialApp(home: GuncelleKarti(servis: SahteGuncelleyici())));
    await tester.pumpAndSettle();
    expect(find.byKey(const Key('guncelle_karti')), findsNothing);
    await tester.pumpWidget(const SizedBox.shrink());
  });
  testWidgets('Izin yonlendirmesi ve genel hata mesaji', (tester) async {
    final servis = SahteGuncelleyici()..yayin = yeni
        ..hata = const GuncellemeSayfasiAcildi();
    await tester.pumpWidget(MaterialApp(home: GuncelleKarti(servis: servis)));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('guncelle_dugme')));
    await tester.pumpAndSettle();
    expect(find.text('Güncelleme sayfası açıldı. Yeni sürümü oradan indir.'), findsOneWidget);
    servis.hata = Exception('hata');
    await tester.tap(find.byKey(const Key('guncelle_dugme')));
    await tester.pumpAndSettle();
    expect(find.text('Güncelleme tamamlanamadı. Yeniden dene.'), findsOneWidget);
    await tester.pumpWidget(const SizedBox.shrink());
  });
  test('sayısal sürüm karşılaştırması ve ön sürüm reddi', () {
    expect(yeniSurum('v1.10.0', '1.9.9+7'), isTrue);
    expect(yeniSurum('v2.0.0', '1.99.99'), isTrue);
    for (final s in ['v1.6.0', 'v1.5.9', 'v1.7.0-beta', 'bozuk']) {
      expect(yeniSurum(s, '1.6.0+7'), isFalse);
    }
  });
  Map<String, dynamic> yayin() => {
    'tag_name': 'v1.7.0',
    'draft': false,
    'prerelease': false,
    'assets': [
      for (final name in ['AfuDesk-android.apk', 'AfuDesk-android.apk.sha256'])
        {
          'name': name,
          'size': 100,
          'browser_download_url':
              'https://github.com/pirncedark/afudesk/releases/download/v1.7.0/$name',
        },
    ],
  };
  test('yalnız platform paketi ve özeti birlikteyse sunulur', () {
    expect(
      Guncelleme.yayindan(yayin(), '1.6.0', 'AfuDesk-android.apk')?.surum,
      'v1.7.0',
    );
    expect(
      Guncelleme.yayindan(yayin(), '1.6.0', 'AfuDesk-windows-x64.zip'),
      isNull,
    );
    final j = yayin();
    (j['assets'] as List).removeLast();
    expect(Guncelleme.yayindan(j, '1.6.0', 'AfuDesk-android.apk'), isNull);
  });
  test('taslak, ön sürüm, eski sürüm ve yabancı adres reddedilir', () {
    for (final key in ['draft', 'prerelease']) {
      final j = yayin()..[key] = true;
      expect(Guncelleme.yayindan(j, '1.6.0', 'AfuDesk-android.apk'), isNull);
    }
    expect(
      Guncelleme.yayindan(yayin(), '1.7.0', 'AfuDesk-android.apk'),
      isNull,
    );
    final j = yayin();
    (j['assets'] as List).first['browser_download_url'] =
        'https://example.com/app.apk';
    expect(
      () => Guncelleme.yayindan(j, '1.6.0', 'AfuDesk-android.apk'),
      throwsFormatException,
    );
  });
}
