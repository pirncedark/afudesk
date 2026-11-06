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

/// Tepsi Durdur/Çıkış: yavaş ya da çevrimdışı ağ çıkışı bekletmesin; süre dolunca
/// ya da hata olunca da işlem sürer. Çıkış olursa `kapat` her durumda çağrılır.
Future<void> tepsiKomutu(
  Future<void> Function() duraklat,
  Future<void> Function() kapat, {
  required bool cikis,
  Duration sure = const Duration(seconds: 3),
}) async {
  try {
    await duraklat().timeout(sure);
  } catch (_) {
    // Zaman aşımı/hata durdurmayı ve çıkışı engellemez.
  } finally {
    if (cikis) await kapat();
  }
}

class ArkaPlan {
  static const kanal = MethodChannel('afudesk/arka_plan');
  static Future<bool> oku() async => Platform.isWindows &&
      (await kanal.invokeMethod<bool>('oku') ?? false);
  static Future<void> ayarla(bool acik) => kanal.invokeMethod<void>(
      'ayarla', acik ? baslangicKomutu(Platform.resolvedExecutable) : null);
}
