// UI/UX testleri: her ekran, her düğme, hata ve kenar durumları (sahte motorla).
import 'package:afudesk/bilesenler/afu_uygulamalar.dart';
import 'package:afudesk/klavye.dart';
import 'package:afudesk/main.dart';
import 'package:afudesk/motor.dart';
import 'package:afudesk/sayfalar/baglanti_ver.dart';
import 'package:afudesk/tema.dart';
import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';

import 'sahte_motor.dart';

const ornekKod = 'AFU2.abcdefghijklmnopqrstuvwxyz0123456789';

Future<SahteMotor> ac(WidgetTester t, {Size boyut = const Size(1200, 800)}) async {
  t.view.physicalSize = boyut;
  t.view.devicePixelRatio = 1;
  addTearDown(t.view.reset);
  final m = SahteMotor();
  await t.pumpWidget(AfuDeskUygulama(motor: m));
  return m;
}

/// Animasyonlar (ilerleme göstergesi sonsuz döner) yüzünden pumpAndSettle yerine sabit adımlar.
Future<void> gec(WidgetTester t) async {
  for (var i = 0; i < 4; i++) {
    await t.pump(const Duration(milliseconds: 250));
  }
}

/// Gerçek görüntü çözme asenkron: gerçek zamanda bekle, her adımda kare pompala.
Future<void> kareBekle(WidgetTester t, SahteMotor m) async {
  await t.runAsync(() async {
    m.izleyici.add(IzleyiciOlay('kare', genislik: 200, yukseklik: 100, rgba: gri(200, 100)));
  });
  for (var i = 0; i < 200 && !_kareVar(t); i++) {
    await t.runAsync(() => Future<void>.delayed(const Duration(milliseconds: 20)));
    await t.pump();
  }
  await t.pump();
}

bool _kareVar(WidgetTester t) => find.byKey(const Key('oturum_kare')).evaluate().isNotEmpty;

Uint8List gri(int g, int y) => Uint8List.fromList(List.filled(g * y * 4, 128));

