import 'dart:ui';
import 'package:afudesk/yakinlastirma.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  const alan = Size(400, 300);
  Yakinlastirma zoom() => Yakinlastirma()..basla(const Offset(150, 150), const Offset(250, 150));

  test('mesafe sabitken birlikte kayma zoom değildir', () {
    final z = zoom();
    z.guncelle(const Offset(150, 180), const Offset(250, 180), alan);
    expect(z.yakinlastiriyor, isFalse);
    expect(z.olcek, 1);
    z.guncelle(const Offset(146, 150), const Offset(254, 150), alan);
    expect(z.yakinlastiriyor, isFalse);
  });
  test('açılma odak altında büyür ve 5x ile sınırlıdır', () {
    final z = zoom();
    z.guncelle(const Offset(100, 150), const Offset(300, 150), alan);
    expect(z.olcek, 2);
    expect(z.tersine(const Offset(200, 150)), const Offset(200, 150));
    z.guncelle(const Offset(-800, 150), const Offset(1200, 150), alan);
    expect(z.olcek, 5);
  });
  test('odak kayınca pan ve dönüşümün tersi', () {
    final z = zoom();
    z.guncelle(const Offset(100, 150), const Offset(300, 150), alan);
    z.guncelle(const Offset(130, 170), const Offset(330, 170), alan);
    expect(z.tersine(const Offset(230, 170)), const Offset(200, 150));
    const p = Offset(73, 92);
    expect((z.tersine(z.uygula(p)) - p).distance, lessThan(0.00001));
    expect(z.matris.storage[0], 2);
    z.guncelle(const Offset(2000, 2000), const Offset(2200, 2000), alan);
    expect(z.kayma, Offset.zero);
    z.guncelle(const Offset(-2200, -2000), const Offset(-2000, -2000), alan);
    expect(z.kayma, const Offset(-400, -300));
  });
  test('1.05 altı bırakıldığında sıfırlanır', () {
    final z = zoom();
    z.guncelle(const Offset(100, 150), const Offset(300, 150), alan);
    z.guncelle(const Offset(148, 150), const Offset(252, 150), alan);
    z.bitir();
    expect(z.olcek, 1);
    expect(z.kayma, Offset.zero);
    expect(z.yakinlastiriyor, isFalse);
  });
  test('2x yeni harekette sabit mesafe pan yapar ve pinch devam eder', () {
    final z = zoom();
    z.guncelle(const Offset(100, 150), const Offset(300, 150), alan);
    z.bitir();
    final onceki = z.kayma;
    z.basla(const Offset(100, 150), const Offset(300, 150));
    expect(z.yakinlastiriyor, isTrue);
    z.guncelle(const Offset(130, 170), const Offset(330, 170), alan);
    expect(z.yakinlastiriyor, isTrue);
    expect(z.olcek, 2);
    expect(z.kayma, onceki + const Offset(30, 20));
    z.guncelle(const Offset(80, 170), const Offset(380, 170), alan);
    expect(z.olcek, 3);
    expect(z.tersine(const Offset(230, 170)), const Offset(200, 150));
  });
  test('küçülme 1x altına inmez ve bitir büyümeyi korur', () {
    final z = zoom();
    z.guncelle(const Offset(100, 150), const Offset(300, 150), alan);
    z.bitir();
    expect(z.olcek, 2);
    z.basla(const Offset(100, 150), const Offset(300, 150));
    z.guncelle(const Offset(190, 150), const Offset(210, 150), alan);
    expect(z.olcek, 1);
    expect(z.kayma, Offset.zero);
  });
  test('sıfırlama ve sıfır mesafe güvenlidir', () {
    final z = zoom();
    z.guncelle(const Offset(100, 150), const Offset(300, 150), alan);
    z.sifirla();
    expect(z.olcek, 1);
    expect(z.kayma, Offset.zero);
    z.basla(Offset.zero, Offset.zero);
    z.guncelle(Offset.zero, const Offset(20, 0), alan);
    expect(z.olcek.isFinite, isTrue);
  });
  test('kenar boşluklu görüntü pan ile ekran dışına kaçmaz', () {
    final z = zoom();
    const resim = Rect.fromLTRB(0, 100, 400, 200);
    z.guncelle(const Offset(100, 150), const Offset(300, 150), alan, goruntu: resim);
    expect(z.uygula(resim.topLeft).dy, 50);
    z.guncelle(const Offset(5000, 5000), const Offset(5500, 5000), alan, goruntu: resim);
    expect(z.uygula(resim.topLeft).dy, 0);
    z.guncelle(const Offset(-5500, -5000), const Offset(-5000, -5000), alan, goruntu: resim);
    expect(z.uygula(resim.bottomRight).dy, alan.height);
  });
}
