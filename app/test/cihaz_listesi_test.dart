import 'package:afudesk/motor.dart';
import 'package:afudesk/sayfalar/ana.dart';
import 'package:afudesk/sayfalar/baglanti_ver.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'sahte_motor.dart';

void main() {
  for (final guvenilen in [false, true]) {
    final on = guvenilen ? 'guvenilen' : 'kayitli';
    testWidgets('$on ad değiştirme, silme ve Açık göstergesi', (t) async {
      t.view.physicalSize = const Size(1200, 1000);
      t.view.devicePixelRatio = 1;
      addTearDown(t.view.reset);
      final m = SahteMotor();
      final liste = guvenilen ? m.guvenilen : m.kayitli;
      liste.add(KayitliCihaz('a', 'PC', DateTime.now(), acik: true));
      liste.add(KayitliCihaz('b', 'Kapalı PC', DateTime.now()));
      await t.pumpWidget(
        MaterialApp(
          home: guvenilen ? BaglantiVerSayfasi(motor: m) : AnaSayfa(motor: m),
        ),
      );
      await t.pump();
      if (guvenilen) {
        m.host.add(const HostOlay('hazir', kod: 'kod', parola: 'parola'));
        await t.pump();
      }
      expect(find.byKey(Key('${on}_acik_0')), findsOneWidget);
      expect(find.byKey(Key('${on}_acik_1')), findsNothing);
      expect(find.text('Açık'), findsOneWidget);
      expect(find.textContaining('Son bağlanma:'), findsNWidgets(2));
      final kalem = find.byKey(Key('${on}_adla_0'));
      await t.ensureVisible(kalem);
      await t.tap(kalem);
      await t.pumpAndSettle();
      expect(find.byType(AlertDialog), findsOneWidget);
      expect(t.widget<TextField>(find.byType(TextField)).maxLength, 40);
      await t.enterText(find.byType(TextField), 'Çalışma bilgisayarı');
      await t.tap(find.text('Kaydet'));
      await t.pumpAndSettle();
      expect(find.text('Çalışma bilgisayarı'), findsOneWidget);
      expect(find.textContaining(' · PC'), findsOneWidget);
      await t.tap(kalem);
      await t.pumpAndSettle();
      await t.enterText(find.byType(TextField), '   ');
      await t.tap(find.text('Kaydet'));
      await t.pumpAndSettle();
      expect(find.text('PC'), findsOneWidget);
      expect(find.textContaining(' · PC'), findsNothing);
      m.acikKimlikler.clear();
      liste[0] = KayitliCihaz('a', 'PC', DateTime.now());
      await t.pump(const Duration(seconds: 15));
      await t.pump();
      expect(find.byKey(Key('${on}_acik_0')), findsNothing);
      await t.pumpWidget(const SizedBox());
      await t.pump(const Duration(seconds: 1));
      await m.host.close();
      await m.izleyici.close();
    });
  }
}