void main() {
  group('Ana ekran', () {
    testWidgets('iki seçenek ve Afu uygulamaları görünür', (t) async {
      await ac(t);
      expect(find.text('AfuDesk'), findsOneWidget);
      expect(find.text('Bağlantı ver'), findsOneWidget);
      expect(find.text('Bağlan'), findsOneWidget);
      for (final u in afuUygulamalari) {
        expect(find.byKey(Key('afu_${u.ad}')), findsOneWidget);
      }
    });

    testWidgets('dar ekranda (telefon) kartlar alt alta, taşma yok', (t) async {
      await ac(t, boyut: const Size(380, 800));
      expect(t.takeException(), isNull);
      final ver = t.getTopLeft(find.byKey(const Key('secenek_ver')));
      final bag = t.getTopLeft(find.byKey(const Key('secenek_baglan')));
      expect(bag.dy, greaterThan(ver.dy), reason: 'dar ekranda alt alta olmalı');
    });

    testWidgets('geniş ekranda kartlar yan yana', (t) async {
      await ac(t);
      final ver = t.getTopLeft(find.byKey(const Key('secenek_ver')));
      final bag = t.getTopLeft(find.byKey(const Key('secenek_baglan')));
      expect(bag.dx, greaterThan(ver.dx));
      expect(bag.dy, ver.dy);
    });

    testWidgets('Afu uygulama çipi doğru adresi açar', (t) async {
      final acilan = <String>[];
      await t.pumpWidget(MaterialApp(home: Scaffold(body: AfuUygulamalar(ac: acilan.add))));
      await t.tap(find.byKey(const Key('afu_AfuDM')));
      expect(acilan.single, contains('pirncedark/AfuDM'));
    });
  });

  group('Bağlantı ver', () {
    Future<SahteMotor> verAc(WidgetTester t) async {
      final m = await ac(t);
      await t.tap(find.byKey(const Key('secenek_ver')));
      await gec(t);
      return m;
    }

    testWidgets('hazırlanıyor → kod ve parola gösterilir', (t) async {
      final m = await verAc(t);
      expect(find.text('Bağlantı hazırlanıyor…'), findsOneWidget);
      expect(m.cagrilar, contains('hostBaslat:TEST-PC'));
      m.host.add(const HostOlay('hazir', kod: ornekKod, parola: '482913', erisim: 'İnternetten ulaşılabilir (1.2.3.4:47470)'));
      await t.pump();
      expect(find.text(ornekKod), findsOneWidget);
      expect(find.text('482913'), findsOneWidget);
      expect(find.textContaining('İnternetten'), findsOneWidget);
      expect(find.textContaining('içinde yenilenecek'), findsOneWidget);
    });

    testWidgets('kopyala düğmeleri panoya yazar', (t) async {
      final m = await verAc(t);
      final pano = <String>[];
      t.binding.defaultBinaryMessenger.setMockMethodCallHandler(SystemChannels.platform, (c) async {
        if (c.method == 'Clipboard.setData') pano.add((c.arguments as Map)['text'] as String);
        return null;
      });
      m.host.add(const HostOlay('hazir', kod: ornekKod, parola: '482913', erisim: 'x'));
      await t.pump();
      await t.tap(find.byKey(const Key('ver_kod_kopyala')));
      await t.tap(find.byKey(const Key('ver_parola_kopyala')));
      await t.pump();
      expect(pano, [ornekKod, '482913']);
      expect(find.text('Parola kopyalandı'), findsOneWidget);
    });

    testWidgets('gelen istek: kabul (kontrol izniyle)', (t) async {
      final m = await verAc(t);
      m.host.add(const HostOlay('hazir', kod: ornekKod, parola: '1', erisim: 'x'));
      m.host.add(const HostOlay('istek', ad: 'Veli'));
      await gec(t);
      expect(find.text('Veli bağlanmak istiyor'), findsOneWidget);
      await t.tap(find.byKey(const Key('istek_kabul')));
      await gec(t);
expect(m.cagrilar, contains('kabul:true:true'));
      expect(m.sonPano, isFalse, reason: 'pano kutusu i?aretlenmeden izin verilmez');
      expect(m.sonOyunKolu, isTrue);
      m.host.add(const HostOlay('baglandi', ad: 'Veli', kontrol: true));
      await t.pump();
      expect(find.text('Veli ekranını görüyor'), findsOneWidget);
      expect(find.text('Fare ve klavyeyi kullanabiliyor.'), findsOneWidget);
      await t.tap(find.byKey(const Key('ver_kes')));
      expect(m.cagrilar, contains('kes'));
    });

    testWidgets('gelen istek: kontrol izni kaldırılarak kabul', (t) async {
      final m = await verAc(t);
      m.host.add(const HostOlay('istek', ad: 'Veli'));
      await gec(t);
      await t.tap(find.byKey(const Key('istek_kontrol')));
      await t.pump();
      await t.tap(find.byKey(const Key('istek_kabul')));
      await gec(t);
      expect(m.cagrilar, contains('kabul:false:true'));
      expect(m.sonPano, isFalse);
      expect(m.sonOyunKolu, isTrue);
    });

    testWidgets('pano izin kutusu varsayılan kapalı', (t) async {
      final m = await verAc(t);
      m.host.add(const HostOlay('istek', ad: 'Veli'));
      await gec(t);
      final kutu = find.byKey(const Key('istek_pano'));
      expect(kutu, findsOneWidget);
      expect(find.text('Pano paylaşımı'), findsOneWidget);
      expect(t.widget<CheckboxListTile>(kutu).value, isFalse, reason: 'gizlilik: varsayılan kapalı');
      await t.tap(kutu);
      await t.pump();
      expect(t.widget<CheckboxListTile>(kutu).value, isTrue);
      await t.tap(find.byKey(const Key('istek_kabul')));
      await gec(t);
      expect(m.cagrilar, contains('kabul:true:true'));
      expect(m.sonPano, isTrue);
      m.host.add(const HostOlay('baglandi', ad: 'Veli', kontrol: true, pano: true));
      await t.pump();
      expect(find.byKey(const Key('ver_pano')), findsOneWidget);

      expect(m.cagrilar, contains('kabul:true:true'));
    });

    testWidgets('oyun kolu izin kutusu', (t) async {
      final m = await verAc(t);
      m.host.add(const HostOlay('istek', ad: 'Veli'));
      await gec(t);
      final kutu = find.byKey(const Key('istek_oyun_kolu'));
      expect(t.widget<CheckboxListTile>(kutu).value, isTrue);
      await t.tap(kutu);
      await t.pump();
      expect(t.widget<CheckboxListTile>(kutu).value, isFalse);
      await t.tap(find.byKey(const Key('istek_kabul')));
      await gec(t);
      expect(m.cagrilar, contains('kabul:true:false'));
    });

    testWidgets('oyun kolu sürücü uyarısı', (t) async {
      final m = await verAc(t);
      m.host.add(const HostOlay('baglandi', ad: 'Veli', oyunKolu: true));
      await t.pump();
      m.host.add(const HostOlay('uyari', metin: 'Oyun kolu sürücüsü gerekli'));
      await t.pump();
      expect(find.text('Oyun kolu sürücüsü gerekli'), findsOneWidget);

    });

    testWidgets('gelen istek: reddet', (t) async {
      final m = await verAc(t);
      m.host.add(const HostOlay('istek', ad: 'Veli'));
      await gec(t);
      await t.tap(find.byKey(const Key('istek_red')));
      await gec(t);
      expect(m.cagrilar, contains('red'));
      expect(m.cagrilar.where((c) => c.startsWith('kabul')), isEmpty);
    });

    testWidgets('dosya alma izin kutusu varsayılan kapalı', (t) async {
      final m = await verAc(t);
      m.host.add(const HostOlay('istek', ad: 'Veli'));
      await gec(t);
      expect(find.text('Dosya almaya izin ver'), findsOneWidget);
      expect(t.widget<CheckboxListTile>(find.byKey(const Key('istek_dosya'))).value, isFalse);
      expect(t.widget<CheckboxListTile>(find.byKey(const Key('istek_kontrol'))).value, isTrue,
          reason: 'kontrol kutusunun varsayılanı değişmemeli');
      await t.tap(find.byKey(const Key('istek_kabul')));
      await gec(t);
      expect(m.cagrilar, contains('kabul:true:true'));
      expect(m.sonDosyaIzni, isFalse, reason: 'dokunulmazsa dosya izni verilmemeli');
      m.host.add(const HostOlay('baglandi', ad: 'Veli', kontrol: true));
      await t.pump();
      expect(find.text('Dosya gönderemez.'), findsOneWidget);

      // Sonraki istekte kutu yine kapalı başlar; işaretlenirse izin gider.
      m.host.add(const HostOlay('koptu', metin: 'İzleyici bağlantıyı kapattı.'));
      m.host.add(const HostOlay('istek', ad: 'Ayşe'));
      await gec(t);
      expect(t.widget<CheckboxListTile>(find.byKey(const Key('istek_dosya'))).value, isFalse,
          reason: 'her istekte yeniden kapalı başlamalı');
      await t.tap(find.byKey(const Key('istek_dosya')));
      await t.pump();
      expect(t.widget<CheckboxListTile>(find.byKey(const Key('istek_dosya'))).value, isTrue);
      await t.tap(find.byKey(const Key('istek_kabul')));
      await gec(t);
      expect(m.sonDosyaIzni, isTrue);
    });

    testWidgets('alınan dosya bildirilir ve listelenir', (t) async {
      final m = await verAc(t);
      m.host.add(const HostOlay('baglandi', ad: 'Veli', kontrol: true, dosya: true));
      await t.pump();
      expect(find.textContaining('Sana dosya gönderebilir'), findsOneWidget);
      expect(find.byKey(const Key('ver_alinanlar')), findsNothing);
      m.host.add(const HostOlay('dosya', ad: 'rapor.pdf', yol: r'C:\Users\x\Downloads\AfuDesk\rapor.pdf'));
      await t.pump();
      expect(find.text('Dosya alındı: rapor.pdf'), findsOneWidget);
      expect(find.byKey(const Key('ver_alinanlar')), findsOneWidget);
      expect(find.descendant(of: find.byKey(const Key('ver_alinanlar')), matching: find.text('rapor.pdf')),
          findsOneWidget);
      expect(find.byTooltip(r'C:\Users\x\Downloads\AfuDesk\rapor.pdf'), findsOneWidget);
    });

    testWidgets('istek penceresi süre dolunca kendiliğinden reddeder', (t) async {
      String? sonuc = 'bos';
      await t.pumpWidget(MaterialApp(
        home: Builder(
          builder: (c) => TextButton(
            onPressed: () async {
              final r = await showDialog<IstekKarari?>(context: c, builder: (_) => const IstekPenceresi(ad: 'X', sure: Duration(seconds: 3)));
              sonuc = r?.toString();
            },
            child: const Text('aç'),
          ),
        ),
      ));
      await t.tap(find.text('aç'));
      await t.pump();
      expect(find.textContaining('3 sn'), findsOneWidget);
      await t.pump(const Duration(seconds: 1));
      await t.pump(const Duration(seconds: 1));
      await t.pump(const Duration(seconds: 1));
      await gec(t);
      expect(find.text('X bağlanmak istiyor'), findsNothing);
      expect(sonuc, isNull);
    });

    testWidgets('bağlantı kopunca mesaj gösterilir ve yeni kod beklenir', (t) async {
      final m = await verAc(t);
      m.host.add(const HostOlay('baglandi', ad: 'Veli'));
      await t.pump();
      m.host.add(const HostOlay('koptu', metin: 'İzleyici bağlantıyı kapattı.'));
      await t.pump();
      expect(find.text('İzleyici bağlantıyı kapattı.'), findsOneWidget);
      expect(find.text('Bağlantı hazırlanıyor…'), findsOneWidget);
    });

    testWidgets('hata durumunda "Tekrar dene" yeniden başlatır', (t) async {
      final m = await verAc(t);
      m.host.addError(Exception('Ağ bağlantısı bulunamadı.'));
      await t.pump();
      expect(find.text('Ağ bağlantısı bulunamadı.'), findsOneWidget);
      await t.tap(find.text('Tekrar dene'));
      await t.pump();
      expect(m.cagrilar.where((c) => c.startsWith('hostBaslat')).length, 2);
    });

    testWidgets('sayfadan çıkınca host durdurulur', (t) async {
      final m = await verAc(t);
      await t.pageBack();
      await gec(t);
      expect(m.cagrilar, contains('durdur'));
    });
  });

  group('Bağlan', () {
    Future<SahteMotor> baglanAc(WidgetTester t) async {
      final m = await ac(t);
      await t.tap(find.byKey(const Key('secenek_baglan')));
      await gec(t);
      return m;
    }

    testWidgets('boş alanlar uyarı verir, bağlanmaz', (t) async {
      final m = await baglanAc(t);
      await t.tap(find.byKey(const Key('baglan_dugme')));
      await t.pump();
      expect(find.text('Bağlantı kodunu yapıştır.'), findsOneWidget);
      expect(find.text('Parolayı yaz.'), findsOneWidget);
      expect(m.cagrilar, isNot(contains('baglan')));
    });

    testWidgets('AfuDesk kodu olmayan metin reddedilir', (t) async {
      final m = await baglanAc(t);
      await t.enterText(find.byKey(const Key('baglan_kod')), 'merhaba dünya');
      await t.enterText(find.byKey(const Key('baglan_parola')), '123456');
      await t.tap(find.byKey(const Key('baglan_dugme')));
      await t.pump();
      expect(find.textContaining('AfuDesk kodu değil'), findsOneWidget);
      expect(m.cagrilar, isNot(contains('baglan')));
    });

    testWidgets('yapıştır düğmesi panodan alır; boşluklar temizlenir', (t) async {
      final m = await baglanAc(t);
      t.binding.defaultBinaryMessenger.setMockMethodCallHandler(SystemChannels.platform, (c) async {
        if (c.method == 'Clipboard.getData') return {'text': '  AFU2.abc\ndef  '};
        return null;
      });
      await t.tap(find.byKey(const Key('baglan_yapistir')));
      await t.pump();
      await t.enterText(find.byKey(const Key('baglan_parola')), ' 123456 ');
      await t.tap(find.byKey(const Key('baglan_dugme')));
      await gec(t);
      expect(m.sonKod, 'AFU2.abcdef');
      expect(m.sonParola, '123456');
    });

    testWidgets('yanlış parola hatası anlaşılır biçimde gösterilir', (t) async {
      final m = await baglanAc(t);
      m.baglanHatasi = Exception('Parola yanlış.');
      await t.enterText(find.byKey(const Key('baglan_kod')), ornekKod);
      await t.enterText(find.byKey(const Key('baglan_parola')), '000000');
      await t.tap(find.byKey(const Key('baglan_dugme')));
      await gec(t);
      expect(find.text('Parola yanlış.'), findsOneWidget);
      expect(find.text('Geri dön'), findsOneWidget);
    });
  });

  group('Oturum', () {
    Future<SahteMotor> oturumAc(WidgetTester t) async {
      final m = await ac(t);
      await t.tap(find.byKey(const Key('secenek_baglan')));
      await gec(t);
      await t.enterText(find.byKey(const Key('baglan_kod')), ornekKod);
      await t.enterText(find.byKey(const Key('baglan_parola')), '123456');
      await t.tap(find.byKey(const Key('baglan_dugme')));
      await gec(t);
      return m;
    }

    testWidgets('bağlanıyor → onay bekleniyor → görüntü', (t) async {
      final m = await oturumAc(t);
      expect(find.byKey(const Key('oturum_baglaniyor')), findsOneWidget);
      m.izleyici.add(IzleyiciOlay('bekliyor', ad: 'Ali PC'));
      await t.pump();
      expect(find.textContaining('Ali PC onayı bekleniyor'), findsOneWidget);
      m.izleyici.add(IzleyiciOlay('kabul', kontrol: true, genislik: 200, yukseklik: 100));
      await t.pump();
      expect(find.text('Kontrol açık'), findsOneWidget);
      await kareBekle(t, m);
      expect(find.byKey(const Key('oturum_kare')), findsOneWidget);
      expect(m.kareOnayi, 1, reason: 'kare çizilince çekirdeğe haber verilmeli');
    });

    testWidgets('kol izni yoksa oyun kolu düğmesi pasif ve açıklamalı', (t) async {
      final m = await oturumAc(t);
      m.dokunmatik = true;
      m.izleyici.add(IzleyiciOlay('kabul', kontrol: true, genislik: 200, yukseklik: 100));
      await t.pump();
      final dugme = t.widget<FilledButton>(find.byKey(const Key('oturum_oyun_kolu')));
      expect(dugme.onPressed, isNull);
      expect(find.byTooltip('Karşı taraf oyun kolu izni vermedi'), findsOneWidget);
    });

    testWidgets('kol durumu değişince gönderilir, aynı durum yinelenmez', (t) async {
      final m = await oturumAc(t);
      m.dokunmatik = true;
      m.izleyici.add(IzleyiciOlay('kabul', kontrol: true, oyunKolu: true, genislik: 200, yukseklik: 100));
      await t.pump();
      // Katmandan gelen değişiklik akışı düğme/eksen değişiminde motoru çağırır.
      await t.tap(find.byKey(const Key('oturum_oyun_kolu')));
      await t.pump();
      // Dokunmatik katmanın gerçek pointer olayı ile durum gönderimini doğrula.
      final analog = find.byKey(const Key('oyun_kolu_analog_sol'));
      final p = t.getCenter(analog);
      final pointer = TestPointer(31);
      await t.sendEventToBinding(pointer.down(p));
      await t.pump(const Duration(milliseconds: 5));
      expect(m.oyunKoluDurumlari, isNotEmpty);
      final adet = m.oyunKoluDurumlari.length;
      await t.sendEventToBinding(pointer.move(p + const Offset(12, 0)));
      await t.pump(const Duration(milliseconds: 5));
      expect(m.oyunKoluDurumlari.length, greaterThan(adet));
      final degisenAdet = m.oyunKoluDurumlari.length;
      await t.sendEventToBinding(pointer.move(p + const Offset(12, 0)));
      await t.pump(const Duration(milliseconds: 5));
      expect(m.oyunKoluDurumlari.length, degisenAdet, reason: 'aynı kol durumu tekrar gönderilmemeli');
      await t.sendEventToBinding(pointer.up());
      await t.pump();
    });

    testWidgets('dokunmatik kol katmanı aynı anda analog ve tuş basışını işler', (t) async {
      final m = await oturumAc(t);
      m.dokunmatik = true;
      m.izleyici.add(IzleyiciOlay('kabul', kontrol: true, oyunKolu: true, genislik: 200, yukseklik: 100));
      await t.pump();
      await t.tap(find.byKey(const Key('oturum_oyun_kolu')));
      await t.pump();
      final analog = t.getCenter(find.byKey(const Key('oyun_kolu_analog_sol')));
      final a = t.getCenter(find.byKey(const Key('oyun_kolu_tus_a')));
      final parmak1 = TestPointer(41), parmak2 = TestPointer(42);
      await t.sendEventToBinding(parmak1.down(analog + const Offset(20, 0)));
      await t.sendEventToBinding(parmak2.down(a));
      await t.pump(const Duration(milliseconds: 5));
      expect(m.oyunKoluDurumlari, isNotEmpty);
      expect(m.oyunKoluDurumlari.last.dugmeler & 0x1000, 0x1000);
      expect(m.oyunKoluDurumlari.last.solX, isNot(0));
      await t.sendEventToBinding(parmak2.up());
      await t.sendEventToBinding(parmak1.up());
      await t.pump();
    });

    testWidgets('uzak kol algılanınca oturum başlığında P2 çipi görünür', (t) async {
      final m = await oturumAc(t);
      m.izleyici.add(IzleyiciOlay('kabul', kontrol: true, genislik: 20, yukseklik: 10));
      await t.pump();
      expect(find.byKey(const Key('oturum_p2')), findsNothing);
      m.izleyici.add(IzleyiciOlay('kol'));
      await t.pump();
      expect(find.text('🎮 P2'), findsOneWidget);
    });

    Future<Rect> goruntuAlani(WidgetTester t, SahteMotor m, {bool kontrol = true}) async {
      m.izleyici.add(IzleyiciOlay('kabul', kontrol: kontrol, genislik: 200, yukseklik: 100));
      await t.pump();
      await kareBekle(t, m);
      return t.getRect(find.byKey(const Key('oturum_ekran')));
    }

    testWidgets('fare konumu en-boy oranına göre normalize edilir, siyah bant dışı yok sayılır', (t) async {
      final m = await oturumAc(t);
      final alan = await goruntuAlani(t, m);
      // 2:1 görüntü, alan daha "uzun" → üst/alt siyah bant. Merkez = (0.5, 0.5).
      final fare = await t.createGesture(kind: PointerDeviceKind.mouse);
      await fare.addPointer(location: alan.center);
      await fare.moveTo(alan.center + const Offset(1, 0));
      await t.pump();
      final k = m.girdiler.lastWhere((g) => g.tur == 'konum');
      expect(k.x, closeTo(0.5, 0.01));
      expect(k.y, closeTo(0.5, 0.01));
      final once = m.girdiler.length;
      await fare.moveTo(Offset(alan.center.dx, alan.top + 2)); // üst siyah bant
      await t.pump();
      expect(m.girdiler.skip(once).where((g) => g.tur == 'konum'), isEmpty);
      await fare.removePointer();
    });

    testWidgets('sol ve sağ tık basılı/bırakıldı olarak gider', (t) async {
      final m = await oturumAc(t);
      final alan = await goruntuAlani(t, m);
      await t.tapAt(alan.center, buttons: kPrimaryMouseButton, kind: PointerDeviceKind.mouse);
      await t.tapAt(alan.center, buttons: kSecondaryMouseButton, kind: PointerDeviceKind.mouse);
      final f = m.girdiler.where((g) => g.tur == 'fare').map((g) => '${g.ad}:${g.basili}').toList();
      expect(f, ['sol:true', 'sol:false', 'sag:true', 'sag:false']);
    });

    testWidgets('tekerlek kaydırma gider', (t) async {
      final m = await oturumAc(t);
      final alan = await goruntuAlani(t, m);
      final p = TestPointer(7, PointerDeviceKind.mouse);
      await t.sendEventToBinding(p.hover(alan.center));
      await t.sendEventToBinding(p.scroll(const Offset(0, 120)));
      await t.sendEventToBinding(p.scroll(const Offset(0, -120)));
      await t.pump();
      expect(m.girdiler.where((g) => g.tur == 'kaydir').map((g) => g.dy).toList(), [1, -1]);
    });

    testWidgets('klavye: tuş basma/bırakma ve Türkçe harf', (t) async {
      final m = await oturumAc(t);
      await goruntuAlani(t, m);
      await t.sendKeyEvent(LogicalKeyboardKey.enter);
      await t.sendKeyDownEvent(LogicalKeyboardKey.controlLeft);
      await t.sendKeyEvent(LogicalKeyboardKey.keyC);
      await t.sendKeyUpEvent(LogicalKeyboardKey.controlLeft);
      final tuslar = m.girdiler.where((g) => g.tur == 'tus').map((g) => '${g.ad}:${g.basili}').toList();
      expect(tuslar, ['Enter:true', 'Enter:false', 'Ctrl:true', 'c:true', 'c:false', 'Ctrl:false']);
    });

    testWidgets('yalnız izleme izninde hiçbir girdi gönderilmez', (t) async {
      final m = await oturumAc(t);
      final alan = await goruntuAlani(t, m, kontrol: false);
      expect(find.text('Yalnız izleme'), findsOneWidget);
      await t.tapAt(alan.center, kind: PointerDeviceKind.mouse);
      await t.sendKeyEvent(LogicalKeyboardKey.enter);
      expect(m.girdiler, isEmpty);
    });

    testWidgets('gecikme göstergesi: renk ağ durumuna göre', (t) async {
      final m = await oturumAc(t);
      await goruntuAlani(t, m);
      expect(find.byKey(const Key('oturum_istatistik')), findsNothing, reason: 'ölçüm gelmeden gösterilmez');
      m.izleyici.add(IzleyiciOlay('istatistik', rttMs: 23, fps: 15, gecikmeMs: 74));
      await t.pump();
      final y = t.widget<Text>(find.byKey(const Key('oturum_istatistik')));
      expect(y.data, '74 ms · 15 fps');
      expect(y.style!.color, Renk.basari);
      m.izleyici.add(IzleyiciOlay('istatistik', rttMs: 350, fps: 8, gecikmeMs: 280));
      await t.pump();
      expect(t.widget<Text>(find.byKey(const Key('oturum_istatistik'))).style!.color, Renk.tehlike);
      m.izleyici.add(IzleyiciOlay('istatistik', rttMs: 350, fps: 8));
      await t.pump();
      expect(t.widget<Text>(find.byKey(const Key('oturum_istatistik'))).data, '350 ms · 8 fps');
    });

    testWidgets('relay üzerinden gelince "aktarmalı" yazar, doğrudan olunca yazmaz', (t) async {
      final m = await oturumAc(t);
      await goruntuAlani(t, m);
      m.izleyici.add(IzleyiciOlay('istatistik', rttMs: 90, fps: 12, metin: 'relay'));
      await t.pump();
      expect(t.widget<Text>(find.byKey(const Key('oturum_istatistik'))).data, '90 ms · 12 fps · aktarmalı');
      m.izleyici.add(IzleyiciOlay('istatistik', rttMs: 4, fps: 20, metin: 'doğrudan'));
      await t.pump();
      expect(t.widget<Text>(find.byKey(const Key('oturum_istatistik'))).data, '4 ms · 20 fps');
    });

    testWidgets('bağlantı kopunca yeniden bağlanma gösterilir, oturum kapanmaz', (t) async {
      final m = await oturumAc(t);
      await goruntuAlani(t, m);
      m.izleyici.add(IzleyiciOlay('istatistik', rttMs: 20, fps: 15));
      await t.pump();
      m.izleyici.add(IzleyiciOlay('yeniden', metin: 'Bağlantı koptu, yeniden bağlanılıyor… (1)'));
      await t.pump();
      expect(find.byKey(const Key('oturum_yeniden')), findsOneWidget);
      expect(find.byKey(const Key('oturum_istatistik')), findsNothing);
      m.izleyici.add(IzleyiciOlay('kabul', kontrol: true));
      await t.pump();
      expect(find.byKey(const Key('oturum_yeniden')), findsNothing, reason: 'geri dönünce kalkar');
      m.izleyici.add(IzleyiciOlay('istatistik', rttMs: 20, fps: 15));
      await t.pump();
      expect(find.byKey(const Key('oturum_istatistik')), findsOneWidget);
    });

    testWidgets('karşıdan pano gelince kısa bildirim gösterilir', (t) async {
      final m = await oturumAc(t);
      await goruntuAlani(t, m);
      m.izleyici.add(IzleyiciOlay('pano', metin: 'kopyalanan'));
      await t.pump();
      expect(find.text('Pano güncellendi'), findsOneWidget);
    });

    testWidgets('dosya gönderme ilerlemesi gösterilir', (t) async {
      final m = await oturumAc(t);
      m.izleyici.add(IzleyiciOlay('kabul', kontrol: true, dosya: true, genislik: 200, yukseklik: 100));
      await t.pump();
      final cubuk = find.byKey(const Key('oturum_dosya_ilerleme'));
      final dugme = find.byKey(const Key('oturum_dosya_gonder'));
      expect(cubuk, findsNothing, reason: 'gönderim yokken çubuk yok');

      // Seçici vazgeçilirse hiçbir şey gönderilmez.
      m.secilecekDosya = null;
      await t.tap(dugme);
      await t.pump();
      expect(m.gonderilenDosyalar, isEmpty);
      expect(cubuk, findsNothing);

      m.secilecekDosya = r'C:\Belgeler\rapor.pdf';
      await t.tap(dugme);
      await t.pump();
      expect(m.gonderilenDosyalar, [r'C:\Belgeler\rapor.pdf']);
      expect(find.text('rapor.pdf hazırlanıyor…'), findsOneWidget);
      expect(t.widget<LinearProgressIndicator>(cubuk).value, isNull);

      m.izleyici.add(IzleyiciOlay('dosya', ad: 'rapor.pdf', gonderilen: 1024 * 1024, toplam: 4 * 1024 * 1024));
      await t.pump();
      expect(t.widget<LinearProgressIndicator>(cubuk).value, closeTo(0.25, 1e-9));
      expect(find.text('rapor.pdf — %25 (1,0 MB / 4,0 MB)'), findsOneWidget);
      // Gönderim sürerken ikinci dosya başlatılamaz.
      expect(t.widget<OutlinedButton>(dugme).onPressed, isNull);

      m.izleyici.add(IzleyiciOlay('dosya', ad: 'rapor.pdf', gonderilen: 3 * 1024 * 1024, toplam: 4 * 1024 * 1024));
      await t.pump();
      expect(t.widget<LinearProgressIndicator>(cubuk).value, closeTo(0.75, 1e-9));

      m.izleyici.add(
          IzleyiciOlay('dosya', ad: 'rapor.pdf', gonderilen: 4 * 1024 * 1024, toplam: 4 * 1024 * 1024, bitti: true));
      await t.pump();
      expect(cubuk, findsNothing);
      expect(find.text('rapor.pdf gönderildi.'), findsOneWidget);
      expect(t.widget<OutlinedButton>(dugme).onPressed, isNotNull);

      // Hata: sebep gösterilir, çubuk kalkar.
      await t.tap(dugme);
      await t.pump();
      expect(cubuk, findsOneWidget);
      m.izleyici.add(IzleyiciOlay('dosya',
          ad: 'rapor.pdf',
          gonderilen: 1024,
          toplam: 4096,
          bitti: true,
          metin: 'Dosya bozuk geldi (SHA-256 uyuşmuyor); yeniden gönder.'));
      await t.pump();
      expect(cubuk, findsNothing);
      expect(find.text('rapor.pdf gönderilemedi: Dosya bozuk geldi (SHA-256 uyuşmuyor); yeniden gönder.'),
          findsOneWidget);
    });

    testWidgets('dosya izni yoksa gönder düğmesi pasif', (t) async {
      final m = await oturumAc(t);
      m.izleyici.add(IzleyiciOlay('kabul', kontrol: true, genislik: 200, yukseklik: 100));
      await t.pump();
      expect(t.widget<OutlinedButton>(find.byKey(const Key('oturum_dosya_gonder'))).onPressed, isNull);
      expect(find.byTooltip('Karşı taraf dosya almaya izin vermedi'), findsOneWidget);
    });

    testWidgets('karşı taraf reddederse sebep gösterilir', (t) async {
      final m = await oturumAc(t);
      m.izleyici.add(IzleyiciOlay('koptu', metin: 'Karşı taraf bağlantıyı reddetti.'));
      await t.pump();
      expect(find.text('Karşı taraf bağlantıyı reddetti.'), findsOneWidget);
      await t.tap(find.text('Geri dön'));
      await gec(t);
      expect(m.cagrilar, contains('izleyiciKapat'));
    });

    testWidgets('Kes düğmesi oturumu kapatır', (t) async {
      final m = await oturumAc(t);
      m.izleyici.add(IzleyiciOlay('kabul', genislik: 10, yukseklik: 10));
      await t.pump();
      await t.tap(find.byKey(const Key('oturum_kes')));
      await gec(t);
      expect(m.cagrilar, contains('izleyiciKapat'));
      expect(find.byKey(const Key('baglan_dugme')), findsOneWidget);
    });
  });

  group('Kayıtlı cihazlar', () {
    Future<SahteMotor> kayitliAc(WidgetTester t) async {
      t.view.physicalSize = const Size(1200, 900);
      t.view.devicePixelRatio = 1;
      addTearDown(t.view.reset);
      final m = SahteMotor()
        ..kayitli.addAll([
          KayitliCihaz('kimlik-ofis-0123456789abcdef', 'Ofis PC', DateTime.now().subtract(const Duration(minutes: 5))),
          KayitliCihaz('kimlik-ev', 'Ev Laptop', DateTime.now().subtract(const Duration(days: 2))),
        ]);
      await t.pumpWidget(AfuDeskUygulama(motor: m));
      return m;
    }

    testWidgets('kayıtlı cihazlar listesi', (t) async {
      final m = await kayitliAc(t);
      expect(find.text('Kayıtlı cihazlar'), findsOneWidget);
      expect(find.text('Ofis PC'), findsOneWidget);
      expect(find.text('Son görülme: 5 dk önce'), findsOneWidget);
      expect(find.text('Son görülme: 2 gün önce'), findsOneWidget);
      // Kullanıcı dostu: kimlik/IP/jeton gösterilmez.
      expect(find.textContaining('kimlik-'), findsNothing);
      await t.tap(find.byKey(const Key('kayitli_baglan_0')));
      await gec(t);
      expect(m.cagrilar, contains('kayitliBaglan:kimlik-ofis-0123456789abcdef'));
      expect(m.sonKod, isNull, reason: 'kod/parola istenmez');
      m.izleyici.add(IzleyiciOlay('bekliyor', ad: 'Ofis PC'));
      await gec(t);
      expect(find.textContaining('Ofis PC'), findsWidgets);
    });

    testWidgets('kayıtlı cihazı unut', (t) async {
      final m = await kayitliAc(t);
      await t.tap(find.byKey(const Key('kayitli_unut_0')));
      await t.pump();
      expect(m.cagrilar, contains('unut:kimlik-ofis-0123456789abcdef'));
      expect(find.text('Ofis PC'), findsNothing);
      expect(find.text('Ev Laptop'), findsOneWidget);
      await t.tap(find.byKey(const Key('kayitli_unut_0')));
      await t.pump();
      expect(find.text('Kayıtlı cihazlar'), findsNothing, reason: 'liste boşalınca bölüm gizlenir');
    });

    testWidgets('kayıt yoksa bölüm görünmez', (t) async {
      await ac(t);
      expect(find.text('Kayıtlı cihazlar'), findsNothing);
    });

    testWidgets('güvenilen cihazı kaldır', (t) async {
      t.view.physicalSize = const Size(1200, 1400);
      t.view.devicePixelRatio = 1;
      addTearDown(t.view.reset);
      final m = SahteMotor()
        ..guvenilen.add(KayitliCihaz('izleyici-1', 'Telefon', DateTime.now().subtract(const Duration(hours: 3))));
      await t.pumpWidget(AfuDeskUygulama(motor: m));
      await t.tap(find.byKey(const Key('secenek_ver')));
      await gec(t);
      m.host.add(const HostOlay('hazir', kod: ornekKod, parola: '1', erisim: 'x'));
      await t.pump();
      expect(find.text('Güvenilen cihazlar'), findsOneWidget);
      expect(find.text('Telefon'), findsOneWidget);
      expect(find.text('Son görülme: 3 saat önce'), findsOneWidget);
      await t.tap(find.byKey(const Key('guvenilen_kaldir_0')));
      await t.pump();
      expect(m.cagrilar, contains('kaldir:izleyici-1'));
      expect(find.text('Güvenilen cihazlar'), findsNothing);
      expect(find.textContaining('Bir dahaki sefere kod gerekecek'), findsOneWidget);
    });

    testWidgets('arka planda kayıtlı cihaz isteği onay penceresi açar (Bağlantı ver kapalıyken)', (t) async {
      final m = await ac(t);
      expect(m.cagrilar, contains('arkaPlan'), reason: 'uygulama açılınca arka planda dinler');
      m.host.add(const HostOlay('istek', ad: 'Telefon', kayitli: true));
      await gec(t);
      expect(find.text('Telefon bağlanmak istiyor'), findsOneWidget);
      expect(find.byKey(const Key('istek_kayitli')), findsOneWidget);
      await t.tap(find.byKey(const Key('istek_kabul')));
      await gec(t);
      expect(m.cagrilar, contains('kabul:true:true'));
      // Bağlanınca oturum ekranı (Kes düğmesiyle) kendiliğinden açılır.
      m.host.add(const HostOlay('baglandi', ad: 'Telefon', kontrol: true));
      await gec(t);
      expect(find.text('Telefon ekranını görüyor'), findsOneWidget);
      expect(find.byKey(const Key('ver_kes')), findsOneWidget);
    });

    testWidgets('arka plan isteği reddedilebilir; kodla gelen istekte rozet yok', (t) async {
      final m = await ac(t);
      m.host.add(const HostOlay('istek', ad: 'Yabancı'));
      await gec(t);
      expect(find.byKey(const Key('istek_kayitli')), findsNothing);
      await t.tap(find.byKey(const Key('istek_red')));
      await gec(t);
      expect(m.cagrilar, contains('red'));
    });

    testWidgets('kayıt olunca oturumda bildirilir', (t) async {
      final m = await ac(t);
      await t.tap(find.byKey(const Key('secenek_baglan')));
      await gec(t);
      await t.enterText(find.byType(TextField).first, ornekKod);
      await t.enterText(find.byType(TextField).last, '123456');
      await t.tap(find.widgetWithText(FilledButton, 'Bağlan'));
      await gec(t);
      m.izleyici.add(IzleyiciOlay('kaydedildi', ad: 'Ofis PC'));
      await t.pump();
      expect(find.textContaining('Ofis PC kaydedildi'), findsOneWidget);
    });

    test('göreli zaman', () {
      final s = DateTime(2026, 9, 25, 12);
      expect(goreliZaman(s.subtract(const Duration(seconds: 20)), simdi: s), 'az önce');
      expect(goreliZaman(s.subtract(const Duration(minutes: 7)), simdi: s), '7 dk önce');
      expect(goreliZaman(s.subtract(const Duration(hours: 5)), simdi: s), '5 saat önce');
      expect(goreliZaman(s.subtract(const Duration(days: 3)), simdi: s), '3 gün önce');
      expect(goreliZaman(DateTime(2026, 1, 2), simdi: s), '02.01.2026');
    });
  });

  group('Yardımcılar', () {
    test('tuş adları', () {
      expect(tusAdi(LogicalKeyboardKey.enter), 'Enter');
      expect(tusAdi(LogicalKeyboardKey.arrowLeft), 'ArrowLeft');
      expect(tusAdi(LogicalKeyboardKey.keyA), 'a');
      expect(tusAdi(LogicalKeyboardKey.digit5), '5');
      expect(tusAdi(LogicalKeyboardKey.mediaPlay), isNull);
    });

    test('dosya adı ve boyut metni', () {
      expect(dosyaAdi(r'C:\Belgeler\rapor.pdf'), 'rapor.pdf');
      expect(dosyaAdi('/home/a/b.txt'), 'b.txt');
      expect(boyutMetni(512), '512 B');
      expect(boyutMetni(1536), '1,5 KB');
      expect(boyutMetni(3 * 1024 * 1024), '3,0 MB');
    });

    test('hata metni temizlenir', () {
      expect(hataMetni(Exception('Parola yanlış.')), 'Parola yanlış.');
      expect(
        hataMetni('AnyhowException(Karşı bilgisayara ulaşılamadı.\nBağlantı veren bilgisayar açık ve internete bağlı mı?\n\nCaused by: x)'),
        "Karşı bilgisayara ulaşılamadı.\nBağlantı veren bilgisayar açık ve internete bağlı mı?",
      );
      final baglanti = hataMetni(Exception('Karşı tarafa ulaşılamadı. (deadline has elapsed)'));
      expect(baglanti, contains('Karşı bilgisayara ulaşılamadı.'));
      expect(baglanti, contains('internete bağlı mı?'));
      expect(baglanti, isNot(contains('deadline')));
      expect(baglanti, isNot(contains('(')));
    });
  });
}
