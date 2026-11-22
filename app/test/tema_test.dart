import 'dart:math' as math;

import 'package:afudesk/main.dart';
import 'package:afudesk/tema.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'sahte_motor.dart';

double _bagilParlaklik(Color c) {
  double kanal(double v) =>
      v <= 0.03928 ? v / 12.92 : math.pow((v + 0.055) / 1.055, 2.4).toDouble();
  return 0.2126 * kanal(c.r) + 0.7152 * kanal(c.g) + 0.0722 * kanal(c.b);
}

double _kontrast(Color a, Color b) {
  final la = _bagilParlaklik(a);
  final lb = _bagilParlaklik(b);
  return (math.max(la, lb) + 0.05) / (math.min(la, lb) + 0.05);
}

void main() {
  test('Gold Glass açık ve koyu paletler AA kontrastı sağlar', () {
    for (final p in [GoldGlassPalet.acik, GoldGlassPalet.koyu]) {
      expect(_kontrast(p.metin, p.zemin), greaterThanOrEqualTo(4.5));
      expect(_kontrast(p.metin, p.katiYuzey), greaterThanOrEqualTo(4.5));
      expect(_kontrast(p.soluk, p.zemin), greaterThanOrEqualTo(4.5));
      expect(_kontrast(p.vurgu, p.zemin), greaterThanOrEqualTo(4.5));
      expect(_kontrast(p.vurguMetin, p.vurgu), greaterThanOrEqualTo(4.5));
      expect(_kontrast(p.basari, p.zemin), greaterThanOrEqualTo(4.5));
      expect(_kontrast(p.tehlike, p.zemin), greaterThanOrEqualTo(4.5));
    }
    expect(Renk.altinBirincil, const Color(0xFFB89452));
    expect(Renk.altinAcik, const Color(0xFFDCC28A));
    expect(Renk.altinYumusak, const Color(0xFFF2E5C9));
    expect(GoldGlassPalet.acik.zemin, const Color(0xFFF5F5F7));
    expect(GoldGlassPalet.koyu.zemin, const Color(0xFF141518));
  });

  testWidgets('sistem açık/koyu temaya uyar ve dokunma hedefleri >= 48dp', (t) async {
    t.view.physicalSize = const Size(1200, 800);
    t.view.devicePixelRatio = 1;
    addTearDown(t.view.reset);

    for (final b in [Brightness.light, Brightness.dark]) {
      t.platformDispatcher.platformBrightnessTestValue = b;
      addTearDown(t.platformDispatcher.clearPlatformBrightnessTestValue);
      await t.pumpWidget(AfuDeskUygulama(key: ValueKey(b), motor: SahteMotor()));
      await t.pump(const Duration(milliseconds: 300));
      final ctx = t.element(find.text('AfuDesk'));
      expect(Theme.of(ctx).brightness, b);
      expect(
        GoldGlassPalet.of(ctx).zemin,
        b == Brightness.dark ? GoldGlassPalet.koyu.zemin : GoldGlassPalet.acik.zemin,
      );

      final ayarDugme = t.getSize(find.byTooltip('Ayarlar'));
      expect(ayarDugme.width, greaterThanOrEqualTo(48));
      expect(ayarDugme.height, greaterThanOrEqualTo(48));

      final ver = t.getSize(find.byKey(const Key('secenek_ver')));
      final bag = t.getSize(find.byKey(const Key('secenek_baglan')));
      expect(ver.height, greaterThanOrEqualTo(48));
      expect(bag.height, greaterThanOrEqualTo(48));
    }
  });

  testWidgets('prefers-reduced-motion animasyon süresini sıfırlar', (t) async {
    t.platformDispatcher.accessibilityFeaturesTestValue =
        const FakeAccessibilityFeatures(disableAnimations: true);
    addTearDown(t.platformDispatcher.clearAccessibilityFeaturesTestValue);

    late Duration sure;
    await t.pumpWidget(
      MaterialApp(
        theme: afuAcikTema(),
        home: Builder(
          builder: (ctx) {
            sure = afuAnimasyonSuresi(ctx);
            return const SizedBox.shrink();
          },
        ),
      ),
    );
    expect(sure, Duration.zero);
  });
}
