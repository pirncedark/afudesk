import 'dart:async';
import 'dart:io';

import 'package:afudesk/hesap/gercek_hesap_servisi.dart';
import 'package:afudesk/hesap/google_hatasi.dart';
import 'package:afudesk/hesap/hesap_sayfasi.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('eksik masaüstü yapılandırması tek cümle G2 gösterir', () {
    expect(
      hesapHataMesaji(StateError('Google (G2)')),
      'Google girişi bu sürümde yapılandırılmamış (G2).',
    );
  });
  for (final code in ['G1', 'G2', 'G3', 'G4', 'G5', 'G9']) {
    test('platform hatası $code kullanıcıya ulaşır', () {
      expect(
        hesapHataMesaji(PlatformException(code: code)),
        contains('($code)'),
      );
    });
  }
  test('eski Android developer error yapılandırma hatasıdır', () {
    expect(hesapHataMesaji(PlatformException(code: '10')), contains('(G2)'));
  });
  test('Rust aşama kodu korunur, ham hata metni tanıya alınmaz', () {
    final hata = GoogleHatasi.cevir(StateError('Google (G4) token=secret'));
    expect(hata.kod, 'G4');
    expect(hata.ayrinti, isNot(contains('secret')));
    expect(GoogleHatasi.cevir(const SocketException('offline')).kod, 'G5');
    expect(GoogleHatasi.cevir(StateError('unknown')).kod, 'G9');
  });
  test('tanı JWT, OAuth tokenı ve hesap bilgilerini gizler', () {
    const token = 'eyJhbGciOiJSUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.signature';
    final hata = GoogleHatasi.cevir(
      PlatformException(
        code: 'G3',
        details: {
          'exception': 'GetCredentialCancellationException',
          'status': 16,
          'message':
              'denied $token token=short user@example.com https://example.com/?secret=x',
        },
      ),
    );
    expect(hata.ayrinti, contains('GetCredentialCancellationException'));
    for (final secret in [token, 'short', 'user@example.com', 'secret=x']) {
      expect(hata.ayrinti, isNot(contains(secret)));
    }
  });
  test('tanı dosyası son hatayla sınırlıdır', () async {
    final dir = await Directory.systemTemp.createTemp('afudesk-google-test');
    addTearDown(() => dir.delete(recursive: true));
    await googleTaniYaz(
      GoogleHatasi.cevir(PlatformException(code: 'G2')),
      klasor: dir,
    );
    await googleTaniYaz(
      GoogleHatasi.cevir(PlatformException(code: 'G3')),
      klasor: dir,
    );
    final text = await File('${dir.path}/google_son_hata.json').readAsString();
    expect(text, contains('G3'));
    expect(text, isNot(contains('G2')));
    expect(await dir.list().length, 1);
  });
  for (final taniBozuk in [false, true]) {
    test(
      'iptal hatası giriş nedenini kaybettirmez (tanı hatası: $taniBozuk)',
      () async {
        GoogleHatasi? kayit;
        final servis = GercekHesapServisi(
          islem: (op, _) async {
            if (op == 'baslat') return '{"nonce":"nonce"}';
            throw StateError('cleanup failure');
          },
          platformGirisi: (nonce, {bool sessiz = false}) async => throw PlatformException(code: 'G3'),
          googleBaglan: (_) => const Stream.empty(),
          hostOlaylari: const Stream.empty(),
          kes: () async {},
          tarayiciAc: (_) async => true,
          taniYaz: (hata) async {
            kayit = hata;
            if (taniBozuk) throw const FileSystemException('disk full');
          },
        );
        addTearDown(servis.dispose);
        await expectLater(
          servis.girisYap(),
          throwsA(isA<GoogleHatasi>().having((h) => h.kod, 'kod', 'G3')),
        );
        expect(kayit?.kod, 'G3');
        expect(servis.hesap, isNull);
        // A second attempt proves the busy flag is cleared after both failures.
        await expectLater(servis.girisYap(), throwsA(isA<GoogleHatasi>()));
      },
    );
  }
}
