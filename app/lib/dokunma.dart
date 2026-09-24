// Dokunmatik ekran hareketlerini fare girdisine çevirir (doğrudan dokunma modeli).
//
//  - Tek parmak dokun           → o noktaya git + sol tık
//  - Tek parmak uzun bas        → o noktaya git + sağ tık
//  - Tek parmak sürükle         → sol tuş basılı sürükleme (seçme/taşıma)
//  - İki parmak sürükle         → tekerlek kaydırma
//  - İki parmak dokun           → sağ tık
//
// Konumlar 0..1 aralığında normalize gelir (ekran dışı: null). Zaman milisaniye.
// Saf sınıf: Flutter'a bağlı değil, birim testle doğrulanır.
import 'dart:math' as math;

import 'motor.dart';

class DokunmaCevirici {
  /// Bu kadar (normalize) hareketten sonra dokunma sürüklemeye döner.
  final double kayma;
  final int uzunBasMs;
  final int ikiParmakDokunMs;
  /// Kaydırmada bir "tık" için gereken dikey hareket (normalize).
  final double kaydirmaAdimi;

  DokunmaCevirici({
    this.kayma = 0.012,
    this.uzunBasMs = 550,
    this.ikiParmakDokunMs = 300,
    this.kaydirmaAdimi = 0.035,
  });

  final _parmaklar = <int, _Parmak>{};
  bool _surukleniyor = false;
  bool _uzunBasildi = false;
  bool _ikiParmakOldu = false;
  bool _ikiParmakHareket = false;
  int _ikiParmakBasla = 0;
  double _kaydirmaBirikim = 0;
  double? _kaydirmaY;

  /// Son gönderilen konum (arayüz imleç işareti çizmek için).
  double? imlecX, imlecY;

  List<Girdi> bas(int id, double x, double y, int t) {
    _parmaklar[id] = _Parmak(x, y, t);
    if (_parmaklar.length == 1) {
      _surukleniyor = false;
      _uzunBasildi = false;
      _ikiParmakOldu = false;
      _ikiParmakHareket = false;
    } else if (_parmaklar.length == 2) {
      _ikiParmakOldu = true;
      _ikiParmakBasla = t;
      _kaydirmaY = _ortY();
      _kaydirmaBirikim = 0;
      // Tek parmak sürüklemesi başladıysa bırak (ikinci parmak kaydırmaya geçer).
      if (_surukleniyor) {
        _surukleniyor = false;
        return [const Girdi('fare', ad: 'sol', basili: false)];
      }
    }
    return const [];
  }

  List<Girdi> hareket(int id, double x, double y, int t) {
    final p = _parmaklar[id];
    if (p == null) return const [];
    p.x = x;
    p.y = y;
    final cikti = <Girdi>[];
    if (_parmaklar.length >= 2) {
      final oy = _ortY();
      final onceki = _kaydirmaY ?? oy;
      _kaydirmaY = oy;
      _kaydirmaBirikim += oy - onceki;
      if (_kaydirmaBirikim.abs() > kayma) _ikiParmakHareket = true;
      while (_kaydirmaBirikim.abs() >= kaydirmaAdimi) {
        final yon = _kaydirmaBirikim > 0 ? 1 : -1;
        // Parmak yukarı itilince içerik aşağı kayar (telefon alışkanlığı).
        cikti.add(Girdi('kaydir', dy: -yon));
        _kaydirmaBirikim -= yon * kaydirmaAdimi;
      }
      return cikti;
    }
    if (_uzunBasildi) return const [];
    final mesafe = math.sqrt(math.pow(x - p.bx, 2) + math.pow(y - p.by, 2));
    if (!_surukleniyor && mesafe > kayma) {
      _surukleniyor = true;
      cikti.add(_konum(p.bx, p.by));
      cikti.add(const Girdi('fare', ad: 'sol', basili: true));
    }
    if (_surukleniyor) cikti.add(_konum(x, y));
    return cikti;
  }

  /// Arayüz bunu düzenli (ör. 100 ms) çağırır: uzun basışı zamanında yakalar.
  List<Girdi> zaman(int t) {
    if (_parmaklar.length != 1 || _surukleniyor || _uzunBasildi || _ikiParmakOldu) return const [];
    final p = _parmaklar.values.first;
    if (t - p.t >= uzunBasMs) {
      _uzunBasildi = true;
      return [_konum(p.bx, p.by), const Girdi('fare', ad: 'sag', basili: true), const Girdi('fare', ad: 'sag', basili: false)];
    }
    return const [];
  }

  List<Girdi> birak(int id, int t) {
    final p = _parmaklar.remove(id);
    if (p == null) return const [];
    final cikti = <Girdi>[];
    if (_ikiParmakOldu) {
      // Son parmak kalkınca: hareket yoksa ve kısa sürdüyse sağ tık.
      if (_parmaklar.isEmpty && !_ikiParmakHareket && t - _ikiParmakBasla <= ikiParmakDokunMs + 200) {
        cikti.addAll([
          _konum(p.bx, p.by),
          const Girdi('fare', ad: 'sag', basili: true),
          const Girdi('fare', ad: 'sag', basili: false),
        ]);
      }
      if (_parmaklar.isEmpty) _ikiParmakOldu = false;
      return cikti;
    }
    if (_surukleniyor) {
      _surukleniyor = false;
      return [_konum(p.x, p.y), const Girdi('fare', ad: 'sol', basili: false)];
    }
    if (_uzunBasildi) return const [];
    // Dokunma: sol tık.
    return [
      _konum(p.bx, p.by),
      const Girdi('fare', ad: 'sol', basili: true),
      const Girdi('fare', ad: 'sol', basili: false),
    ];
  }

  /// Parmak iptal (sistem hareketi vb.): basılı tuş kalmasın.
  List<Girdi> iptal(int id) {
    _parmaklar.remove(id);
    if (_surukleniyor) {
      _surukleniyor = false;
      return [const Girdi('fare', ad: 'sol', basili: false)];
    }
    return const [];
  }

  Girdi _konum(double x, double y) {
    imlecX = x.clamp(0.0, 1.0);
    imlecY = y.clamp(0.0, 1.0);
    return Girdi('konum', x: imlecX!, y: imlecY!);
  }

  double _ortY() => _parmaklar.values.map((p) => p.y).reduce((a, b) => a + b) / _parmaklar.length;
}

class _Parmak {
  final double bx, by;
  final int t;
  double x, y;
  _Parmak(this.bx, this.by, this.t)
      : x = bx,
        y = by;
}
