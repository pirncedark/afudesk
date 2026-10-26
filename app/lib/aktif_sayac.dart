import 'dart:convert';
import 'dart:io';
import 'dart:math';

import 'guncelleme.dart';

const sayacUrl = 'https://afu-sayac.REPLACE.workers.dev';
typedef SayacIstek = Future<Map<String, dynamic>> Function(String method, Uri uri, [Map<String, dynamic>? body]);

class AktifSayac {
  AktifSayac({SayacIstek? istek, DateTime Function()? simdi,
      Future<String?> Function(String)? oku, Future<void> Function(String, String)? yaz})
      : _istek = istek ?? _httpIstek,
        _simdi = simdi ?? DateTime.now,
        _oku = oku ?? yerelAyarOku,
        _yaz = yaz ?? yerelAyarYaz;
  final SayacIstek _istek;
  final DateTime Function() _simdi;
  final Future<String?> Function(String) _oku;
  final Future<void> Function(String, String) _yaz;

  Future<String> _kimlik() async {
    var id = await _oku('sayacKimlik');
    if (id == null) {
      final r = Random.secure();
      final b = List<int>.generate(16, (_) => r.nextInt(256));
      b[6] = (b[6] & 0x0f) | 0x40;
      b[8] = (b[8] & 0x3f) | 0x80;
      final h = b.map((v) => v.toRadixString(16).padLeft(2, '0')).join();
      id = '${h.substring(0, 8)}-${h.substring(8, 12)}-${h.substring(12, 16)}-${h.substring(16, 20)}-${h.substring(20)}';
      await _yaz('sayacKimlik', id);
    }
    return id;
  }

  Future<Map<String, dynamic>> yukle({required bool katil, required String surum}) async {
    if (!katil) return {};
    final id = await _kimlik();
    final gun = _simdi().toIso8601String().substring(0, 10);
    if (await _oku('sayacSonGun') != gun) {
      await _istek('POST', Uri.parse('$sayacUrl/ping'), {'app': 'afudesk', 'id': id, 'v': surum});
      await _yaz('sayacSonGun', gun);
    }
    return _istek('GET', Uri.parse('$sayacUrl/stats?app=afudesk'));
  }
}

Future<Map<String, dynamic>> _httpIstek(String method, Uri uri, [Map<String, dynamic>? body]) async {
  final client = HttpClient()..connectionTimeout = const Duration(seconds: 10);
  try {
    final request = method == 'POST' ? await client.postUrl(uri) : await client.getUrl(uri);
    if (body != null) {
      request.headers.contentType = ContentType.json;
      request.write(jsonEncode(body));
    }
    final response = await request.close().timeout(const Duration(seconds: 15));
    if (response.statusCode < 200 || response.statusCode >= 300) throw const HttpException('Sayaç yanıt vermedi');
    return jsonDecode(await response.transform(utf8.decoder).join()) as Map<String, dynamic>;
  } finally { client.close(force: true); }
}
