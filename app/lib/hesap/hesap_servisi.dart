// EKİP-1: Bu arayüzü gerçek OAuth/çekirdek adaptöründe uygulayın ve main'de
// AfuDeskUygulama.hesapServisi olarak verin. Sahte servis yalnız test/önizlemedir.
// Kimlik doğrulama, cihaz yetkisi, PIN deneme sınırı/hash/hatırlama ve şifreli
// kayıtlar çekirdeğin sorumluluğudur; UI güvenlik kararı vermez. baglan gerçek
// oturumu açar; gelen/giden oturum değişimleri baglantilar + aktifBaglantilar
// üzerinden birlikte yayımlanır. Hesap/ayar/liste değişince notifyListeners çağırın.
// pinDogrula yalnız geçerli oturuma bağlı kısa ömürlü izin verir; baglan yeniden
// yetki kontrol eder. Çıkış/cihaz çıkarma izinleri ve hatırlanan PIN'leri iptal eder.
// Cihaz.pinGerekli bu istemcide PIN sorulması gerektiğini belirtir; güvenli biçimde
// hatırlanmış izin varsa false döner. Liste bu cihazı içermez, ayarlarda çıkarma
// diğer cihazların kaydını iptal eder. baglantiyiKes bütün etkin oturumları keser.
import 'dart:async';
import 'dart:convert';
import 'dart:math';

import 'package:crypto/crypto.dart';
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
  final bool pinGerekli;
  const Cihaz({required this.id, required this.ad, required this.cevrimici,
    this.pinGerekli = false});
}

class HesapHatasi implements Exception {
  final String mesaj;
  const HesapHatasi(this.mesaj);
}

abstract class HesapServisi extends ChangeNotifier {
  Hesap? get hesap;
  bool get uzaktanErisim;
  bool get onaysizBaglanti;
  bool get pinKurulu;
  List<Cihaz> get baglantilar;
  Stream<List<Cihaz>> get aktifBaglantilar;
  Future<void> girisYap();
  Future<void> cikisYap();
  Future<List<Cihaz>> bilgisayarlarim();
  Future<void> uzaktanErisimAyarla(bool acik);
  Future<void> onaysizBaglantiAyarla(bool acik);
  Future<void> pinKur(String pin);
  Future<bool> pinDogrula(String cihazId, String pin, bool hatirla);
  Future<void> baglan(String cihazId);
  Future<void> baglantiyiKes();
  Future<void> cihazCikar(String cihazId);
}

/// Adaptör hazır olana kadar giriş kapalıdır; üretimde sahte hesap açılmaz.
class HazirDegilHesapServisi extends HesapServisi {
  Never _eksik() => throw const HesapHatasi(
    'Google ile giriş henüz hazır değil, kodla bağlantıyı kullan.');
  @override
  Hesap? get hesap => null;
  @override
  bool get uzaktanErisim => false;
  @override
  bool get onaysizBaglanti => false;
  @override
  bool get pinKurulu => false;
  @override
  List<Cihaz> get baglantilar => const [];
  @override
  Stream<List<Cihaz>> get aktifBaglantilar => const Stream.empty();
  @override
  Future<void> girisYap() async => _eksik();
  @override
  Future<void> cikisYap() async => _eksik();
  @override
  Future<List<Cihaz>> bilgisayarlarim() async => _eksik();
  @override
  Future<void> uzaktanErisimAyarla(bool acik) async => _eksik();
  @override
  Future<void> onaysizBaglantiAyarla(bool acik) async => _eksik();
  @override
  Future<void> pinKur(String pin) async => _eksik();
  @override
  Future<bool> pinDogrula(String cihazId, String pin, bool hatirla) async => _eksik();
  @override
  Future<void> baglan(String cihazId) async => _eksik();
  @override
  Future<void> baglantiyiKes() async => _eksik();
  @override
  Future<void> cihazCikar(String cihazId) async => _eksik();
}

