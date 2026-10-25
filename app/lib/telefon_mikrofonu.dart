import 'dart:io' show Platform;

import 'package:flutter/services.dart';

/// Telefon sesi: PCM16 little-endian, mono 16 kHz, 20 ms (640 bayt).
abstract class TelefonMikrofonu {
  Stream<Uint8List> get parcalar;
  Future<bool> izinIste();
  Future<void> baslat();
  Future<void> durdur();
}

class AndroidTelefonMikrofonu implements TelefonMikrofonu {
  static const _komut = MethodChannel('afudesk/mikrofon');
  static const _ses = EventChannel('afudesk/mikrofon/ses');
  @override
  late final Stream<Uint8List> parcalar = _ses
      .receiveBroadcastStream()
      .map((veri) => veri as Uint8List)
      .where((veri) => veri.length == 640);

  @override
  Future<bool> izinIste() async =>
      Platform.isAndroid && (await _komut.invokeMethod<bool>('izin') ?? false);
  @override
  Future<void> baslat() => _komut.invokeMethod<void>('baslat');
  @override
  Future<void> durdur() => _komut.invokeMethod<void>('durdur');
}
