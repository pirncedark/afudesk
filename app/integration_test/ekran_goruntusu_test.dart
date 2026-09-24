// Görsel UX kontrolü: her ekranı gerçek Windows motorunda (gerçek fontlarla) çizip
// docs/ekranlar/ altına PNG olarak kaydeder. Sahte motor kullanır (ağ gerekmez).
import 'dart:io';
import 'dart:typed_data';
import 'dart:ui' as ui;

import 'package:afudesk/main.dart';
import 'package:afudesk/motor.dart';
import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';

import '../test/sahte_motor.dart';

final _kok = GlobalKey();

Future<void> kaydet(WidgetTester t, String ad) async {
  await t.pump(const Duration(milliseconds: 400));
  late Uint8List png;
  await t.runAsync(() async {
    final sinir = _kok.currentContext!.findRenderObject()! as RenderRepaintBoundary;
    final img = await sinir.toImage(pixelRatio: 1);
    png = (await img.toByteData(format: ui.ImageByteFormat.png))!.buffer.asUint8List();
  });
  final dosya = File('../docs/ekranlar/$ad.png')..createSync(recursive: true);
  dosya.writeAsBytesSync(png);
  debugPrint('EKRAN ${dosya.absolute.path}');
}

Future<SahteMotor> ac(WidgetTester t, Size boyut) async {
  await t.binding.setSurfaceSize(boyut);
  final m = SahteMotor();
  await t.pumpWidget(RepaintBoundary(key: _kok, child: AfuDeskUygulama(key: UniqueKey(), motor: m)));
  await t.pump();
  return m;
}

Uint8List desen(int g, int y) {
  final b = Uint8List(g * y * 4);
  for (var i = 0; i < g * y; i++) {
    final x = i % g, yy = i ~/ g;
    b[i * 4] = 30 + (x * 60 ~/ g);
    b[i * 4 + 1] = 60 + (yy * 90 ~/ y);
    b[i * 4 + 2] = 140;
    b[i * 4 + 3] = 255;
  }
  return b;
}

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  testWidgets('ekran görüntüleri', (t) async {
    // 1) Ana ekran (masaüstü ve telefon genişliği)
    var m = await ac(t, const Size(1000, 700));
    await kaydet(t, '1_ana_masaustu');
    m = await ac(t, const Size(390, 800));
    await kaydet(t, '2_ana_telefon');

    // 2) Bağlantı ver: kod + parola
    m = await ac(t, const Size(1000, 760));
    await t.tap(find.byKey(const Key('secenek_ver')));
    await t.pump(const Duration(milliseconds: 500));
    m.host.add(const HostOlay('hazir',
        kod: 'AFU2.q8Xv3Lk0sP2aN9dF7hJ1mZ4cW6bT5yR8uE3iO0pA2sD4fG6hJ8kL0zX2cV4bN6mQ8wE0rT2yU4iO6pA8sD0fG2hJ4kL6zX8cV0bN2mQ4wE6rT8yU0iO2pA4sD6fG8hJ0kL2zX4cV6bN8mQ0wE2rT4yU6iO8pA0sD2fG4hJ6kL8zX0cV2bN4mQ6wE8rT0yU2iO4pA6sD8fG0',
        parola: '482913',
        erisim: 'Yalnız aynı ağdan ulaşılabilir — modemde UPnP kapalı'));
    await kaydet(t, '3_baglanti_ver');

    // 3) Gelen istek penceresi
    m.host.add(const HostOlay('istek', ad: 'VELI-LAPTOP'));
    await t.pump(const Duration(milliseconds: 500));
    await kaydet(t, '4_istek_penceresi');
    await t.tap(find.byKey(const Key('istek_kabul')));
    await t.pump(const Duration(milliseconds: 500));
    m.host.add(const HostOlay('baglandi', ad: 'VELI-LAPTOP', kontrol: true));
    await kaydet(t, '5_baglanti_verildi');

    // 4) Bağlan ekranı (hatalı giriş dahil)
    m = await ac(t, const Size(1000, 700));
    await t.tap(find.byKey(const Key('secenek_baglan')));
    await t.pump(const Duration(milliseconds: 500));
    await t.tap(find.byKey(const Key('baglan_dugme')));
    await kaydet(t, '6_baglan_bos_uyari');

    // 5) Oturum: onay bekleniyor + görüntü
    await t.enterText(find.byKey(const Key('baglan_kod')), 'AFU2.abc');
    await t.enterText(find.byKey(const Key('baglan_parola')), '482913');
    await t.tap(find.byKey(const Key('baglan_dugme')));
    await t.pump(const Duration(milliseconds: 500));
    m.izleyici.add(IzleyiciOlay('bekliyor', ad: 'ALI-PC'));
    await kaydet(t, '7_onay_bekleniyor');
    m.izleyici.add(IzleyiciOlay('kabul', kontrol: true, genislik: 1280, yukseklik: 720));
    await t.pump();
    await t.runAsync(() async => m.izleyici.add(IzleyiciOlay('kare', genislik: 1280, yukseklik: 720, rgba: desen(1280, 720))));
    for (var i = 0; i < 100 && find.byKey(const Key('oturum_kare')).evaluate().isEmpty; i++) {
      await t.runAsync(() => Future<void>.delayed(const Duration(milliseconds: 20)));
      await t.pump();
    }
    await kaydet(t, '8_oturum');

    // 6) Hata ekranı
    m.izleyici.add(IzleyiciOlay('koptu', metin: 'Karşı tarafa ulaşılamadı. Aynı ağda değilseniz, bağlantı veren tarafın modeminde UPnP açık olmalı.'));
    await kaydet(t, '9_oturum_hata');
    // 7) Telefon: ana ekran (ekran paylaşımı yakında) ve dokunmatik oturum
    await t.binding.setSurfaceSize(const Size(400, 820));
    m = SahteMotor()
      ..dokunmatik = true
      ..baglantiVerilebilir = false;
    await t.pumpWidget(RepaintBoundary(key: _kok, child: AfuDeskUygulama(key: UniqueKey(), motor: m)));
    await t.pump();
    await kaydet(t, '10_telefon_ana');
    await t.tap(find.byKey(const Key('secenek_baglan')));
    await t.pump(const Duration(milliseconds: 500));
    await t.enterText(find.byKey(const Key('baglan_kod')), 'AFU2.abc');
    await t.enterText(find.byKey(const Key('baglan_parola')), '482913');
    await t.tap(find.byKey(const Key('baglan_dugme')));
    await t.pump(const Duration(milliseconds: 500));
    m.izleyici.add(IzleyiciOlay('bekliyor', ad: 'ALI-PC'));
    m.izleyici.add(IzleyiciOlay('kabul', kontrol: true, genislik: 1280, yukseklik: 720));
    await t.pump();
    await t.runAsync(() async => m.izleyici.add(IzleyiciOlay('kare', genislik: 1280, yukseklik: 720, rgba: desen(1280, 720))));
    for (var i = 0; i < 100 && find.byKey(const Key('oturum_kare')).evaluate().isEmpty; i++) {
      await t.runAsync(() => Future<void>.delayed(const Duration(milliseconds: 20)));
      await t.pump();
    }
    await t.tapAt(t.getCenter(find.byKey(const Key('oturum_ekran'))));
    await kaydet(t, '11_telefon_oturum');
  });
}
