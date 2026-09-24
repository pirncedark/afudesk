import 'package:flutter/services.dart';

/// Flutter tuşunu çekirdeğin anladığı tuş adına çevirir. Bilinmeyen tuş: null.
String? tusAdi(LogicalKeyboardKey k) {
  final ozel = <LogicalKeyboardKey, String>{
    LogicalKeyboardKey.enter: 'Enter',
    LogicalKeyboardKey.numpadEnter: 'Enter',
    LogicalKeyboardKey.backspace: 'Backspace',
    LogicalKeyboardKey.tab: 'Tab',
    LogicalKeyboardKey.escape: 'Escape',
    LogicalKeyboardKey.space: 'Space',
    LogicalKeyboardKey.delete: 'Delete',
    LogicalKeyboardKey.home: 'Home',
    LogicalKeyboardKey.end: 'End',
    LogicalKeyboardKey.pageUp: 'PageUp',
    LogicalKeyboardKey.pageDown: 'PageDown',
    LogicalKeyboardKey.arrowUp: 'ArrowUp',
    LogicalKeyboardKey.arrowDown: 'ArrowDown',
    LogicalKeyboardKey.arrowLeft: 'ArrowLeft',
    LogicalKeyboardKey.arrowRight: 'ArrowRight',
    LogicalKeyboardKey.shiftLeft: 'Shift',
    LogicalKeyboardKey.shiftRight: 'Shift',
    LogicalKeyboardKey.controlLeft: 'Ctrl',
    LogicalKeyboardKey.controlRight: 'Ctrl',
    LogicalKeyboardKey.altLeft: 'Alt',
    LogicalKeyboardKey.altRight: 'Alt',
    LogicalKeyboardKey.metaLeft: 'Meta',
    LogicalKeyboardKey.metaRight: 'Meta',
    LogicalKeyboardKey.capsLock: 'CapsLock',
    LogicalKeyboardKey.f1: 'F1',
    LogicalKeyboardKey.f2: 'F2',
    LogicalKeyboardKey.f3: 'F3',
    LogicalKeyboardKey.f4: 'F4',
    LogicalKeyboardKey.f5: 'F5',
    LogicalKeyboardKey.f6: 'F6',
    LogicalKeyboardKey.f7: 'F7',
    LogicalKeyboardKey.f8: 'F8',
    LogicalKeyboardKey.f9: 'F9',
    LogicalKeyboardKey.f10: 'F10',
    LogicalKeyboardKey.f11: 'F11',
    LogicalKeyboardKey.f12: 'F12',
  };
  final o = ozel[k];
  if (o != null) return o;
  final etiket = k.keyLabel;
  // Tek karakterli tuşlar (harf, rakam, noktalama, Türkçe harfler): küçük harf gönder;
  // büyük harf karşı tarafta Shift ile oluşur, Ctrl+C gibi kısayollar da böyle çalışır.
  if (etiket.runes.length == 1) return etiket.toLowerCase();
  return null;
}
