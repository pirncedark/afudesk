import 'package:afudesk/motor.dart';
import 'package:afudesk/sayfalar/oturum.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'sahte_motor.dart';

void main() {
  testWidgets('eski host: bas konus guncelleme uyarisini bir kez gosterir', (
    t,
  ) async {
    final motor = SahteMotor()..sesHostEski = true;
    await t.pumpWidget(MaterialApp(home: OturumSayfasi(motor: motor)));
    motor.izleyici.add(IzleyiciOlay('kabul', kontrol: true));
    await t.pump();
    await t.tap(find.byKey(const Key('oturum_terminal')));
    motor.izleyici.add(
      IzleyiciOlay(
        'terminal_liste',
        metin: '[{"id":42,"baslik":"Kod","uygulama":"PowerShell"}]',
      ),
    );
    await t.pump();
    motor.izleyici.add(IzleyiciOlay('terminal_durum', metin: 'acik'));
    await t.pump();
    await t.pump(const Duration(seconds: 4));
    const uyari = "PC'deki AfuDesk eski. Güncellemek için PC'de AfuDesk'i aç.";
    for (var i = 0; i < 2; i++) {
      final basma = await t.startGesture(
        t.getCenter(find.byKey(const Key('terminal_konusma_modu'))),
      );
      await t.pump(const Duration(milliseconds: 100));
      await basma.up();
      await t.pump();
      expect(find.text(uyari), i == 0 ? findsOneWidget : findsNothing);
      // Kuyruktaki tüm bildirimleri kapat; ikinci basışta yeniden çıkmamalı.
      ScaffoldMessenger.of(
        t.element(find.byType(OturumSayfasi)),
      ).clearSnackBars();
      await t.pump(const Duration(seconds: 1));
    }
    expect(
      motor.girdiler.where((g) => g.tur == 'ses_ac' || g.tur == 'ses'),
      isEmpty,
    );
    await t.pumpWidget(const SizedBox.shrink());
  });
}
