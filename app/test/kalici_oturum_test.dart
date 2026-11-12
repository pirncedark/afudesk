import 'dart:async';

import 'package:afudesk/hesap/gercek_hesap_servisi.dart';
import 'package:afudesk/hesap/oturum_kasasi.dart';
import 'package:afudesk/motor.dart';
import 'package:flutter_test/flutter_test.dart';

class _Kasa implements OturumKasasi {
  String? veri = 'kayitli';
  Future<String?>? bekleyenOkuma;
  @override
  Future<String?> oku() async {
    if (bekleyenOkuma != null) return await bekleyenOkuma!;
    return veri;
  }
  @override
  Future<void> yaz(String value) async {
    veri = value;
  }

  @override
  Future<void> sil() async {
    veri = null;
  }
}

void main() {
  GercekHesapServisi olustur(
    _Kasa kasa,
    Future<String> Function(String, String) islem,
  ) => GercekHesapServisi(
    kasa: kasa,
    islem: islem,
    googleBaglan: (_) => const Stream<IzleyiciOlay>.empty(),
    kes: () async {},
    hostOlaylari: const Stream<HostOlay>.empty(),
    platformGirisi: (nonce, {bool sessiz = false}) async =>
        throw StateError('Sessiz olmalı'),
    tarayiciAc: (_) async => throw StateError('Sessiz olmalı'),
  );

  testWidgets(
    'yükleme ağ hatasında 5/10 saniye bekler ve sessizce toparlanır',
    (tester) async {
      final kasa = _Kasa();
      var deneme = 0;
      final servis = olustur(kasa, (op, _) async {
        if (op == 'kaliciKayit') return '{}';
        expectSync(op, 'geriYukle');
        if (++deneme < 3) throw StateError('(G5)');
        return '{"ad":"Google hesabın","oturum":"yeni"}';
      });
      await servis.geriYukle();
      expect(servis.durum, 'Bağlanıyor…');
      expect(kasa.veri, 'kayitli');
      await tester.pump(const Duration(seconds: 5));
      await tester.idle();
      expect(deneme, 2);
      await tester.pump(const Duration(seconds: 9));
      expect(deneme, 2);
      await tester.pump(const Duration(seconds: 1));
      await tester.idle();
      expect(deneme, 3);
      expect(servis.hesap, isNotNull);
      expect(servis.durum, isNull);
      expect(kasa.veri, 'yeni');
      servis.dispose();
    },
  );

  testWidgets('heartbeat ağ hatasında hesabı korur ve yeniden dener', (
    tester,
  ) async {
    final kasa = _Kasa();
    var yenileme = 0;
    final servis = olustur(kasa, (op, _) async {
      if (op == 'yenile' && ++yenileme == 1) throw StateError('(G5)');
      return '{"ad":"Google hesabın","oturum":"yeni"}';
    });
    await servis.geriYukle();
    await tester.pump(const Duration(seconds: 60));
    expect(servis.hesap, isNotNull);
    expect(servis.durum, 'Bağlanıyor…');
    await tester.pump(const Duration(seconds: 5));
    expect(yenileme, 2);
    expect(servis.durum, isNull);
    servis.dispose();
  });

  testWidgets('yalnız G6 yeniden giriş ister, diğer hatalar kaydı korur', (
    tester,
  ) async {
    for (final kod in ['G2', 'G3', 'G4', 'G5', 'G6', 'G9']) {
      final kasa = _Kasa();
      final servis = olustur(kasa, (_, _) async => throw StateError('($kod)'));
      await servis.geriYukle();
      expect(kasa.veri, kod == 'G6' ? isNull : 'kayitli');
      expect(
        servis.durum,
        kod == 'G6'
            ? 'Google izni sona erdi. Yeniden bağlanmak için dokun.'
            : 'Bağlanıyor…',
      );
      servis.dispose();
    }
  });

  testWidgets('durdurma bekleyen sessiz denemeyi iptal eder', (tester) async {
    final kasa = _Kasa();
    var deneme = 0;
    final servis = olustur(kasa, (op, _) async {
      if (op == 'duraklat') return '{}';
      if (op == 'kaliciKayit') return '{}';
      deneme++;
      throw StateError('(G5)');
    });
    await servis.geriYukle();
    await servis.duraklat();
    await tester.pump(const Duration(minutes: 10));
    expect(deneme, 1);
    expect(kasa.veri, 'kayitli');
    servis.dispose();
  });

  testWidgets(
    'duraklatma geciken kasa okumasinin native hesabi acmasini engeller',
    (tester) async {
      final okuma = Completer<String?>();
      final kasa = _Kasa()..bekleyenOkuma = okuma.future;
      var geriYuklemeler = 0;
      final servis = olustur(kasa, (op, _) async {
        if (op == 'geriYukle') geriYuklemeler++;
        return '{}';
      });
      final yukleme = servis.geriYukle();
      await tester.idle();
      await servis.duraklat();
      okuma.complete('kayitli');
      await yukleme;
      expect(geriYuklemeler, 0);
      expect(servis.hesap, isNull);
      servis.dispose();
    },
  );

  testWidgets(
    'guven suresi heartbeat sirasinda dolarsa kasa silinir ve giris gorunur',
    (tester) async {
      final kasa = _Kasa();
      var yenilemeler = 0;
      final servis = olustur(kasa, (op, _) async {
        if (op == 'yenile') {
          yenilemeler++;
          throw StateError('(G6)');
        }
        if (op == 'cikis') return '{}';
        return '{"ad":"Google hesabın","oturum":"imzali"}';
      });
      await servis.geriYukle();
      await tester.pump(const Duration(seconds: 60));
      await tester.idle();
      expect(servis.hesap, isNull);
      expect(kasa.veri, isNull);
      expect(servis.durum, contains('dokun'));
      await tester.pump(const Duration(minutes: 10));
      expect(yenilemeler, 1);
      servis.dispose();
    },
  );

  testWidgets('rotation sonrasi ag hatasinda son bundle kasaya yazilir', (
    tester,
  ) async {
    final kasa = _Kasa();
    final servis = olustur(kasa, (op, _) async {
      if (op == 'geriYukle') throw StateError('(G5)');
      if (op == 'kaliciKayit') return '{"oturum":"rotated-imzali-bundle"}';
      throw StateError('Beklenmeyen islem');
    });
    await servis.geriYukle();
    expect(kasa.veri, 'rotated-imzali-bundle');
    expect(servis.hesap, isNull);
    expect(servis.durum, 'Bağlanıyor…');
    servis.dispose();
  });

  testWidgets(
    'rotation kaydi beklerken cikis yeni bundle yazilmasini engeller',
    (tester) async {
      final kasa = _Kasa();
      final kayit = Completer<String>();
      final servis = olustur(kasa, (op, _) async {
        if (op == 'geriYukle') throw StateError('(G5)');
        if (op == 'kaliciKayit') return kayit.future;
        if (op == 'cikis') return '{}';
        throw StateError('Beklenmeyen islem');
      });
      final yukleme = servis.geriYukle();
      await tester.idle();
      await servis.cikisYap();
      kayit.complete('{"oturum":"rotated-imzali-bundle"}');
      await yukleme;
      expect(kasa.veri, isNull);
      expect(servis.hesap, isNull);
      expect(servis.durum, isNull);
      servis.dispose();
    },
  );

  testWidgets('suresi dolan oturumdan sonra tek tik giris yeni bundle saklar', (
    tester,
  ) async {
    final kasa = _Kasa();
    var tarayici = 0;
    final servis = GercekHesapServisi(
      kasa: kasa,
      islem: (op, _) async {
        if (op == 'geriYukle') throw StateError('(G6)');
        if (op == 'baslat') return '{"url":"https://accounts.google.com/"}';
        if (op == 'tamamla') {
          return '{"ad":"Hesabım","oturum":"yeni-imzali-bundle"}';
        }
        throw StateError('Beklenmeyen islem');
      },
      googleBaglan: (_) => const Stream<IzleyiciOlay>.empty(),
      kes: () async {},
      hostOlaylari: const Stream<HostOlay>.empty(),
      platformGirisi: (_, {bool sessiz = false}) async =>
          throw StateError('Desktop'),
      tarayiciAc: (_) async {
        tarayici++;
        return true;
      },
    );
    await servis.geriYukle();
    expect(kasa.veri, isNull);
    await servis.girisYap();
    expect(tarayici, 1);
    expect(servis.hesap?.ad, 'Hesabım');
    expect(kasa.veri, 'yeni-imzali-bundle');
    expect(servis.durum, isNull);
    servis.dispose();
  });
}
