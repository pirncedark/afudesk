import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';

import 'package:afudesk/google_otomasyon.dart';

const _basari =
    '{"status":"signed-in","needsUserApproval":false,"message":"Siteye giriş yapıldı."}\n';

class _SahteSurec implements Process {
  _SahteSurec({String cikti = _basari, int kod = 0, bool bekle = false})
    : stdout = Stream.value(utf8.encode(cikti)),
      stderr = Stream.value(utf8.encode('password=fixture-secret')),
      exitCode = bekle ? Completer<int>().future : Future.value(kod) {
    stdin = IOSink(_girdi.sink);
    girdi = _girdi.stream.expand((bytes) => bytes).toList();
  }

  final _girdi = StreamController<List<int>>();
  late final Future<List<int>> girdi;
  bool olduruldu = false;
  @override
  late final IOSink stdin;
  @override
  final Stream<List<int>> stdout;
  @override
  final Stream<List<int>> stderr;
  @override
  final Future<int> exitCode;
  @override
  int get pid => 123;
  @override
  bool kill([ProcessSignal signal = ProcessSignal.sigterm]) {
    olduruldu = true;
    return true;
  }
}

void main() {
  test('siteyeGir CLI çağırır, izinli JSON yazar ve sonucu döndürür', () async {
    final surec = _SahteSurec();
    final servis = GoogleOtomasyon(
      otomasyonDizini: '/fixture/otomasyon',
      surecBaslat: (komut, argumanlar, {workingDirectory}) async {
        expect(komut, 'node');
        expect(argumanlar, ['dist/src/cli.js']);
        expect(workingDirectory, '/fixture/otomasyon');
        return surec;
      },
    );
    final sonuc = await servis.siteyeGir('demo');
    expect(sonuc.basarili, isTrue);
    expect(sonuc.kullaniciOnayiGerekiyor, isFalse);
    expect(sonuc.mesaj, 'Siteye giriş yapıldı.');
    expect(jsonDecode(utf8.decode(await surec.girdi)), {
      'siteId': 'demo',
      'permission': 'giriş',
    });
    expect(surec.olduruldu, isFalse);
  });

  test(
    'CAPTCHA durma nedeni korunur, serbest çıktı kullanıcıya verilmez',
    () async {
      final surec = _SahteSurec(
        cikti: '{"status":"stopped","reason":"captcha","needsUserApproval":true,"message":"fixture-secret"}\n',
      );
      final sonuc = await GoogleOtomasyon(
        surecBaslat: (_, _, {workingDirectory}) async => surec,
      ).siteyeGir('demo');
      expect(sonuc.basarili, isFalse);
      expect(sonuc.neden, 'captcha');
      expect(sonuc.kullaniciOnayiGerekiyor, isTrue);
      expect(
        sonuc.mesaj,
        'İnsan doğrulaması gerekiyor; doğrulamayı kendiniz tamamlayın.',
      );
    },
  );

  for (final cikti in [
    'fixture-secret',
    '',
    '{}',
    'null',
    '$_basari$_basari',
    '{"status":"signed-in","needsUserApproval":true}',
    '{"status":"stopped","reason":"fixture-secret","needsUserApproval":true}',
    'x' * 8193,
  ]) {
    test(
      'bozuk süreç çıktısını güvenli hataya çevirir (${cikti.length})',
      () async {
        final sonuc = await GoogleOtomasyon(
          surecBaslat: (_, _, {workingDirectory}) async =>
              _SahteSurec(cikti: cikti),
        ).siteyeGir('demo');
        expect(sonuc.basarili, isFalse);
        expect(sonuc.neden, 'error');
        expect(sonuc.mesaj, isNot(contains('fixture')));
      },
    );
  }

  test('Node bulunamazsa tek cümle döner ve ham hatayı gizler', () async {
    final sonuc = await GoogleOtomasyon(
      surecBaslat: (_, _, {workingDirectory}) async =>
          throw const ProcessException('node', [], 'fixture-secret'),
    ).siteyeGir('demo');
    expect(sonuc.neden, 'error');
    expect(sonuc.mesaj, isNot(contains('fixture')));
  });

  test('başarılı JSON olsa bile başarısız çıkış kodunu reddeder', () async {
    final sonuc = await GoogleOtomasyon(
      surecBaslat: (_, _, {workingDirectory}) async => _SahteSurec(kod: 1),
    ).siteyeGir('demo');
    expect(sonuc.basarili, isFalse);
  });

  test('zaman aşımında süreci kapatıp güvenli hata döner', () async {
    final surec = _SahteSurec(bekle: true);
    final sonuc = await GoogleOtomasyon(
      zamanAsimi: const Duration(milliseconds: 20),
      surecBaslat: (_, _, {workingDirectory}) async => surec,
    ).siteyeGir('demo');
    expect(sonuc.neden, 'error');
    expect(surec.olduruldu, isTrue);
  });

  test('geçersiz site kimliği için süreç başlatmaz', () async {
    var basladi = false;
    final sonuc = await GoogleOtomasyon(
      surecBaslat: (_, _, {workingDirectory}) async {
        basladi = true;
        return _SahteSurec();
      },
    ).siteyeGir('../fixture');
    expect(basladi, isFalse);
    expect(sonuc.neden, 'error');
  });

  test('zaman aşımından sonra başlayan süreç de kapatılır', () async {
    final surec = _SahteSurec();
    final sonuc = await GoogleOtomasyon(
      zamanAsimi: const Duration(milliseconds: 10),
      surecBaslat: (_, _, {workingDirectory}) async {
        await Future<void>.delayed(const Duration(milliseconds: 40));
        return surec;
      },
    ).siteyeGir('demo');
    expect(sonuc.neden, 'error');
    await Future<void>.delayed(const Duration(milliseconds: 60));
    expect(surec.olduruldu, isTrue);
  });

  test('aşırı çıktı veren ve kapanmayan süreç hemen durdurulur', () async {
    final surec = _SahteSurec(cikti: 'x' * 8193, bekle: true);
    final sonuc = await GoogleOtomasyon(
      zamanAsimi: const Duration(seconds: 2),
      surecBaslat: (_, _, {workingDirectory}) async => surec,
    ).siteyeGir('demo').timeout(const Duration(milliseconds: 500));
    expect(sonuc.neden, 'error');
    expect(surec.olduruldu, isTrue);
  });
}
