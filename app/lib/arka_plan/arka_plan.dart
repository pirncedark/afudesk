import 'dart:io';
import 'package:flutter/services.dart';

String baslangicKomutu(String exe) {
  if (exe.isEmpty || exe.contains('"') || exe.contains('\n') || exe.contains('\r')) {
    throw ArgumentError('Geçersiz uygulama yolu');
  }
  return '"$exe" --arka-plan';
}

bool tepsiyeGizle({required bool arkaPlan, required bool cikiliyor}) =>
    arkaPlan && !cikiliyor;

Duration? yenidenBaglanmaBeklemesi(int deneme) =>
    deneme >= 0 && deneme < 3 ? Duration(seconds: 1 << deneme) : null;

class ArkaPlan {
  static const kanal = MethodChannel('afudesk/arka_plan');
  static Future<bool> oku() async => Platform.isWindows &&
      (await kanal.invokeMethod<bool>('oku') ?? false);
  static Future<void> ayarla(bool acik) => kanal.invokeMethod<void>(
      'ayarla', acik ? baslangicKomutu(Platform.resolvedExecutable) : null);
}
