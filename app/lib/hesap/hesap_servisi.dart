import 'dart:async';

import 'package:flutter/foundation.dart';

class Hesap {
  final String eposta;
  final String ad;
  const Hesap({required this.eposta, required this.ad});
}

class Cihaz {
  final String id;
  final String ad;
  final bool cevrimici;
  const Cihaz({required this.id, required this.ad, required this.cevrimici});
}

class HesapHatasi implements Exception {
  final String mesaj;
  const HesapHatasi(this.mesaj);
}

abstract class HesapServisi extends ChangeNotifier {
  Hesap? get hesap;
  bool get uzaktanErisim;
  List<Cihaz> get baglantilar;
  Stream<List<Cihaz>> get aktifBaglantilar;
  Future<void> girisYap();
  Future<void> cikisYap();
  Future<List<Cihaz>> bilgisayarlarim();
  Future<void> uzaktanErisimAyarla(bool acik);
  Future<void> baglan(String cihazId);
  Future<void> baglantiyiKes();
  Future<void> cihazCikar(String cihazId);
}
