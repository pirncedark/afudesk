import 'package:afudesk/motor.dart';
import 'package:afudesk/sayfalar/oturum.dart';
import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';

import 'sahte_motor.dart';

Future<SahteMotor> ac(WidgetTester t, {bool dokunmatik = false}) async {
  final m = SahteMotor()..dokunmatik = dokunmatik;
  await t.pumpWidget(MaterialApp(home: OturumSayfasi(motor: m)));
  m.izleyici.add(IzleyiciOlay('kabul', kontrol: true));
  await t.pump();
  await t.pump();
  return m;
}

void main() {
  testWidgets('physical HID and unknown logical key are sent', (t) async {
    final m = await ac(t);
    for (final k in [LogicalKeyboardKey.keyA, LogicalKeyboardKey.printScreen, LogicalKeyboardKey.numpadAdd, LogicalKeyboardKey.f13]) {
      await t.sendKeyEvent(k, platform: 'windows');
    }
    final focus = t.widgetList<Focus>(find.byType(Focus)).firstWhere((f) => f.onKeyEvent != null && f.autofocus);
    focus.onKeyEvent!(focus.focusNode!, const KeyDownEvent(
      physicalKey: PhysicalKeyboardKey.printScreen,
      logicalKey: LogicalKeyboardKey.unidentified,
      timeStamp: Duration.zero,
    ));
    focus.onKeyEvent!(focus.focusNode!, const KeyUpEvent(
      physicalKey: PhysicalKeyboardKey.printScreen,
      logicalKey: LogicalKeyboardKey.unidentified,
      timeStamp: Duration.zero,
    ));
    final g = m.girdiler.where((g) => g.tur == 'tus').toList();
    expect(g, hasLength(10));
    expect(g.map((g) => g.hid), [0x70004, 0x70004, 0x70046, 0x70046, 0x70057, 0x70057, 0x70068, 0x70068, 0x70046, 0x70046]);
  });

  testWidgets('repeat is pressed and rate limited per HID', (t) async {
    final m = await ac(t);
    final focus = t.widgetList<Focus>(find.byType(Focus)).firstWhere((f) => f.onKeyEvent != null && f.autofocus);
    void send(KeyEvent event) => focus.onKeyEvent!(focus.focusNode!, event);
    send(const KeyDownEvent(
      physicalKey: PhysicalKeyboardKey.keyA,
      logicalKey: LogicalKeyboardKey.keyA,
      timeStamp: Duration.zero,
    ));
    send(const KeyRepeatEvent(
      physicalKey: PhysicalKeyboardKey.keyA,
      logicalKey: LogicalKeyboardKey.keyA,
      timeStamp: Duration.zero,
    ));
    send(const KeyRepeatEvent(
      physicalKey: PhysicalKeyboardKey.keyA,
      logicalKey: LogicalKeyboardKey.keyA,
      timeStamp: Duration(milliseconds: 10),
    ));
    expect(m.girdiler.where((g) => g.tur == 'tus' && g.basili), hasLength(2));
    send(const KeyRepeatEvent(
      physicalKey: PhysicalKeyboardKey.keyA,
      logicalKey: LogicalKeyboardKey.keyA,
      timeStamp: Duration(milliseconds: 34),
    ));
    expect(m.girdiler.where((g) => g.tur == 'tus' && g.basili), hasLength(3));
    send(const KeyUpEvent(
      physicalKey: PhysicalKeyboardKey.keyA,
      logicalKey: LogicalKeyboardKey.keyA,
      timeStamp: Duration(milliseconds: 35),
    ));
  });
  testWidgets('focus loss releases Shift and mouse', (t) async {
    final m = await ac(t);
    await t.sendKeyDownEvent(LogicalKeyboardKey.shiftLeft, platform: 'windows');
    final listener = t.widget<Listener>(find.byKey(const Key('oturum_ekran')));
    listener.onPointerDown!(const PointerDownEvent(buttons: kPrimaryMouseButton));
    FocusManager.instance.primaryFocus!.unfocus();
    await t.pump();
    expect(m.girdiler.any((g) => g.ad == 'Shift' && !g.basili), isTrue);
    expect(m.girdiler.any((g) => g.ad == 'sol' && !g.basili), isTrue);
    await t.sendKeyUpEvent(LogicalKeyboardKey.shiftLeft, platform: 'windows');
  });

  testWidgets('dispose releases pressed keys', (t) async {
    final m = await ac(t);
    await t.sendKeyDownEvent(LogicalKeyboardKey.controlLeft, platform: 'windows');
    await t.pumpWidget(const SizedBox());
    expect(m.girdiler.last.ad, 'Ctrl');
    expect(m.girdiler.last.basili, isFalse);
    await t.sendKeyUpEvent(LogicalKeyboardKey.controlLeft, platform: 'windows');
  });

  for (final olay in ['yeniden', 'koptu', 'izin']) {
    testWidgets('$olay releases pressed keys', (t) async {
      final m = await ac(t);
      await t.sendKeyDownEvent(LogicalKeyboardKey.shiftRight, platform: 'windows');
      m.izleyici.add(IzleyiciOlay(olay == 'izin' ? 'kabul' : olay, kontrol: false));
      await t.pump();
      expect(m.girdiler.last.basili, isFalse);
      await t.sendKeyUpEvent(LogicalKeyboardKey.shiftRight, platform: 'windows');
    });
  }

  testWidgets('lifecycle releases pressed keys', (t) async {
    final m = await ac(t);
    await t.sendKeyDownEvent(LogicalKeyboardKey.altLeft, platform: 'windows');
    t.binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
    await t.pump();
    expect(m.girdiler.last.basili, isFalse);
    t.binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
    await t.sendKeyUpEvent(LogicalKeyboardKey.altLeft, platform: 'windows');
  });

  testWidgets('physical keyboard works in touch mode and text panel', (t) async {
    final m = await ac(t, dokunmatik: true);
    await t.sendKeyEvent(LogicalKeyboardKey.keyA, platform: 'windows');
    expect(m.girdiler.where((g) => g.tur == 'tus'), hasLength(2));
    await t.tap(find.byKey(const Key('oturum_klavye')));
    await t.pump();
    await t.sendKeyEvent(LogicalKeyboardKey.keyB, platform: 'windows');
    expect(m.girdiler.where((g) => g.tur == 'tus'), hasLength(4));
    expect(m.girdiler.where((g) => g.tur == 'metin'), isEmpty);
    await t.enterText(find.byKey(const Key('oturum_yazi')), '\u015f');
    expect(m.girdiler.last.metin, '\u015f');
    await t.pumpWidget(const SizedBox());
  });

  testWidgets('two mouse buttons release independently', (t) async {
    final m = await ac(t);
    final l = t.widget<Listener>(find.byKey(const Key('oturum_ekran')));
    l.onPointerDown!(const PointerDownEvent(buttons: kPrimaryMouseButton));
    // Flutter ikinci d??me de?i?imini PointerMove olarak da iletir.
    l.onPointerMove!(const PointerMoveEvent(buttons: kPrimaryMouseButton | kSecondaryMouseButton));
    l.onPointerMove!(const PointerMoveEvent(buttons: kSecondaryMouseButton));
    l.onPointerUp!(const PointerUpEvent());
    expect(m.girdiler.where((g) => g.tur == 'fare').map((g) => '${g.ad}:${g.basili}'), ['sol:true', 'sag:true', 'sol:false', 'sag:false']);
  });
}
