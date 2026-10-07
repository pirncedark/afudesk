import 'dart:ui';
import 'package:flutter/rendering.dart' show Matrix4;

/// Yerel görüntü dönüşümü; uzak bilgisayara girdi göndermez.
class Yakinlastirma {
  static const esik = 0.08;
  double olcek = 1;
  Offset kayma = Offset.zero;
  bool yakinlastiriyor = false;
  double _mesafe = 0, _basOlcek = 1;
  Offset _odak = Offset.zero;

  void basla(Offset p1, Offset p2) {
    yakinlastiriyor = olcek > 1.0;
    _mesafe = (p2 - p1).distance;
    _basOlcek = olcek;
    _odak = tersine((p1 + p2) / 2);
  }

  void guncelle(Offset p1, Offset p2, Size alan, {Rect? goruntu}) {
    if (_mesafe <= 0) return;
    final mesafe = (p2 - p1).distance;
    final oran = mesafe / _mesafe;
    final esigiGecti = (mesafe - _mesafe).abs() / _mesafe > esik;
    if (!yakinlastiriyor && !esigiGecti) return;
    yakinlastiriyor = true;
    olcek = (_basOlcek * oran).clamp(1.0, 5.0);
    final yeni = (p1 + p2) / 2 - _odak * olcek;
    final sinir = goruntu ?? (Offset.zero & alan);
    kayma = Offset(
      _sinirla(yeni.dx, sinir.left, sinir.right, alan.width),
      _sinirla(yeni.dy, sinir.top, sinir.bottom, alan.height),
    );
  }

  double _sinirla(double deger, double bas, double son, double alan) {
    if ((son - bas) * olcek <= alan) {
      return (alan - (bas + son) * olcek) / 2;
    }
    return deger.clamp(alan - son * olcek, -bas * olcek);
  }

  void bitir() {
    yakinlastiriyor = false;
    _mesafe = 0;
    if (olcek < 1.05) sifirla();
  }

  void sifirla() {
    olcek = 1;
    kayma = Offset.zero;
    yakinlastiriyor = false;
    _mesafe = 0;
  }

  Offset tersine(Offset ekran) => (ekran - kayma) / olcek;
  Offset uygula(Offset gorunum) => gorunum * olcek + kayma;
  Matrix4 get matris => Matrix4.identity()
    ..setEntry(0, 0, olcek)
    ..setEntry(1, 1, olcek)
    ..setEntry(0, 3, kayma.dx)
    ..setEntry(1, 3, kayma.dy);
}
