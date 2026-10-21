import 'dart:convert';
import 'dart:io';

import 'package:flutter/services.dart';

import 'hesap_servisi.dart';

/// Diagnostics use a small allowlist; raw bridge errors and credentials never
/// reach the file or the account page.
class GoogleHatasi extends HesapHatasi {
  final String kod;
  final String ayrinti;

  GoogleHatasi(this.kod, this.ayrinti) : super(_mesaj(kod));

  static String _mesaj(String kod) => switch (kod) {
    'G1' => 'Hesap seçilmedi ($kod), yeniden dene.',
    'G2' => 'Google girişi bu sürümde yapılandırılmamış (G2).',
    'G3' => 'Drive erişimi tamamlanamadı ($kod), yeniden giriş yapıp izin ver.',
    'G4' => 'Google hesabı doğrulanamadı ($kod), yeniden giriş yap.',
    'G5' =>
      'Google’a ulaşılamadı ($kod), internetini kontrol edip yeniden dene.',
    'G6' => 'Google ile yeniden giriş yap',
    _ => 'Google girişi tamamlanamadı (G9), yeniden dene.',
  };

  static GoogleHatasi cevir(Object hata, {String asama = 'giris'}) {
    if (hata is GoogleHatasi) return hata;
    var kod = 'G9';
    String tur = hata.runtimeType.toString();
    String mesaj = '';
    String durum = '';
    if (hata is PlatformException) {
      if (RegExp(r'^G[1234569]$').hasMatch(hata.code)) {
        kod = hata.code;
      } else if (hata.code == '10' || hata.code == 'DEVELOPER_ERROR') {
        kod = 'G2';
      } else if (hata.code == 'network_error') {
        kod = 'G5';
      } else if (hata.code == 'sign_in_canceled' || hata.code == '12501') {
        kod = 'G1';
      }
      if (hata.details case final Map details) {
        final sinif = '${details['exception'] ?? tur}';
        if (RegExp(r'^[A-Za-z][A-Za-z0-9.$]{0,150}$').hasMatch(sinif)) {
          tur = sinif;
        }
        mesaj = guvenliTani('${details['message'] ?? ''}');
        durum = guvenliTani('${details['status'] ?? ''}');
      }
    } else if (hata is SocketException || hata is HttpException) {
      kod = 'G5';
    } else {
      // Rust emits only these stage markers, never its underlying error text.
      kod = RegExp(r'\bG[1234569]\b').firstMatch('$hata')?.group(0) ?? kod;
    }
    return GoogleHatasi(kod, '$kod | $asama | $tur | $durum | $mesaj');
  }
}

/// Redact even unlabelled JWTs, OAuth tokens, URLs and account identifiers.
String guvenliTani(String text) => text
    .replaceAll(
      RegExp(
        r'(?:id_token|access_token|refresh_token|client_secret|token|secret|nonce|authorization)\s*[:=]\s*\S+',
        caseSensitive: false,
      ),
      '[gizli]',
    )
    .replaceAll(RegExp(r'https?://\S+|\S+@\S+'), '[gizli]')
    .replaceAll(
      RegExp(r'(?:Bearer\s+)?[A-Za-z0-9_./+=-]{24,}', caseSensitive: false),
      '[gizli]',
    )
    .replaceAll(RegExp(r'[\r\n\t]'), ' ')
    .split('')
    .take(400)
    .join();

Future<void> googleTaniYaz(GoogleHatasi hata, {Directory? klasor}) async {
  String? yol = klasor?.path;
  if (yol == null && Platform.isAndroid) {
    yol = await const MethodChannel(
      'afudesk/kol',
    ).invokeMethod<String>('veriKlasoru');
    if (yol != null) yol = '$yol/AfuDesk';
  } else if (yol == null && Platform.isWindows) {
    final local = Platform.environment['LOCALAPPDATA'];
    if (local != null) yol = '$local/AfuDesk';
  }
  if (yol == null) return;
  await Directory(yol).create(recursive: true);
  await File('$yol/google_son_hata.json').writeAsString(
    jsonEncode({
      'zaman': DateTime.now().toUtc().toIso8601String(),
      'kod': hata.kod,
      'ayrinti': hata.ayrinti,
    }),
    flush: true,
  );
}
