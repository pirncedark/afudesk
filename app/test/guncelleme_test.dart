import 'dart:convert';

import 'package:afudesk/guncelleme.dart';
import 'package:afudesk/bilesenler/guncelle_karti.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:afudesk/sayfalar/ayarlar.dart';

class SahteGuncelleyici extends Guncelleyici {
  Guncelleme? sonuc;
  int kontroller = 0;
  Object? hata;
  Object? kontrolHatasi;
  @override
  Future<String> surum() async => '1.6.0';
  @override
  Future<Guncelleme?> kontrol() async {
    kontroller++;
    if (kontrolHatasi != null) throw kontrolHatasi!;
    return sonuc;
  }

  @override
  Future<void> kur(Guncelleme g, void Function(double) ilerleme) async {
    if (hata != null) throw hata!;
  }
}

void main() {
  final yeni = Guncelleme(
    'v1.7.0',
    Uri.parse('https://github.com/paket'),
    Uri.parse('https://github.com/ozet'),
    100,
  );
  testWidgets('Acilista ve gunde bir kontrol eder, geri donuste tekrarlamaz', (
    tester,
  ) async {
    final servis = SahteGuncelleyici();
    await tester.pumpWidget(
      MaterialApp(home: GuncelleKarti(servis: servis, automaticCheck: true)),
    );
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
  testWidgets('Açılışta kontrol kapalıysa otomatik denetim yapılmaz', (
    tester,
  ) async {
    final servis = SahteGuncelleyici();
    await tester.pumpWidget(
      MaterialApp(home: GuncelleKarti(servis: servis, automaticCheck: false)),
    );
    await tester.pumpAndSettle();
    expect(servis.kontroller, 0);
    await tester.pumpWidget(const SizedBox.shrink());
  });
  testWidgets('Bes dakika sonra donuste yeni yayin karti gorunur', (
    tester,
  ) async {
    final servis = SahteGuncelleyici();
    await tester.pumpWidget(MaterialApp(home: GuncelleKarti(servis: servis)));
    await tester.pumpAndSettle();
    servis.sonuc = yeni;
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
    await tester.pump(const Duration(minutes: 4, seconds: 59));
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
    await tester.pumpAndSettle();
    expect(servis.kontroller, 1);
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
    await tester.pump(const Duration(seconds: 1));
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
    await tester.pumpAndSettle();
    expect(servis.kontroller, 2);
    expect(find.byKey(const Key('guncelle_karti')), findsOneWidget);
    await tester.pumpWidget(const SizedBox.shrink());
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
    await tester.pump(const Duration(minutes: 5));
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
    await tester.pumpAndSettle();
    expect(servis.kontroller, 2);
  });
  testWidgets('Ag hatasi sonraki donuste yeniden denenir ve Guncelle gorunur', (
    tester,
  ) async {
    final servis = SahteGuncelleyici()..kontrolHatasi = Exception('ag yok');
    await tester.pumpWidget(
      MaterialApp(home: GuncelleKarti(servis: servis, ayarDugmesi: true)),
    );
    await tester.pumpAndSettle();
    expect(servis.kontroller, 1);
    servis.kontrolHatasi = null;
    servis.sonuc = yeni;
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
    await tester.pumpAndSettle();
    expect(servis.kontroller, 2);
    expect(find.widgetWithText(TextButton, 'Güncelle'), findsOneWidget);
    expect(find.byIcon(Icons.system_update), findsOneWidget);
    await tester.pumpWidget(const SizedBox.shrink());
  });
  testWidgets('Hesap ekranindaki Guncelle dugmesi Ayarlar acar', (
    tester,
  ) async {
    final servis = SahteGuncelleyici()..sonuc = yeni;
    await tester.pumpWidget(
      MaterialApp(
        routes: {
          '/ayarlar': (_) => const Scaffold(body: Text('Guncelleme ayarlari')),
        },
        home: Scaffold(
          appBar: AppBar(
            actions: [GuncelleKarti(servis: servis, ayarDugmesi: true)],
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    await tester.tap(find.widgetWithText(TextButton, 'Güncelle'));
    await tester.pumpAndSettle();
    expect(find.text('Guncelleme ayarlari'), findsOneWidget);
    await tester.pumpWidget(const SizedBox.shrink());
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
    expect(
      RegExp(r'\$source = Paket-Kaynagi').allMatches(windowsKurulum).length,
      2,
    );
    expect(windowsKurulum, contains('UnauthorizedAccessException'));
    expect(windowsKurulum, contains('exit 3'));
  });
  test('Prepare gizli pencere argumanlarini kullanir', () {
    expect(windowsPrepareArgumanlari('kur.ps1'), [
      '-NoProfile',
      '-NonInteractive',
      '-WindowStyle',
      'Hidden',
      '-ExecutionPolicy',
      'Bypass',
      '-File',
      'kur.ps1',
      '-Prepare',
    ]);
  });
  testWidgets('Yeni sürüm şeridi Ayarlar açar', (tester) async {
    final servis = SahteGuncelleyici()..sonuc = yeni;
    await tester.pumpWidget(
      MaterialApp(
        routes: {'/ayarlar': (_) => AyarlarSayfasi(servis: servis)},
        home: Scaffold(body: GuncelleKarti(servis: servis)),
      ),
    );
    await tester.pumpAndSettle();
    expect(find.byKey(const Key('guncelle_karti')), findsOneWidget);
    expect(find.byKey(const Key('guncelle_dugme')), findsNothing);
    await tester.tap(find.text('Yeni sürüm hazır — dokun'));
    await tester.pumpAndSettle();
    expect(find.byType(AyarlarSayfasi), findsOneWidget);
    expect(find.text('Güncelle'), findsOneWidget);
    await tester.pumpWidget(const SizedBox.shrink());
    final sayi = servis.kontroller;
    guncellemeKontrolIstegi.value++;
    expect(servis.kontroller, sayi);
  });
  testWidgets('Yeni surum yoksa kart yer kaplamaz', (tester) async {
    await tester.pumpWidget(
      MaterialApp(home: GuncelleKarti(servis: SahteGuncelleyici())),
    );
    await tester.pumpAndSettle();
    expect(find.byKey(const Key('guncelle_karti')), findsNothing);
    await tester.pumpWidget(const SizedBox.shrink());
  });
  testWidgets('Ayarlar denetimi ana ekrandaki şeridi de yeniler', (
    tester,
  ) async {
    final servis = SahteGuncelleyici();
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(body: GuncelleKarti(servis: servis)),
      ),
    );
    await tester.pumpAndSettle();
    expect(find.byKey(const Key('guncelle_karti')), findsNothing);
    servis.sonuc = yeni;
    await servis.denetle();
    await tester.pumpAndSettle();
    expect(find.text('Yeni sürüm hazır — dokun'), findsOneWidget);
    servis.sonuc = null;
    await servis.denetle();
    await tester.pumpAndSettle();
    expect(find.byKey(const Key('guncelle_karti')), findsNothing);
    await tester.pumpWidget(const SizedBox.shrink());
  });
  testWidgets(
    'Giriş ekranındaki dişli güncelleme olmasa da görünür ve denetler',
    (tester) async {
      final servis = SahteGuncelleyici();
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            appBar: AppBar(
              actions: [GuncelleKarti(servis: servis, ayarDugmesi: true)],
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      expect(find.byTooltip('Ayarlar'), findsOneWidget);
      expect(servis.kontroller, 1);
      await tester.pump(const Duration(days: 1));
      await tester.pumpAndSettle();
      expect(servis.kontroller, 2);
      await tester.pumpWidget(const SizedBox.shrink());
    },
  );
  testWidgets('Izin yonlendirmesi ve genel hata mesaji', (tester) async {
    final servis = SahteGuncelleyici()
      ..sonuc = yeni
      ..hata = const GuncellemeSayfasiAcildi();
    await servis.denetle();
    await tester.pumpWidget(MaterialApp(home: AyarlarSayfasi(servis: servis)));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Güncelle'));
    await tester.pumpAndSettle();
    expect(
      find.text('Güncelleme sayfası açıldı, yeni sürümü oradan indir.'),
      findsOneWidget,
    );
    servis.hata = Exception('hata');
    await tester.tap(find.text('Güncelle'));
    await tester.pumpAndSettle();
    expect(
      find.text('Güncelleme tamamlanamadı, yeniden dene.'),
      findsOneWidget,
    );
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
