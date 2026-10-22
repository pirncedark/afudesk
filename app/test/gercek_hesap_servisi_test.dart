import 'dart:async';
import 'package:afudesk/hesap/gercek_hesap_servisi.dart';
import 'package:afudesk/motor.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  late StreamController<HostOlay> host;
  late StreamController<IzleyiciOlay> viewer;
  late GercekHesapServisi servis;
  late List<String> islemler;
  late int kesildi;

  setUp(() {
    host = StreamController<HostOlay>.broadcast(sync: true);
    viewer = StreamController<IzleyiciOlay>();
    islemler = [];
    kesildi = 0;
    servis = GercekHesapServisi(
      islem: (op, data) async {
        islemler.add(op);
        return switch (op) {
          'baslat' => '{"url":"https://accounts.google.com/"}',
          'tamamla' => '{"ad":"Google hesabın"}',
          'liste' => '[{"id":"ev","ad":"Ev","cevrimici":true}]',
          _ => '{}',
        };
      },
      googleBaglan: (id) {
        expect(id, 'ev');
        return viewer.stream;
      },
      kes: () async {
        kesildi++;
      },
      hostOlaylari: host.stream,
      platformGirisi: (nonce, {bool sessiz = false}) async =>
          throw StateError('Masaüstü tarayıcı kullanır'),
      tarayiciAc: (_) async => true,
    );
  });

  tearDown(() async {
    servis.dispose();
    await host.close();
  });

  test('gerçek adaptör giriş ve Drive listesini köprüden alır', () async {
    await servis.girisYap();
    expect(servis.hesap, isNotNull);
    expect((await servis.bilgisayarlarim()).single.ad, 'Ev');
    expect(islemler, ['baslat', 'tamamla', 'liste']);
  });

  test('gelen bağlantı şeridi gerçek kopma olayına kadar kalır', () async {
    host.add(const HostOlay('baglandi', ad: 'Telefon'));
    expect(servis.baglantilar.single.ad, 'Telefon');
    await servis.baglantiyiKes();
    expect(kesildi, 1);
    expect(servis.baglantilar.single.ad, 'Telefon');
    host.add(const HostOlay('koptu'));
    expect(servis.baglantilar, isEmpty);
  });

  test(
    'giden bağlantı yalnız kabulden sonra görünür ve akış bitince temizlenir',
    () async {
      await servis.girisYap();
      StreamSubscription<IzleyiciOlay>? subscription;
      servis.oturumAc = (cihaz, ac, {required terminalleBasla}) {
        expect(terminalleBasla, isFalse);
        subscription = ac().listen((_) {});
      };
      await servis.baglan('ev');
      expect(servis.baglantilar, isEmpty);
      viewer.add(IzleyiciOlay('kabul'));
      await Future<void>.delayed(Duration.zero);
      expect(servis.baglantilar.single.id, 'ev');
      await viewer.close();
      await Future<void>.delayed(Duration.zero);
      expect(servis.baglantilar, isEmpty);
      await subscription!.cancel();
    },
  );

  test('çıkış çekirdek yetkisini kaldırır ve bağlantıyı keser', () async {
    await servis.girisYap();
    await servis.cikisYap();
    expect(servis.hesap, isNull);
    expect(islemler.last, 'cikis');
    expect(kesildi, 1);
  });
}
