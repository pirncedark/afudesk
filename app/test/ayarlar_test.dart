import 'package:afudesk/guncelleme.dart';
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

  testWidgets('sürüm, güncel durumu ve son denetim gösterilir', (t) async {
    final servis = AyarlarGuncelleyici();
    await t.pumpWidget(MaterialApp(home: AyarlarSayfasi(servis: servis)));
    await t.pumpAndSettle();
    expect(find.text('1.9.1+11'), findsOneWidget);
    expect(find.text('Son denetim'), findsNothing);
    await t.tap(find.text('Güncellemeleri denetle'));
    await t.pumpAndSettle();
    expect(find.text('Güncelsin'), findsOneWidget);
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
      expect(find.text('1.9.1+11'), findsOneWidget);
      await t.pumpWidget(const SizedBox.shrink());
      hesap?.dispose();
    });
  }
}
