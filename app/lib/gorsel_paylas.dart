import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';

/// Oturum açılmadan gelen paylaşımları uygulama ömrü boyunca saklar.
class GorselPaylas extends ChangeNotifier {
  static final ortak = GorselPaylas();
  static const _kanal = MethodChannel('afudesk/gorsel');
  final _bekleyen = <String>[];
  Future<void>? _baslangic;
  bool _kapali = false;

  String? get ilk => _bekleyen.isEmpty ? null : _bekleyen.first;

  Future<void> baslat() => _baslangic ??= _baslat();

  Future<void> _baslat() async {
    _kanal.setMethodCallHandler((call) async {
      if (call.method == 'gorsel' && call.arguments is String) {
        ekle(call.arguments as String);
      }
    });
    try {
      final yollar = await _kanal.invokeListMethod<String>('bekleyenleriAl');
      for (final yol in yollar ?? <String>[]) {
        ekle(yol);
      }
    } on MissingPluginException {
      // Masaüstünde Android paylaşım kanalı yoktur.
    } on PlatformException {
      // Android yeni paylaşımları kanal üzerinden göndermeye devam eder.
    }
  }

  void ekle(String yol) {
    if (_kapali || yol.isEmpty || _bekleyen.contains(yol)) return;
    _bekleyen.add(yol);
    notifyListeners();
  }

  void al(String yol) {
    _bekleyen.remove(yol);
  }

  @override
  void dispose() {
    _kapali = true;
    _kanal.setMethodCallHandler(null);
    super.dispose();
  }
}
