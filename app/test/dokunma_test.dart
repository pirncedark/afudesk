import 'package:afudesk/dokunma.dart';
import 'package:afudesk/motor.dart';
import 'package:flutter_test/flutter_test.dart';

String ozet(List<Girdi> g) => g
    .map((e) => switch (e.tur) {
          'konum' => 'konum(${e.x.toStringAsFixed(2)},${e.y.toStringAsFixed(2)})',
          'fare' => '${e.ad}:${e.basili ? 'bas' : 'birak'}',
          'kaydir' => 'kaydir(${e.dy})',
          _ => e.tur,
        })
    .join(' ');

void main() {
  test('dokunma = o noktaya git + sol tık', () {
    final d = DokunmaCevirici();
    expect(d.bas(1, 0.3, 0.4, 0), isEmpty);
    expect(ozet(d.birak(1, 120)), 'konum(0.30,0.40) sol:bas sol:birak');
    expect(d.imlecX, 0.3);
  });

  test('küçük titreme dokunmayı bozmaz', () {
    final d = DokunmaCevirici();
    d.bas(1, 0.5, 0.5, 0);
    expect(d.hareket(1, 0.505, 0.503, 30), isEmpty);
    expect(ozet(d.birak(1, 90)), 'konum(0.50,0.50) sol:bas sol:birak');
  });

  test('uzun bas = sağ tık, bırakınca ek tık yok', () {
    final d = DokunmaCevirici();
    d.bas(1, 0.2, 0.2, 0);
    expect(d.zaman(300), isEmpty);
    expect(ozet(d.zaman(600)), 'konum(0.20,0.20) sag:bas sag:birak');
    expect(d.zaman(900), isEmpty, reason: 'bir kez');
    expect(d.birak(1, 1000), isEmpty);
  });

  test('sürükleme = sol basılı hareket, bırakınca sol bırak', () {
    final d = DokunmaCevirici();
    d.bas(1, 0.1, 0.1, 0);
    expect(ozet(d.hareket(1, 0.2, 0.1, 50)), 'konum(0.10,0.10) sol:bas konum(0.20,0.10)');
    expect(ozet(d.hareket(1, 0.3, 0.2, 80)), 'konum(0.30,0.20)');
    expect(d.zaman(2000), isEmpty, reason: 'sürüklerken uzun bas tetiklenmez');
    expect(ozet(d.birak(1, 2100)), 'konum(0.30,0.20) sol:birak');
  });

  test('iki parmak sürükle = kaydırma (yukarı itince aşağı kayar)', () {
    final d = DokunmaCevirici();
    d.bas(1, 0.4, 0.6, 0);
    d.bas(2, 0.6, 0.6, 10);
    final g = <Girdi>[];
    for (var i = 1; i <= 10; i++) {
      g.addAll(d.hareket(1, 0.4, 0.6 - i * 0.01, 10 + i * 10));
      g.addAll(d.hareket(2, 0.6, 0.6 - i * 0.01, 10 + i * 10));
    }
    final kaydir = g.where((e) => e.tur == 'kaydir').toList();
    expect(kaydir, isNotEmpty);
    expect(kaydir.every((e) => e.dy == 1), isTrue);
    expect(g.where((e) => e.tur == 'fare'), isEmpty);
    expect(d.birak(1, 200), isEmpty);
    expect(d.birak(2, 210), isEmpty, reason: 'kaydırma sonrası sağ tık yok');
  });

  test('iki parmak dokun = sağ tık', () {
    final d = DokunmaCevirici();
    d.bas(1, 0.5, 0.5, 0);
    d.bas(2, 0.55, 0.5, 20);
    expect(d.birak(2, 120), isEmpty);
    expect(ozet(d.birak(1, 140)), 'konum(0.50,0.50) sag:bas sag:birak');
  });

  test('sürüklerken ikinci parmak gelirse sol tuş bırakılır', () {
    final d = DokunmaCevirici();
    d.bas(1, 0.1, 0.1, 0);
    d.hareket(1, 0.3, 0.1, 50);
    expect(ozet(d.bas(2, 0.5, 0.5, 60)), 'sol:birak');
  });

  test('iptal edilen parmak basılı tuş bırakmaz', () {
    final d = DokunmaCevirici();
    d.bas(1, 0.1, 0.1, 0);
    d.hareket(1, 0.3, 0.1, 50);
    expect(ozet(d.iptal(1)), 'sol:birak');
    expect(d.birak(1, 100), isEmpty);
  });

  test('ekran dışı konum 0..1 aralığına kırpılır', () {
    final d = DokunmaCevirici();
    d.bas(1, 1.2, -0.1, 0);
    expect(ozet(d.birak(1, 50)), 'konum(1.00,0.00) sol:bas sol:birak');
  });
}
