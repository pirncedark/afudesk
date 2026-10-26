import 'package:afudesk/guncelleme.dart';
import 'package:afudesk/aktif_sayac.dart';
import 'package:afudesk/main.dart';
import 'package:afudesk/sayfalar/ayarlar.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'sahte_hesap_servisi.dart';
import 'sahte_motor.dart';

class AyarlarGuncelleyici extends Guncelleyici {
  Guncelleme? sonuc;
  Object? hata;
  int kurulumlar = 0;
  @override
  Future<String> surum() async => '1.9.1+11';
  @override
  Future<Guncelleme?> kontrol() async {
    if (hata != null) throw hata!;
    return sonuc;
  }

  @override
  Future<void> kur(Guncelleme g, void Function(double) ilerleme) async {
    kurulumlar++;
    ilerleme(1);
    if (hata != null) throw hata!;
  }
}

void main() {
  final yeni = Guncelleme(
    '1.9.2',
    Uri.parse('https://github.com/paket'),
    Uri.parse('https://github.com/ozet'),
    100,
  );

  testWidgets(
    'üç standart kart görünür, Windows paylaşımında APK seçeneği yok',
    (t) async {
      await t.pumpWidget(
        MaterialApp(home: AyarlarSayfasi(servis: AyarlarGuncelleyici())),
      );
      await t.pumpAndSettle();
      for (final baslik in ['Güncelleme', 'Paylaş', 'Topluluk']) {
        expect(find.text(baslik), findsOneWidget);
      }
      expect(find.text('APK gönder'), findsNothing);
    },
  );

  test(
    'sayaç aynı gün ikinci ping göndermez; kapalıyken ağ isteği yapmaz',
    () async {
      final store = <String, String>{};
      final calls = <String>[];
      Future<Map<String, dynamic>> fake(
        String method,
        Uri uri, [
        Map<String, dynamic>? body,
      ]) async {
        calls.add(method);
        return {'aktif7': 12, 'aktif1': 3, 'toplam': 20};
      }

      Future<String?> oku(String key) async => store[key];
      Future<void> yaz(String key, String value) async {
        store[key] = value;
      }

      final sayac = AktifSayac(
        istek: fake,
        oku: oku,
        yaz: yaz,
        simdi: () => DateTime(2026, 10, 26),
      );
      await sayac.yukle(katil: false, surum: '1.0');
      expect(calls, isEmpty);
      await sayac.yukle(katil: true, surum: '1.0');
      await sayac.yukle(katil: true, surum: '1.0');
      expect(calls.where((m) => m == 'POST').length, 1);
      expect(calls.where((m) => m == 'GET').length, 2);
      expect(
        store['sayacKimlik'],
        matches(
          RegExp(
            r'^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$',
          ),
        ),
      );
    },
  );

  testWidgets('sürüm, güncel durumu ve son denetim gösterilir', (t) async {
    final servis = AyarlarGuncelleyici();
    await t.pumpWidget(MaterialApp(home: AyarlarSayfasi(servis: servis)));
    await t.pumpAndSettle();
    expect(find.text('Sürüm 1.9.1+11'), findsOneWidget);
    expect(find.text('Son denetim'), findsNothing);
    await t.tap(find.text('Güncellemeleri denetle'));
    await t.pumpAndSettle();
    expect(find.text('Güncelsin'), findsOneWidget);
    await t.scrollUntilVisible(find.text('Son denetim'), 200);
    expect(find.text('Son denetim'), findsOneWidget);
  });

  testWidgets('yeni sürüm mevcut servisle kurulur ve hata sade gösterilir', (
    t,
  ) async {
    final servis = AyarlarGuncelleyici()..sonuc = yeni;
    await t.pumpWidget(MaterialApp(home: AyarlarSayfasi(servis: servis)));
    await t.pumpAndSettle();
    await t.tap(find.text('Güncellemeleri denetle'));
    await t.pumpAndSettle();
    expect(find.text('Yeni sürüm var: 1.9.2'), findsOneWidget);
    await t.tap(find.text('Güncelle'));
    await t.pumpAndSettle();
    expect(servis.kurulumlar, 1);
    servis.hata = Exception('teknik ayrıntı');
    await t.tap(find.text('Güncelle'));
    await t.pumpAndSettle();
    expect(
      find.text('Güncelleme tamamlanamadı, yeniden dene.'),
      findsOneWidget,
    );
    expect(find.textContaining('teknik ayrıntı'), findsNothing);
  });

  testWidgets('denetim hatası güncel diye gösterilmez ve yeniden denenebilir', (
    t,
  ) async {
    final servis = AyarlarGuncelleyici()..hata = Exception('ağ');
    await t.pumpWidget(MaterialApp(home: AyarlarSayfasi(servis: servis)));
    await t.pumpAndSettle();
    await t.tap(find.text('Güncellemeleri denetle'));
    await t.pumpAndSettle();
    expect(
      find.text('Güncellemeler denetlenemedi, yeniden dene.'),
      findsOneWidget,
    );
    expect(find.text('Güncelsin'), findsNothing);
    servis.hata = null;
    await t.tap(find.text('Güncellemeleri denetle'));
    await t.pumpAndSettle();
    expect(find.text('Güncelsin'), findsOneWidget);
  });

  testWidgets('hesap e-postası ve çıkış mevcut servisi kullanır', (t) async {
    final hesap = SahteHesapServisi();
    await hesap.girisYap();
    await t.pumpWidget(
      MaterialApp(
        home: AyarlarSayfasi(
          servis: AyarlarGuncelleyici(),
          hesapServisi: hesap,
        ),
      ),
    );
    await t.pumpAndSettle();
    // Hesap bölümü kartların altında; ekran dışında kalabilir.
    await t.scrollUntilVisible(find.text('Çıkış yap'), 200);
    expect(find.text('onizleme@example.com'), findsOneWidget);
    await t.tap(find.text('Çıkış yap'));
    await t.pumpAndSettle();
    expect(hesap.hesap, isNull);
    expect(find.text('onizleme@example.com'), findsNothing);
    await t.pumpWidget(const SizedBox.shrink());
    hesap.dispose();
  });

  for (final hesapli in [false, true]) {
    testWidgets('${hesapli ? "giriş" : "ana"} ekranındaki dişli Ayarlar açar', (
      t,
    ) async {
      t.binding.defaultBinaryMessenger.setMockMethodCallHandler(
        guncelleKanali,
        (call) async => call.method == 'surum' ? '1.9.1+11' : null,
      );
      addTearDown(
        () => t.binding.defaultBinaryMessenger.setMockMethodCallHandler(
          guncelleKanali,
          null,
        ),
      );
      final hesap = hesapli ? SahteHesapServisi() : null;
      await t.pumpWidget(
        AfuDeskUygulama(motor: SahteMotor(), hesapServisi: hesap),
      );
      await t.tap(find.byTooltip('Ayarlar'));
      await t.pumpAndSettle();
      expect(find.byType(AyarlarSayfasi), findsOneWidget);
      expect(find.text('Sürüm 1.9.1+11'), findsOneWidget);
      await t.pumpWidget(const SizedBox.shrink());
      hesap?.dispose();
    });
  }
}
