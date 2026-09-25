import 'package:afudesk/oyun_kolu.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('eksen ve tetik değerleri XInput aralığına kırpılır', () {
    expect(OyunKoluDurumu.eksen(-2), -32767);
    expect(OyunKoluDurumu.eksen(0.5), 16384);
    expect(OyunKoluDurumu.eksen(2), 32767);
    expect(OyunKoluDurumu.tetik(-1), 0);
    expect(OyunKoluDurumu.tetik(0.5), 128);
    expect(OyunKoluDurumu.tetik(2), 255);
  });

  test('XInput tuş maskesi D-pad, omuz ve yüz tuşlarını eşler', () {
    expect(OyunKoluDurumu.dugmeMaskesi(['up', 'lb', 'a', 'y']), 0x9101);
  });

  test('Android eksenleri ve analog tetikleri korunur', () {
    final durum = OyunKoluDurumu.android({'keys': <String>[], 'lx': 0.25, 'lt': 0.4, 'rt': 0.8});
    expect(durum.solX, 8192);
    expect(durum.solTetik, 102);
    expect(durum.sagTetik, 204);
  });
}
