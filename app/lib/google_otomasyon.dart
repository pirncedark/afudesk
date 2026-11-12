import 'dart:async';
import 'dart:convert';
import 'dart:io';

typedef OtomasyonSurecBaslat = Future<Process> Function(
  String executable,
  List<String> arguments, {
  String? workingDirectory,
});

const _mesajlar = {
  'consent': 'Yeni erişim izni gerekiyor; izin ekranını kendiniz onaylayın.',
  'account-creation':
      'Yeni hesap oluşturulacak; devam etmek için kendiniz onaylayın.',
  'captcha': 'İnsan doğrulaması gerekiyor; doğrulamayı kendiniz tamamlayın.',
  '2fa': 'İkinci doğrulama gerekiyor; doğrulamayı kendiniz tamamlayın.',
  'phone': 'Telefon doğrulaması gerekiyor; doğrulamayı kendiniz tamamlayın.',
  'payment': 'Ödeme bilgisi isteniyor; devam etmek için kendiniz onaylayın.',
  'unlisted-site':
      'Bu siteye izin verilmemiş; devam etmek için siteyi onaylayın.',
  'permission':
      'Bu işlem için izin verilmemiş; devam etmek için işlem iznini onaylayın.',
  'account-choice':
      'Google hesabı seçilmeli; kullanacağınız hesabı kendiniz seçin.',
  'manual-login': 'Google girişi gerekiyor; siteyi açıp kendiniz oturum açın.',
  'login-unavailable':
      'Otomatik giriş tamamlanamadı; siteye kendiniz giriş yapın.',
  'error': 'Giriş tamamlanamadı; siteyi açıp yeniden deneyin.',
};

class GoogleOtomasyonSonucu {
  const GoogleOtomasyonSonucu._(this.neden);

  /// Null yalnız doğrulanmış bir signed-in yanıtını temsil eder.
  final String? neden;
  bool get basarili => neden == null;
  bool get kullaniciOnayiGerekiyor => !basarili;
  String get mesaj => basarili ? 'Siteye giriş yapıldı.' : _mesajlar[neden]!;

  static GoogleOtomasyonSonucu _oku(Object? value) {
    if (value is Map<String, dynamic>) {
      if (value['status'] == 'signed-in' &&
          value['needsUserApproval'] == false) {
        return const GoogleOtomasyonSonucu._(null);
      }
      final reason = value['reason'];
      if (value['status'] == 'stopped' &&
          value['needsUserApproval'] == true &&
          reason is String &&
          _mesajlar.containsKey(reason)) {
        return GoogleOtomasyonSonucu._(reason);
      }
    }
    return const GoogleOtomasyonSonucu._('error');
  }
}

/// Yerel Node modülünü stdio ile çağırır; mevcut arayüzden bağımsızdır.
/// Paketleme sırasında otomasyonDizini uygulama içinden verilebilir.
class GoogleOtomasyon {
  GoogleOtomasyon({
    String? otomasyonDizini,
    OtomasyonSurecBaslat? surecBaslat,
    this.zamanAsimi = const Duration(seconds: 45),
  }) : _dizin = otomasyonDizini,
       _baslat = surecBaslat ?? _surecBaslat;

  final String? _dizin;
  final OtomasyonSurecBaslat _baslat;
  final Duration zamanAsimi;

  static Future<Process> _surecBaslat(
    String executable,
    List<String> arguments, {
    String? workingDirectory,
  }) =>
      Process.start(executable, arguments, workingDirectory: workingDirectory);

  String _modulDizini() {
    if (_dizin != null) return _dizin;
    final adaylar = [
      Directory('${File(Platform.resolvedExecutable).parent.path}/otomasyon'),
      Directory('${Directory.current.path}/otomasyon'),
      Directory('${Directory.current.parent.path}/otomasyon'),
    ];
    return adaylar
        .firstWhere(
          (dizin) => File('${dizin.path}/dist/src/cli.js').existsSync(),
          orElse: () => adaylar.first,
        )
        .absolute
        .path;
  }

  Future<GoogleOtomasyonSonucu> siteyeGir(String siteId) async {
    if (!RegExp(r'^[a-z0-9][a-z0-9-]{0,63}$').hasMatch(siteId)) {
      return const GoogleOtomasyonSonucu._('error');
    }
    Process? surec;
    var bitti = false;
    StreamSubscription<List<int>>? ciktiAkisi;
    StreamSubscription<List<int>>? hataAkisi;
    try {
      final islem = () async {
        final baslayan = await _baslat('node', [
          'dist/src/cli.js',
        ], workingDirectory: _modulDizini());
        if (bitti) {
          baslayan.kill();
          return const GoogleOtomasyonSonucu._('error');
        }
        surec = baslayan;
        final cikti = <int>[];
        final tamam = Completer<String>();
        ciktiAkisi = baslayan.stdout.listen(
          (bytes) {
            if (tamam.isCompleted) return;
            if (cikti.length + bytes.length > 8192) {
              tamam.completeError(const FormatException('Invalid response'));
            } else {
              cikti.addAll(bytes);
            }
          },
          onError: (Object hata, StackTrace iz) {
            if (!tamam.isCompleted) tamam.completeError(hata, iz);
          },
          onDone: () {
            if (!tamam.isCompleted) {
              try {
                tamam.complete(utf8.decode(cikti));
              } catch (hata, iz) {
                tamam.completeError(hata, iz);
              }
            }
          },
        );
        // Drain stderr without retaining or exposing diagnostic/secret data.
        hataAkisi = baslayan.stderr.listen((_) {}, onError: (Object _) {});
        // Attach error handlers before writing; output may arrive immediately.
        final degerler = await Future.wait<Object?>([
          tamam.future,
          baslayan.exitCode,
          () async {
            baslayan.stdin.write(
              jsonEncode({'siteId': siteId, 'permission': 'giriş'}),
            );
            await baslayan.stdin.close();
            return null;
          }(),
        ], eagerError: true);
        if (degerler[1] != 0) return const GoogleOtomasyonSonucu._('error');
        final metin = degerler[0] as String;
        if (!RegExp(r'^\{[^\r\n]+\}\r?\n$').hasMatch(metin)) {
          return const GoogleOtomasyonSonucu._('error');
        }
        return GoogleOtomasyonSonucu._oku(jsonDecode(metin));
      }();
      return await islem.timeout(zamanAsimi);
    } catch (_) {
      surec?.kill();
      return const GoogleOtomasyonSonucu._('error');
    } finally {
      bitti = true;
      await ciktiAkisi?.cancel();
      await hataAkisi?.cancel();
    }
  }
}
