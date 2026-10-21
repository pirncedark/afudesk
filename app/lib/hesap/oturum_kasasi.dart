import 'package:flutter/services.dart';

/// Windows runner encrypts the complete bundle using CurrentUser DPAPI.
/// Tests replace this boundary; no token is stored in preferences or logs.
abstract class OturumKasasi {
  Future<String?> oku();
  Future<void> yaz(String veri);
  Future<void> sil();
}

class WindowsOturumKasasi implements OturumKasasi {
  static const _kanal = MethodChannel('afudesk/secure_storage');
  @override
  Future<String?> oku() => _kanal.invokeMethod<String>('read');
  @override
  Future<void> yaz(String veri) => _kanal.invokeMethod<void>('write', veri);
  @override
  Future<void> sil() => _kanal.invokeMethod<void>('clear');
}
