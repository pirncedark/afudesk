import 'package:afudesk/gorsel_paylas.dart';
import 'package:afudesk/motor.dart';
import 'package:afudesk/sayfalar/oturum.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'sahte_motor.dart';

void main() {
  const kanal = MethodChannel('afudesk/gorsel');
  late GorselPaylas paylas;
  late SahteMotor motor;
  late List<String> ilkYollar;

  setUp(() {
    ilkYollar = [];
    paylas = GorselPaylas();
    motor = SahteMotor()..dokunmatik = true;
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(kanal, (call) async => ilkYollar);
  });
  tearDown(() {
    paylas.dispose();
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(kanal, null);
  });

  Future<void> gorsel(WidgetTester t, String yol) async {
    await TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .handlePlatformMessage(
          kanal.name,
          kanal.codec.encodeMethodCall(MethodCall('gorsel', yol)),
          (_) {},
        );
    await t.pump();
  }

  Future<void> ac(WidgetTester t, {bool kabul = true, bool izin = true}) async {
    await t.pumpWidget(
      MaterialApp(
        home: OturumSayfasi(motor: motor, gorselPaylas: paylas),
      ),
    );
    await t.pump();
    if (kabul) {
      motor.izleyici.add(IzleyiciOlay('kabul', dosya: izin));
      await t.pump();
    }
  }

  testWidgets('paylasim kanali gorseli mevcut aktarimla gonderir', (t) async {
    await ac(t);
    expect(find.byKey(const Key('oturum_gorsel_gonder')), findsOneWidget);
    await gorsel(t, '/cache/ekran.png');
    expect(motor.gonderilenDosyalar, ['/cache/ekran.png']);
    expect(
      find.text("Görsel PC'ye gönderildi; PC'de Ctrl+V (terminalde Alt+V) ile yapıştırabilirsin."),
      findsNothing,
    );
    motor.izleyici.add(IzleyiciOlay('dosya', ad: 'ekran.png', bitti: true));
    await t.pump();
    expect(
      find.text("Görsel PC'ye gönderildi; PC'de Ctrl+V (terminalde Alt+V) ile yapıştırabilirsin."),
      findsOneWidget,
    );
    await t.pumpWidget(const SizedBox.shrink());
  });

  testWidgets('soguk baslangic gorseli baglanti ve izin bekler', (t) async {
    ilkYollar = ['/cache/ilk.png'];
    await paylas.baslat();
    await ac(t, kabul: false);
    expect(motor.gonderilenDosyalar, isEmpty);
    motor.izleyici.add(IzleyiciOlay('kabul', dosya: false));
    await t.pump();
    expect(motor.gonderilenDosyalar, isEmpty);
    motor.izleyici.add(IzleyiciOlay('kabul', dosya: true));
    await t.pump();
    expect(motor.gonderilenDosyalar, ['/cache/ilk.png']);
    await t.pumpWidget(const SizedBox.shrink());
  });

  testWidgets('aktif aktarim varken gorseller sirayla gonderilir', (t) async {
    await ac(t);
    await gorsel(t, '/cache/bir.png');
    await gorsel(t, '/cache/iki.jpg');
    expect(motor.gonderilenDosyalar, ['/cache/bir.png']);
    motor.izleyici.add(IzleyiciOlay('dosya', ad: 'bir.png', bitti: true));
    await t.pump();
    expect(motor.gonderilenDosyalar, ['/cache/bir.png', '/cache/iki.jpg']);
    await t.pumpWidget(const SizedBox.shrink());
  });

  testWidgets('dugme yalniz gorsel secimini ister', (t) async {
    await ac(t);
    motor.secilecekDosya = '/cache/galeri.jpg';
    await t.tap(find.byKey(const Key('oturum_gorsel_gonder')));
    await t.pump();
    expect(motor.yalnizGorselSecildi, isTrue);
    expect(motor.gonderilenDosyalar, ['/cache/galeri.jpg']);
    await t.pumpWidget(const SizedBox.shrink());
  });

  testWidgets('oturum ekrani yokken gelen paylasim kaybolmaz', (t) async {
    await paylas.baslat();
    await gorsel(t, '/cache/bekleyen.png');
    expect(motor.gonderilenDosyalar, isEmpty);
    await ac(t);
    expect(motor.gonderilenDosyalar, ['/cache/bekleyen.png']);
    await t.pumpWidget(const SizedBox.shrink());
  });

  testWidgets('yeniden baglanirken gorsel bekler', (t) async {
    await ac(t);
    motor.izleyici.add(IzleyiciOlay('yeniden', metin: 'Yeniden bağlanılıyor…'));
    await t.pump();
    await gorsel(t, '/cache/yeniden.png');
    expect(motor.gonderilenDosyalar, isEmpty);
    motor.izleyici.add(IzleyiciOlay('istatistik'));
    await t.pump();
    expect(motor.gonderilenDosyalar, ['/cache/yeniden.png']);
    await t.pumpWidget(const SizedBox.shrink());
  });

  testWidgets('basarisiz aktarim basari mesaji gostermez', (t) async {
    await ac(t);
    await gorsel(t, '/cache/hata.png');
    motor.izleyici.add(
      IzleyiciOlay('dosya', ad: 'hata.png', bitti: true, metin: 'reddedildi'),
    );
    await t.pump();
    expect(find.text('Görsel gönderilemedi. Yeniden dene.'), findsOneWidget);
    expect(
      find.text("Görsel PC'ye gönderildi; PC'de Ctrl+V (terminalde Alt+V) ile yapıştırabilirsin."),
      findsNothing,
    );
    await t.pumpWidget(const SizedBox.shrink());
  });
}
