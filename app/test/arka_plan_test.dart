import 'dart:async';
import 'package:afudesk/arka_plan/arka_plan.dart';
import 'package:afudesk/hesap/gercek_hesap_servisi.dart';
import 'package:afudesk/hesap/oturum_kasasi.dart';
import 'package:afudesk/motor.dart';
import 'package:flutter_test/flutter_test.dart';

class SahteKasa implements OturumKasasi {
  String? sifreli;
  @override
  Future<String?> oku() async => sifreli;
  @override
  Future<void> yaz(String veri) async { sifreli = veri; }
  @override
  Future<void> sil() async { sifreli = null; }
}

void main() {
  GercekHesapServisi servisOlustur({
    required Future<String> Function(String, String) islem,
    OturumKasasi? kasa,
    Stream<IzleyiciOlay> Function(String)? baglan,
    Future<void> Function(Duration)? bekle,
  }) {
    final servis = GercekHesapServisi(
      islem: islem, kasa: kasa,
      googleBaglan: baglan ?? (_) => const Stream<IzleyiciOlay>.empty(),
      kes: () async {}, hostOlaylari: const Stream<HostOlay>.empty(),
      platformGirisi: (_) async => throw StateError('Sessiz yükleme'),
      tarayiciAc: (_) async => throw StateError('Tarayıcı açılmamalı'),
      bekle: bekle ?? (sure) => Future<void>.delayed(sure),
    );
    addTearDown(servis.dispose);
    return servis;
  }

  test('sessiz yükleme kasayı okur ve yenilenen paketi saklar', () async {
    final kasa = SahteKasa()..sifreli = 'eski';
    final servis = servisOlustur(kasa: kasa, islem: (op, veri) async {
      expect(op, 'geriYukle'); expect(veri, 'eski');
      return '{"ad":"Google hesabın","oturum":"yeni"}';
    });
    await servis.geriYukle();
    expect(servis.hesap, isNotNull);
    expect(await kasa.oku(), 'yeni');
  });

  test('ağ kesintisi kasayı korur, geçersiz token silinir', () async {
    for (final kod in ['G5', 'G6']) {
      final kasa = SahteKasa()..sifreli = 'paket';
      final servis = servisOlustur(kasa: kasa, islem: (_, _) async {
        throw StateError('Google işlemi tamamlanamadı ($kod)');
      });
      await servis.geriYukle();
      expect(await kasa.oku(), kod == 'G6' ? isNull : 'paket');
      expect(servis.hesap, isNull);
      if (kod == 'G6') expect(servis.durum, 'Google ile yeniden giriş yap');
    }
  });

  test('durdurma hatasında hesap temizlenir, kasa korunur', () async {
    final kasa = SahteKasa()..sifreli = 'paket';
    final servis = servisOlustur(kasa: kasa, islem: (op, _) async {
      if (op == 'duraklat') throw StateError('çevrimdışı');
      return '{"ad":"Google hesabın"}';
    });
    await servis.geriYukle();
    await expectLater(servis.duraklat(), throwsStateError);
    expect(servis.hesap, isNull);
    expect(servis.durum, contains('durduruldu'));
    expect(await kasa.oku(), 'paket');
  });

  test('bağlantı akışı üç kez 1, 2, 4 saniye bekler', () async {
    var acilis = 0;
    final beklemeler = <Duration>[];
    final servis = servisOlustur(
      islem: (_, _) async => '[{"id":"ev","ad":"Ev","cevrimici":true}]',
      bekle: (sure) async { beklemeler.add(sure); },
      baglan: (_) async* {
        acilis++;
        if (acilis == 1) {
          yield IzleyiciOlay('kabul'); yield IzleyiciOlay('yeniden');
        } else { yield IzleyiciOlay('hata'); }
      },
    );
    late Future<List<IzleyiciOlay>> olaylar;
    servis.oturumAc = (_, ac) { olaylar = ac().toList(); };
    await servis.baglan('ev');
    final sonuc = await olaylar;
    expect(acilis, 4);
    expect(beklemeler, [const Duration(seconds: 1),
      const Duration(seconds: 2), const Duration(seconds: 4)]);
    expect(sonuc.where((o) => o.tur == 'yeniden').length, 3);
    expect(sonuc.last.tur, 'koptu');
    expect(servis.sonBaglanmaSuresi, isNotNull);
    expect(servis.baglantilar, isEmpty);
  });

  test('Run komutu boşluk ve Unicode içeren exe yolunu tırnaklar', () {
    expect(baslangicKomutu(r'C:\Kullanıcı Dosyaları\AfuDesk.exe'),
        '"C:\\Kullanıcı Dosyaları\\AfuDesk.exe" --arka-plan');
    expect(() => baslangicKomutu('a"b.exe'), throwsArgumentError);
    expect(() => baslangicKomutu(''), throwsArgumentError);
  });
  test('yalnız arka plan açıkken X tepsiye gizler', () {
    expect(tepsiyeGizle(arkaPlan: true, cikiliyor: false), isTrue);
    expect(tepsiyeGizle(arkaPlan: false, cikiliyor: false), isFalse);
    expect(tepsiyeGizle(arkaPlan: true, cikiliyor: true), isFalse);
  });
  test('yeniden bağlantı üç kez 1, 2, 4 saniye bekler', () {
    expect([for (var i = 0; i < 4; i++) yenidenBaglanmaBeklemesi(i)],
        [const Duration(seconds: 1), const Duration(seconds: 2),
         const Duration(seconds: 4), null]);
  });
  test('oturum kasası yenileme paketini yükler, değiştirir ve siler', () async {
    final kasa = SahteKasa();
    expect(await kasa.oku(), isNull);
    await kasa.yaz('{"refreshToken":"ilk"}');
    expect(await kasa.oku(), '{"refreshToken":"ilk"}');
    await kasa.yaz('{"refreshToken":"yeni"}');
    expect(await kasa.oku(), '{"refreshToken":"yeni"}');
    await kasa.sil();
    expect(await kasa.oku(), isNull);
  });
}
