/// Oyun kolu değerlerini XInput düzenine çeviren Flutter bağımsız yardımcılar.
class OyunKoluDurumu {
  final int dugmeler;
  final int solX, solY, sagX, sagY, solTetik, sagTetik;
  const OyunKoluDurumu({this.dugmeler = 0, this.solX = 0, this.solY = 0, this.sagX = 0, this.sagY = 0, this.solTetik = 0, this.sagTetik = 0});

  static const tusBitleri = <String, int>{
    'up': 0x0001, 'down': 0x0002, 'left': 0x0004, 'right': 0x0008,
    'start': 0x0010, 'back': 0x0020, 'leftStick': 0x0040, 'rightStick': 0x0080,
    'lb': 0x0100, 'rb': 0x0200, 'lt': 0x0100, 'rt': 0x0200,
    'a': 0x1000, 'b': 0x2000, 'x': 0x4000, 'y': 0x8000,
  };

  static int dugmeMaskesi(Iterable<String> basili) => basili.fold(0, (m, k) => m | (tusBitleri[k] ?? 0));
  static int eksen(double v) => (v.clamp(-1.0, 1.0) * 32767).round();
  static int tetik(double v) => (v.clamp(0.0, 1.0) * 255).round();

  factory OyunKoluDurumu.android(Map<Object?, Object?> v) {
    final keys = (v['keys'] as List<Object?>? ?? const []).whereType<String>();
    return OyunKoluDurumu(
      dugmeler: dugmeMaskesi(keys),
      solX: eksen((v['lx'] as num?)?.toDouble() ?? 0),
      solY: eksen((v['ly'] as num?)?.toDouble() ?? 0),
      sagX: eksen((v['rx'] as num?)?.toDouble() ?? 0),
      sagY: eksen((v['ry'] as num?)?.toDouble() ?? 0),
      solTetik: tetik(((v['lt'] as num?)?.toDouble() ?? 0).clamp(keys.contains('lt') ? 1.0 : 0.0, 1.0)),
      sagTetik: tetik(((v['rt'] as num?)?.toDouble() ?? 0).clamp(keys.contains('rt') ? 1.0 : 0.0, 1.0)),
    );
  }

  @override
  bool operator ==(Object other) => other is OyunKoluDurumu && dugmeler == other.dugmeler && solX == other.solX && solY == other.solY && sagX == other.sagX && sagY == other.sagY && solTetik == other.solTetik && sagTetik == other.sagTetik;
  @override
  int get hashCode => Object.hash(dugmeler, solX, solY, sagX, sagY, solTetik, sagTetik);
}
