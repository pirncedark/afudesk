import 'dart:async';

import 'motor.dart';
import 'telefon_mikrofonu.dart';

/// IME tamponunu sıfırlamadan, değişen kuyruğu terminalde yeniden yazar.
/// Terminal imleci son gönderilen metnin sonundadır; ortak son ek de yenilenir.
List<Girdi> imeGirdileri(String onceki, String simdiki) {
  final a = onceki.runes.toList(), b = simdiki.runes.toList();
  var ortak = 0;
  while (ortak < a.length && ortak < b.length && a[ortak] == b[ortak]) {
    ortak++;
  }
  return [
    for (var i = ortak; i < a.length; i++) ...[
      const Girdi('tus', ad: 'Backspace', basili: true),
      const Girdi('tus', ad: 'Backspace', basili: false),
    ],
    if (ortak < b.length)
      Girdi('metin', metin: String.fromCharCodes(b.skip(ortak))),
  ];
}

/// Holds Space with terminal key-repeat until the final source releases it.
class BasKonusGirdisi {
  BasKonusGirdisi(
    this.gonder, {
    bool Function()? sesDestegi,
    bool Function()? sesSurucuYok,
    this.surucuUyarisi,
    TelefonMikrofonu? mikrofon,
  }) : sesDestegi = sesDestegi ?? (() => false),
       sesSurucuYok = sesSurucuYok ?? (() => false),
       mikrofon = mikrofon ?? AndroidTelefonMikrofonu();
  final bool Function() sesDestegi;
  final bool Function() sesSurucuYok;
  final void Function()? surucuUyarisi;
  bool _surucuUyarildi = false;
  final TelefonMikrofonu mikrofon;
  StreamSubscription? _sesAkisi;
  Future<void> _mikIslem = Future.value();
  Timer? _sesBekle;
  int _nesil = 0;
  bool _sesAcik = false, _tusBasili = false;
  final void Function(List<Girdi>) gonder;
  final _kaynaklar = <String>{};
  Timer? _bekle;
  Timer? _tekrarBaslangic, _tekrarSaat;
  bool _space = false;

  void _tut(String kaynak) {
    if (!_kaynaklar.add(kaynak)) return;
    if (_kaynaklar.length != 1) return;
    final nesil = ++_nesil;
    if (sesDestegi() && !sesSurucuYok()) {
      _sesHazirla(nesil);
    } else {
      _yalnizSpace();
    }
  }

  void _yalnizSpace() {
    _spaceIndir();
    if (sesSurucuYok() && !_surucuUyarildi) {
      _surucuUyarildi = true;
      surucuUyarisi?.call();
    }
  }

  Future<void> _sesHazirla(int nesil) async {
    var izin = false;
    try {
      izin = await mikrofon.izinIste();
    } catch (_) {}
    if (nesil != _nesil || _kaynaklar.isEmpty) return;
    if (!izin || !sesDestegi() || sesSurucuYok()) {
      _yalnizSpace();
      return;
    }
    _sesAcik = true;
    gonder([const Girdi('ses_ac')]);
    _sesBekle = Timer(const Duration(milliseconds: 150), () {
      if (nesil != _nesil || _kaynaklar.isEmpty) return;
      _spaceIndir();
      _sesAkisi = mikrofon.parcalar.listen((veri) {
        if (nesil == _nesil && _sesAcik && _tusBasili && veri.length == 640) {
          gonder([Girdi('ses', veri: veri)]);
        }
      }, onError: (Object _) => _sesiKapat());
      _mikIslem = _mikIslem
          .then((_) async {
            if (nesil == _nesil && _sesAcik) await mikrofon.baslat();
          })
          .catchError((Object _) {
            if (nesil == _nesil) _sesiKapat();
          });
    });
  }

  void _spaceIndir() {
    _tusBasili = true;
    gonder([const Girdi('tus', ad: 'Space', basili: true)]);
    _tekrarBaslangic = Timer(const Duration(milliseconds: 500), () {
      _tekrarla();
      _tekrarSaat = Timer.periodic(
        const Duration(milliseconds: 50),
        (_) => _tekrarla(),
      );
    });
  }

  void _sesiKapat() {
    _sesBekle?.cancel();
    _sesAkisi?.cancel();
    _sesAkisi = null;
    if (!_sesAcik) return;
    _sesAcik = false;
    gonder([const Girdi('ses_kapat')]);
    _mikIslem = _mikIslem
        .then((_) => mikrofon.durdur())
        .catchError((Object _) {});
  }

  void _bitir() {
    _nesil++;
    _tekrariDurdur();
    if (_tusBasili) {
      _tusBasili = false;
      gonder([const Girdi('tus', ad: 'Space', basili: false)]);
    }
    _sesiKapat();
  }

  void _tekrarla() {
    if (_kaynaklar.isNotEmpty && _tusBasili) {
      gonder([const Girdi('tus', ad: 'Space', basili: true)]);
    }
  }

  void _tekrariDurdur() {
    _tekrarBaslangic?.cancel();
    _tekrarSaat?.cancel();
  }

  void _birak(String kaynak) {
    if (_kaynaklar.remove(kaynak) && _kaynaklar.isEmpty) {
      _bitir();
    }
  }

  void dugme(bool basili) => basili ? _tut('dugme') : _birak('dugme');

  void spaceBas({bool aninda = false}) {
    if (_space) return; // Otomatik tuş tekrarı yeni KEYDOWN üretmez.
    _space = true;
    if (aninda) {
      _tut('space');
    } else {
      _bekle = Timer(const Duration(milliseconds: 400), () => _tut('space'));
    }
  }

  void spaceBirak() {
    if (!_space) return;
    _space = false;
    _bekle?.cancel();
    if (_kaynaklar.contains('space')) {
      _birak('space');
    } else if (_kaynaklar.isEmpty) {
      gonder([
        const Girdi('tus', ad: 'Space', basili: true),
        const Girdi('tus', ad: 'Space', basili: false),
      ]);
    }
  }

  void iptal() {
    _bekle?.cancel();
    _tekrariDurdur();
    _space = false;
    if (_kaynaklar.isNotEmpty) {
      _kaynaklar.clear();
      _bitir();
    }
  }
}
