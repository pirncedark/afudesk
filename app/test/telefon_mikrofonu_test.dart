import 'dart:async';
import 'dart:typed_data';

import 'package:afudesk/telefon_mikrofonu.dart';
import 'package:afudesk/terminal_girdi.dart';
import 'package:flutter_test/flutter_test.dart';

import 'sahte_motor.dart';

class SahteMikrofon implements TelefonMikrofonu {
  final akis = StreamController<Uint8List>.broadcast(sync: true);
  bool izin = true;
  int istek = 0, baslatma = 0, durdurma = 0;
  Completer<bool>? bekleyen;
  @override
  Stream<Uint8List> get parcalar => akis.stream;
  @override
  Future<bool> izinIste() async {
    istek++;
    return bekleyen == null ? izin : await bekleyen!.future;
  }

  @override
  Future<void> baslat() async {
    baslatma++;
  }

  @override
  Future<void> durdur() async {
    durdurma++;
  }
}

void main() {
  for (final (destek, izin) in [(true, true), (false, true), (true, false)]) {
    testWidgets('bas konus: destek=$destek izin=$izin', (t) async {
      final motor = SahteMotor()..sesDestegi = destek;
      final mik = SahteMikrofon()..izin = izin;
      final b = BasKonusGirdisi(
        (g) => g.forEach(motor.girdi),
        sesDestegi: () => motor.sesDestegi,
        mikrofon: mik,
      );
      b.dugme(true);
      await t.pump();
      if (destek && izin) {
        expect(motor.girdiler.map((g) => g.tur), ['ses_ac']);
        await t.pump(const Duration(milliseconds: 149));
        expect(motor.girdiler, hasLength(1));
        await t.pump(const Duration(milliseconds: 1));
        mik.akis.add(Uint8List(639));
        expect(motor.girdiler.map((g) => g.tur), ['ses_ac', 'tus']);
        final pcm = Uint8List(640)..[0] = 42;
        mik.akis.add(pcm);
        expect(motor.girdiler.last.veri, pcm);
      }
      b.dugme(false);
      mik.akis.add(Uint8List(640)); // Bırakıldıktan sonra ses gitmez.
      await t.pump();
      expect(
        motor.girdiler.map((g) => g.tur),
        destek && izin
            ? ['ses_ac', 'tus', 'ses', 'tus', 'ses_kapat']
            : ['tus', 'tus'],
      );
      expect(motor.girdiler.where((g) => g.tur == 'tus').map((g) => g.basili), [
        true,
        false,
      ]);
      expect(mik.istek, destek ? 1 : 0);
      expect(mik.baslatma, destek && izin ? 1 : 0);
      expect(mik.durdurma, destek && izin ? 1 : 0);
      await mik.akis.close();
    });
  }

  testWidgets('surucu yok: yalniz Space ve uyari bir kez', (t) async {
    final motor = SahteMotor()
      ..sesDestegi = true
      ..sesSurucuYok = true;
    final mik = SahteMikrofon();
    var uyarilar = 0;
    final b = BasKonusGirdisi(
      (g) => g.forEach(motor.girdi),
      sesDestegi: () => motor.sesDestegi,
      sesSurucuYok: () => motor.sesSurucuYok,
      surucuUyarisi: () => uyarilar++,
      mikrofon: mik,
    );
    for (var i = 0; i < 2; i++) {
      b.spaceBas(aninda: true);
      await t.pump();
      b.spaceBirak();
    }
    expect(motor.girdiler.map((g) => g.tur), ['tus', 'tus', 'tus', 'tus']);
    expect(motor.girdiler.map((g) => g.ad), everyElement('Space'));
    expect(motor.girdiler.map((g) => g.basili), [true, false, true, false]);
    expect(uyarilar, 1);
    expect(mik.istek, 0);
    expect(mik.baslatma, 0);
    await mik.akis.close();
  });

  for (final izinBekliyor in [true, false]) {
    testWidgets('hazirlik sirasinda iptal: izinBekliyor=$izinBekliyor', (
      t,
    ) async {
      final motor = SahteMotor()..sesDestegi = true;
      final mik = SahteMikrofon();
      if (izinBekliyor) mik.bekleyen = Completer<bool>();
      final b = BasKonusGirdisi(
        (g) => g.forEach(motor.girdi),
        sesDestegi: () => motor.sesDestegi,
        mikrofon: mik,
      );
      b.dugme(true);
      await t.pump();
      b.iptal();
      mik.bekleyen?.complete(true);
      await t.pump(const Duration(seconds: 1));
      expect(motor.girdiler.where((g) => g.tur == 'tus'), isEmpty);
      expect(
        motor.girdiler.map((g) => g.tur),
        izinBekliyor ? [] : ['ses_ac', 'ses_kapat'],
      );
      expect(mik.baslatma, 0);
      await mik.akis.close();
    });
  }
}
