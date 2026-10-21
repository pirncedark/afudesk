import 'dart:async';
import 'package:afudesk/hesap/gercek_hesap_servisi.dart';
import 'package:afudesk/hesap/oturum_kasasi.dart';
import 'package:afudesk/motor.dart';
import 'package:flutter_test/flutter_test.dart';

class _Kasa implements OturumKasasi {
  String? veri = 'kayitli';
  @override
  Future<String?> oku() async => veri;
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
    platformGirisi: (_) async => throw StateError('Sessiz olmalı'),
    tarayiciAc: (_) async => throw StateError('Sessiz olmalı'),
  );

  testWidgets(
    'yükleme ağ hatasında 5/10 saniye bekler ve sessizce toparlanır',
    (tester) async {
      final kasa = _Kasa();
      var deneme = 0;
      final servis = olustur(kasa, (op, _) async {
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
}
