import 'package:afudesk/motor.dart';
import 'package:afudesk/sayfalar/oturum.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'sahte_motor.dart';

void main() {
  for (final dikey in [true, false]) {
    testWidgets('terminal secici $dikey', (t) async {
      t.view.physicalSize = dikey ? const Size(390, 844) : const Size(844, 390);
      t.view.devicePixelRatio = 1;
      addTearDown(t.view.resetPhysicalSize);
      addTearDown(t.view.resetDevicePixelRatio);
      final m = SahteMotor();
      await t.pumpWidget(MaterialApp(home: OturumSayfasi(motor: m)));
      m.izleyici.add(IzleyiciOlay('kabul', kontrol: true));
      await t.pump();
      await t.tap(find.byKey(const Key('oturum_terminal')));
      m.izleyici.add(
        IzleyiciOlay(
          'terminal_liste',
          metin:
              '[{"id":1,"baslik":"Bir","uygulama":"PowerShell"},{"id":2,"baslik":"Iki","uygulama":"PowerShell"}]',
        ),
      );
      await t.pump();
      await t.pump(const Duration(milliseconds: 300));
      await t.tap(find.text('Bir'));
      await t.pump();
      await t.pump(const Duration(milliseconds: 300));
      m.izleyici.add(IzleyiciOlay('terminal_durum', metin: 'acik'));
      await t.pump();
      final secici = find.byKey(const Key('terminal_secici'));
      expect(secici, dikey ? findsOneWidget : findsNothing);
      if (dikey) {
        await t.tap(secici);
        await t.pump();
        await t.pump(const Duration(milliseconds: 300));
        await t.tap(find.text('Iki').last);
        await t.pump();
        expect(m.girdiler.last.tur, 'terminal_ac');
        expect(m.girdiler.last.metin, '2');
        expect(m.girdiler.last.dx, 390);
        expect(m.girdiler.last.dy, 844);
      }
      expect(m.cagrilar.where((c) => c == 'baglan'), hasLength(1));
      await t.pumpWidget(const SizedBox.shrink());
    });
  }
}
