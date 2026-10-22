import 'dart:async';

import 'motor.dart';

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
  BasKonusGirdisi(this.gonder);
  final void Function(List<Girdi>) gonder;
  final _kaynaklar = <String>{};
  Timer? _bekle;
  Timer? _tekrarBaslangic, _tekrarSaat;
  bool _space = false;

  void _tut(String kaynak) {
    if (!_kaynaklar.add(kaynak)) return;
    if (_kaynaklar.length == 1) {
      gonder([const Girdi('tus', ad: 'Space', basili: true)]);
      // Terminal hold detection needs repeat DOWN events, not repeated taps.
      _tekrarBaslangic = Timer(const Duration(milliseconds: 500), () {
        _tekrarla();
        _tekrarSaat = Timer.periodic(
          const Duration(milliseconds: 50),
          (_) => _tekrarla(),
        );
      });
    }
  }

  void _tekrarla() {
    if (_kaynaklar.isNotEmpty) {
      gonder([const Girdi('tus', ad: 'Space', basili: true)]);
    }
  }

  void _tekrariDurdur() {
    _tekrarBaslangic?.cancel();
    _tekrarSaat?.cancel();
  }

  void _birak(String kaynak) {
    if (_kaynaklar.remove(kaynak) && _kaynaklar.isEmpty) {
      _tekrariDurdur();
      gonder([const Girdi('tus', ad: 'Space', basili: false)]);
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
      gonder([const Girdi('tus', ad: 'Space', basili: false)]);
    }
  }
}
