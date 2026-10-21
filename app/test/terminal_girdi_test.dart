import 'package:afudesk/motor.dart';
import 'package:afudesk/terminal_girdi.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  String uygula(String metin, List<Girdi> girdiler) {
    for (final g in girdiler) {
      if (g.tur == 'metin') metin += g.metin;
      if (g.ad == 'Backspace' && g.basili) {
        metin = String.fromCharCodes(metin.runes.take(metin.runes.length - 1));
      }
    }
    return metin;
  }

  for (final (a, b) in [
    ('', 'İstanbul ışığı'),
    ('çağ', 'çağrı'),
    ('cagri', 'çağrı'),
    ('yanlış kuyruk', 'yeni kuyruk'),
    ('şğüıöç', 'şğü'),
    ('ses', ''),
    ('', '😀'),
  ]) {
    test('IME farkı $a → $b', () {
      expect(uygula(a, imeGirdileri(a, b)), b);
    });
  }
  test('composing commit yalnız aralık değişirse metin tekrarlanmaz', () {
    expect(imeGirdileri('çağrı', 'çağrı'), isEmpty);
  });

  testWidgets('bas-konuş kaynakları birbirinin Space tuşunu bırakmaz', (t) async {
    final g = <Girdi>[];
    final b = BasKonusGirdisi(g.addAll);
    b.dugme(true);
    b.spaceBas();
    await t.pump(const Duration(milliseconds: 401));
    b.dugme(false);
    expect(g.map((x) => x.basili), [true]);
    b.spaceBirak();
    expect(g.map((x) => x.basili), [true, false]);
    b.iptal();
    expect(g, hasLength(2));
  });

  testWidgets('bekleyen uzun Space iptal olunca boşluk göndermez', (t) async {
    final g = <Girdi>[];
    final b = BasKonusGirdisi(g.addAll);
    b.spaceBas();
    b.iptal();
    await t.pump(const Duration(seconds: 1));
    b.spaceBirak();
    expect(g, isEmpty);
    b.dugme(true);
    b.iptal();
    b.dugme(false);
    expect(g.map((x) => x.basili), [true, false]);
  });
}