/// Yalnız bellekte çalışan test/önizleme; kalıcı kimlik veya ağ bağlantısı yoktur.
class SahteHesapServisi extends HesapServisi {
  SahteHesapServisi({List<Cihaz> cihazlar = const [],
    Map<String, String> cihazPinleri = const {}})
      : _cihazlar = List.of(cihazlar) {
    for (final kayit in cihazPinleri.entries) {
      _pinler[kayit.key] = _Pin(kayit.value);
    }
  }
  Hesap? _hesap;
  bool _uzaktan = false;
  bool _onaysiz = true;
  _Pin? _yerelPin;
  final List<Cihaz> _cihazlar;
  final Map<String, _Pin> _pinler = {};
  final Set<String> _hatirlanan = {};
  final Set<String> _dogrulanan = {};
  final _akis = StreamController<List<Cihaz>>.broadcast();
  List<Cihaz> _baglantilar = [];
  @override
  Hesap? get hesap => _hesap;
  @override
  bool get uzaktanErisim => _uzaktan;
  @override
  bool get onaysizBaglanti => _onaysiz;
  @override
  bool get pinKurulu => _yerelPin != null;
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
    notifyListeners();
  }
  @override
  Future<void> cikisYap() async {
    _hesap = null;
    _uzaktan = false;
    _onaysiz = true;
    _yerelPin = null;
    _hatirlanan.clear();
    _dogrulanan.clear();
    _pinler.clear();
    _cihazlar.clear();
    _baglantilar = [];
    _yayinla();
  }
  @override
  Future<List<Cihaz>> bilgisayarlarim() async {
    _girisGerekli();
    return List.unmodifiable(_cihazlar.map((c) => Cihaz(id: c.id, ad: c.ad,
      cevrimici: c.cevrimici,
      pinGerekli: c.pinGerekli && !_hatirlanan.contains(c.id))));
  }
  @override
  Future<void> uzaktanErisimAyarla(bool acik) async {
    _girisGerekli();
    if (acik && !pinKurulu) {
      throw const HesapHatasi('Erişimi açmak için önce PIN belirle.');
    }
    _uzaktan = acik;
    notifyListeners();
  }
  @override
  Future<void> onaysizBaglantiAyarla(bool acik) async {
    _girisGerekli();
    _onaysiz = acik;
    notifyListeners();
  }
  @override
  Future<void> pinKur(String pin) async {
    _girisGerekli();
    if (!RegExp(r'^[0-9]{6,}$').hasMatch(pin)) {
      throw const HesapHatasi('PIN için en az 6 rakam gir.');
    }
    _yerelPin = _Pin(pin);
    notifyListeners();
  }
  @override
  Future<bool> pinDogrula(String cihazId, String pin, bool hatirla) async {
    _girisGerekli();
    final dogru = _pinler[cihazId]?.dogrula(pin) ?? false;
    if (dogru) {
      _dogrulanan.add(cihazId);
      if (hatirla) _hatirlanan.add(cihazId);
    }
    return dogru;
  }
  @override
  Future<void> baglan(String cihazId) async {
    _girisGerekli();
    final cihaz = _cihazlar.where((c) => c.id == cihazId).firstOrNull;
    if (cihaz == null || !cihaz.cevrimici) {
      throw const HesapHatasi('Bilgisayara ulaşılamadı, açık olduğunu kontrol et.');
    }
    if (cihaz.pinGerekli && !_hatirlanan.contains(cihazId) &&
        !_dogrulanan.remove(cihazId)) {
      throw const HesapHatasi('Bağlanmak için PIN gir.');
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
    _pinler.remove(cihazId);
    _hatirlanan.remove(cihazId);
    _dogrulanan.remove(cihazId);
    _yayinla();
  }
  @override
  void dispose() {
    _akis.close();
    super.dispose();
  }
}

// Test verisinde bile PIN'i düz metin saklama; bu hash üretim KDF'si değildir.
class _Pin {
  final String tuz = base64Encode(List.generate(16, (_) => Random.secure().nextInt(256)));
  late final String ozet;
  _Pin(String pin) { ozet = _ozet(pin); }
  String _ozet(String pin) => sha256.convert(utf8.encode('$tuz:$pin')).toString();
  bool dogrula(String pin) => _ozet(pin) == ozet;
}
