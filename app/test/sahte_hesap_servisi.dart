import 'dart:async';
import 'package:afudesk/hesap/hesap_servisi.dart';

class SahteHesapServisi extends HesapServisi {
  SahteHesapServisi({List<Cihaz> cihazlar = const []})
    : _cihazlar = List.of(cihazlar);
  Hesap? _hesap;
  bool _uzaktan = false;
  final List<Cihaz> _cihazlar;
  final _akis = StreamController<List<Cihaz>>.broadcast();
  List<Cihaz> _baglantilar = [];
  @override
  Hesap? get hesap => _hesap;
  @override
  bool get uzaktanErisim => _uzaktan;
  @override
  List<Cihaz> get baglantilar => List.unmodifiable(_baglantilar);
  @override
  Stream<List<Cihaz>> get aktifBaglantilar => _akis.stream;

  void _girisGerekli() {
    if (_hesap == null) {
      throw const HesapHatasi('Hesabın kapalı, yeniden giriş yap.');
    }
  }

  void _yayinla() {
    _akis.add(baglantilar);
    notifyListeners();
  }

  @override
  Future<void> girisYap() async {
    _hesap = const Hesap(eposta: 'onizleme@example.com', ad: 'Önizleme');
    _uzaktan = true;
    notifyListeners();
  }

  @override
  Future<void> cikisYap() async {
    _hesap = null;
    _uzaktan = false;
    _cihazlar.clear();
    _baglantilar = [];
    _yayinla();
  }

  @override
  Future<List<Cihaz>> bilgisayarlarim() async {
    _girisGerekli();
    return List.unmodifiable(_cihazlar);
  }

  @override
  Future<void> uzaktanErisimAyarla(bool acik) async {
    _girisGerekli();
    _uzaktan = acik;
    notifyListeners();
  }

  @override
  Future<void> baglan(String cihazId) async {
    _girisGerekli();
    final cihaz = _cihazlar.where((c) => c.id == cihazId).firstOrNull;
    if (cihaz == null || !cihaz.cevrimici) {
      throw const HesapHatasi(
        'Bilgisayara ulaşılamadı, açık olduğunu kontrol et.',
      );
    }
    _baglantilar = [cihaz];
    _yayinla();
  }

  @override
  Future<void> baglantiyiKes() async {
    _baglantilar = [];
    _yayinla();
  }

  @override
  Future<void> cihazCikar(String cihazId) async {
    _girisGerekli();
    _cihazlar.removeWhere((c) => c.id == cihazId);
    _baglantilar.removeWhere((c) => c.id == cihazId);
    _yayinla();
  }

  @override
  void dispose() {
    _akis.close();
    super.dispose();
  }
}
