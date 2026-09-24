import 'dart:async';

import 'package:afudesk/motor.dart';

/// Testler için motor: olayları testin kendisi basar, çağrıları kaydeder.
class SahteMotor implements Motor {
  final host = StreamController<HostOlay>.broadcast(sync: true);
  final izleyici = StreamController<IzleyiciOlay>.broadcast(sync: true);
  final cagrilar = <String>[];
  final girdiler = <Girdi>[];
  Object? baglanHatasi;
  String? sonKod, sonParola;
  int kareOnayi = 0;

  @override
  String cihazAdi() => 'TEST-PC';

  @override
  Stream<HostOlay> hostBaslat({required String ad, String parola = '', bool upnp = true}) {
    cagrilar.add('hostBaslat:$ad');
    return host.stream;
  }

  @override
  Future<void> hostKabul({required bool kontrol}) async => cagrilar.add('kabul:$kontrol');
  @override
  Future<void> hostRed() async => cagrilar.add('red');
  @override
  Future<void> hostKes() async => cagrilar.add('kes');
  @override
  Future<void> hostDurdur() async => cagrilar.add('durdur');

  @override
  Stream<IzleyiciOlay> baglan({required String kod, required String parola, required String ad}) {
    sonKod = kod;
    sonParola = parola;
    cagrilar.add('baglan');
    if (baglanHatasi != null) return Stream.error(baglanHatasi!);
    return izleyici.stream;
  }

  @override
  void kareCizildi() => kareOnayi++;
  @override
  void girdi(Girdi g) => girdiler.add(g);
  @override
  Future<void> izleyiciKapat() async => cagrilar.add('izleyiciKapat');
}
