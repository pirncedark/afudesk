import 'package:afudesk/motor.dart';
import 'package:afudesk/sayfalar/oturum.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';

import 'sahte_motor.dart';

void main() {
  testWidgets('Google akisi terminali kodla tekrar baglanmadan acar', (
    t,
  ) async {
    final m = SahteMotor();
    var acilis = 0;
    await t.pumpWidget(
      MaterialApp(
        home: OturumSayfasi(
          motor: m,
          terminalleBasla: true,
          baglantiAc: () {
            acilis++;
            return m.izleyici.stream;
          },
        ),
      ),
    );
    expect(m.girdiler, isEmpty);
    m.izleyici.add(IzleyiciOlay('kabul', kontrol: true));
    await t.pump();
    expect(m.girdiler.last.tur, 'terminal_liste');
    m.izleyici.add(
      IzleyiciOlay(
        'terminal_liste',
        metin: '[{"id":42,"baslik":"Proje","uygulama":"Windows Terminal"}]',
      ),
    );
    await t.pump();
    expect(m.girdiler.last.tur, 'terminal_ac');
    m.izleyici.add(IzleyiciOlay('terminal_durum', metin: 'acik'));
    await t.pump();
    expect(find.byKey(const Key('terminal_modu')), findsOneWidget);
    expect(acilis, 1);
    expect(m.cagrilar.where((s) => s == 'baglan'), isEmpty);
    await t.tap(find.byTooltip('Masaüstüne dön'));
    await t.pump();
    m.izleyici.add(IzleyiciOlay('kabul', kontrol: true));
    await t.pump();
    expect(m.girdiler.last.tur, 'terminal_kapat');
    await t.pumpWidget(const SizedBox.shrink());
  });

  Future<SahteMotor> baglan(WidgetTester t, {bool kontrol = true}) async {
    final m = SahteMotor();
    await t.pumpWidget(MaterialApp(home: OturumSayfasi(motor: m)));
    m.izleyici.add(IzleyiciOlay('kabul', kontrol: kontrol));
    await t.pump();
    return m;
  }

  testWidgets('tek terminal host onayindan sonra ayni oturumda acilir', (
    t,
  ) async {
    final m = await baglan(t);
    await t.tap(find.byKey(const Key('oturum_terminal')));
    expect(m.girdiler.last.tur, 'terminal_liste');
    m.izleyici.add(
      IzleyiciOlay(
        'terminal_liste',
        metin: '[{"id":42,"baslik":"Kod projesi","uygulama":"PowerShell"}]',
      ),
    );
    await t.pump();
    expect(m.girdiler.last.tur, 'terminal_ac');
    expect(m.girdiler.last.metin, '42');
    expect(find.byKey(const Key('terminal_modu')), findsNothing);
    m.izleyici.add(IzleyiciOlay('terminal_durum', metin: 'acik'));
    await t.pump();
    expect(find.byKey(const Key('terminal_modu')), findsOneWidget);
    await t.pump(const Duration(seconds: 4));
    await t.ensureVisible(find.byKey(const Key('terminal_ctrl_c')));
    await t.tap(find.byKey(const Key('terminal_ctrl_c')));
    expect(m.girdiler.where((g) => g.ad == 'Ctrl').map((g) => g.basili), [
      true,
      false,
    ]);
    await t.tap(find.byTooltip('Masaüstüne dön'));
    await t.pump();
    expect(m.girdiler.last.tur, 'terminal_kapat');
    expect(find.byKey(const Key('terminal_modu')), findsNothing);
    expect(m.cagrilar.where((s) => s == 'baglan').length, 1);
    await t.pumpWidget(const SizedBox.shrink());
  });

  Future<SahteMotor> terminalAc(WidgetTester t) async {
    final m = await baglan(t);
    await t.tap(find.byKey(const Key('oturum_terminal')));
    m.izleyici.add(
      IzleyiciOlay(
        'terminal_liste',
        metin: '[{"id":42,"baslik":"Kod","uygulama":"PowerShell"}]',
      ),
    );
    await t.pump();
    m.izleyici.add(IzleyiciOlay('terminal_durum', metin: 'acik'));
    await t.pump();
    await t.pump(const Duration(seconds: 4));
    m.girdiler.clear();
    return m;
  }

  testWidgets('IME composing commit ve Turkce duzeltme kaybolmaz', (t) async {
    final m = await terminalAc(t);
    await t.tap(find.byKey(const Key('terminal_klavye')));
    await t.pump();
    await t.showKeyboard(find.byKey(const Key('terminal_yazi')));
    void ime(String text, {bool composing = false}) {
      t.testTextInput.updateEditingValue(
        TextEditingValue(
          text: text,
          selection: TextSelection.collapsed(offset: text.length),
          composing: composing
              ? TextRange(start: 0, end: text.length)
              : TextRange.empty,
        ),
      );
    }

    ime('cag', composing: true);
    ime('çağrı', composing: true);
    ime('çağrı');
    ime('çağ');
    ime('çağ İstanbul');
    var host = '';
    for (final g in m.girdiler) {
      if (g.tur == 'metin') host += g.metin;
      if (g.ad == 'Backspace' && g.basili) {
        host = String.fromCharCodes(host.runes.take(host.runes.length - 1));
      }
    }
    expect(host, 'çağ İstanbul');
    expect(
      m.girdiler.where((g) => g.ad == 'Backspace' && g.basili),
      isNotEmpty,
    );
    await t.pumpWidget(const SizedBox.shrink());
  });

  testWidgets('Space normal klavye tusu olarak aninda iletilir', (t) async {
    final m = await terminalAc(t);
    await t.sendKeyDownEvent(LogicalKeyboardKey.space);
    expect(m.girdiler.map((g) => g.basili), [true]);
    await t.sendKeyUpEvent(LogicalKeyboardKey.space);
    expect(m.girdiler.map((g) => g.basili), [true, false]);
    expect(m.girdiler.every((g) => g.ad == 'Space'), isTrue);
    await t.pumpWidget(const SizedBox.shrink());
  });

  testWidgets('Bas konus birakma ve iptalde Space serbest kalir', (t) async {
    final m = await terminalAc(t);
    final b = find.byKey(const Key('terminal_bas_konus'));
    final gesture = await t.startGesture(t.getCenter(b));
    await t.pump(const Duration(seconds: 1));
    expect(m.girdiler.map((g) => g.basili), [true]);
    await gesture.cancel();
    expect(m.girdiler.map((g) => g.basili), [true, false]);
    m.girdiler.clear();
    final g2 = await t.startGesture(t.getCenter(b));
    await g2.up();
    expect(m.girdiler.map((g) => g.basili), [true, false]);
    await t.pumpWidget(const SizedBox.shrink());
  });

  testWidgets('izleme izniyle terminal dugmesi kapali', (t) async {
    await baglan(t, kontrol: false);
    expect(
      t.widget<IconButton>(find.byKey(const Key('oturum_terminal'))).onPressed,
      isNull,
    );
    await t.pumpWidget(const SizedBox.shrink());
  });

  testWidgets('terminal listesi secim ve baglanti kopusunu yonetir', (t) async {
    final m = await baglan(t);
    await t.tap(find.byKey(const Key('oturum_terminal')));
    m.izleyici.add(
      IzleyiciOlay(
        'terminal_liste',
        metin:
            '[{"id":1,"baslik":"Bir","uygulama":"PowerShell"},'
            '{"id":2,"baslik":"Iki","uygulama":"Windows Terminal"}]',
      ),
    );
    // İlk kare henüz yok: yükleme göstergesi sürekli animasyon yapar.
    await t.pump();
    await t.pump(const Duration(milliseconds: 300));
    await t.tap(find.text('Iki'));
    await t.pump();
    await t.pump(const Duration(milliseconds: 300));
    expect(m.girdiler.last.metin, '2');
    m.izleyici.add(IzleyiciOlay('terminal_durum', metin: 'acik'));
    await t.pump();
    m.izleyici.add(IzleyiciOlay('koptu', metin: 'Bağlantı kapandı.'));
    await t.pump();
    expect(find.byKey(const Key('terminal_modu')), findsNothing);
    expect(find.text('Bağlantı kapandı.'), findsOneWidget);
    await t.pumpWidget(const SizedBox.shrink());
  });

  testWidgets('bos liste aciklama gosterir ve pencere acmaz', (t) async {
    final m = await baglan(t);
    await t.tap(find.byKey(const Key('oturum_terminal')));
    m.izleyici.add(IzleyiciOlay('terminal_liste', metin: '[]'));
    await t.pump();
    expect(
      find.text('Açık terminal bulunamadı. Bilgisayarda terminal aç.'),
      findsOneWidget,
    );
    expect(m.girdiler.where((g) => g.tur == 'terminal_ac'), isEmpty);
    await t.pumpWidget(const SizedBox.shrink());
  });

  testWidgets('geciken host onayi kapatilir', (t) async {
    final m = await baglan(t);
    await t.tap(find.byKey(const Key('oturum_terminal')));
    m.izleyici.add(
      IzleyiciOlay(
        'terminal_liste',
        metin: '[{"id":42,"baslik":"Kod","uygulama":"PowerShell"}]',
      ),
    );
    await t.pump();
    await t.pump(const Duration(seconds: 11));
    expect(m.girdiler.last.tur, 'terminal_kapat');
    m.izleyici.add(IzleyiciOlay('terminal_durum', metin: 'acik'));
    await t.pump();
    expect(find.byKey(const Key('terminal_modu')), findsNothing);
    expect(m.girdiler.last.tur, 'terminal_kapat');
    await t.pumpWidget(const SizedBox.shrink());
  });
}
