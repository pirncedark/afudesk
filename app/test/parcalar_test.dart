import 'dart:typed_data';
import 'dart:async';
import 'dart:ui' as ui;
import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:afudesk/sayfalar/terminal_modu.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:afudesk/motor.dart';
import 'package:afudesk/sayfalar/oturum.dart';
import 'sahte_motor.dart';

Future<ui.Image> pixel(int r, int g, int b) {
  final c = Completer<ui.Image>();
  ui.decodeImageFromPixels(
    Uint8List.fromList([r, g, b, 255]),
    1,
    1,
    ui.PixelFormat.rgba8888,
    c.complete,
  );
  return c.future;
}

void main() {
  testWidgets('position-only cursor messages retain the shape hotspot', (
    tester,
  ) async {
    final motor = SahteMotor();
    await tester.pumpWidget(MaterialApp(home: OturumSayfasi(motor: motor)));
    motor.izleyici.add(IzleyiciOlay('kabul', kontrol: true));
    await tester.pump();
    motor.izleyici.add(
      IzleyiciOlay(
        'imlec',
        genislik: 1,
        yukseklik: 1,
        rgba: Uint8List.fromList([255, 255, 255, 255]),
        metin: '{"x":0.5,"y":0.5,"gorunur":true,"sicak_x":1,"sicak_y":1}',
      ),
    );
    // Image decoding completes outside the fake widget clock.
    await tester.runAsync(
      () => Future<void>.delayed(const Duration(milliseconds: 50)),
    );
    final packet = Uint8List(20);
    ByteData.sublistView(packet).setUint32(8, 1, Endian.little);
    ByteData.sublistView(packet).setUint32(12, 1, Endian.little);
    packet.setRange(16, 20, [0, 0, 0, 255]);
    motor.izleyici.add(
      IzleyiciOlay(
        'parcalar',
        genislik: 1,
        yukseklik: 1,
        rgba: packet,
        metin: '{"tam":true}',
      ),
    );
    await tester.runAsync(
      () => Future<void>.delayed(const Duration(milliseconds: 50)),
    );
    await tester.pump();
    motor.izleyici.add(
      IzleyiciOlay(
        'imlec',
        metin: '{"x":0.7,"y":0.6,"gorunur":true,"sicak_x":0,"sicak_y":0}',
      ),
    );
    await tester.pump();
    final dynamic painter = tester
        .widget<CustomPaint>(find.byKey(const Key('oturum_kare')))
        .painter;
    expect(painter.goruntu.hotspot, const Offset(1, 1));
    expect(painter.goruntu.cursorPosition, const Offset(.7, .6));
    await tester.pumpWidget(const SizedBox());
    await tester.pump();
  });
  testWidgets(
    'tile painter preserves untouched tile and draws cursor separately',
    (tester) async {
      final images = (await tester.runAsync(
        () => Future.wait([
          pixel(255, 0, 0),
          pixel(0, 0, 255),
          pixel(0, 255, 0),
          pixel(255, 255, 255),
        ]),
      ))!;
      final red = images[0],
          blue = images[1],
          green = images[2],
          cursor = images[3];
      final key = GlobalKey();
      Future<Uint8List> render(UzakGoruntu frame) async {
        await tester.pumpWidget(
          MaterialApp(
            home: Center(
              child: RepaintBoundary(
                key: key,
                child: SizedBox(
                  width: 2,
                  height: 1,
                  child: frame.widget(fit: BoxFit.fill),
                ),
              ),
            ),
          ),
        );
        await tester.pump();
        final boundary =
            key.currentContext!.findRenderObject()! as RenderRepaintBoundary;
        final image = await boundary.toImage(pixelRatio: 1);
        final data = (await image.toByteData())!.buffer.asUint8List();
        image.dispose();
        return data;
      }

      final tiles = <Offset, ui.Image>{
        Offset.zero: red,
        const Offset(1, 0): blue,
      };
      expect(await render(UzakGoruntu(2, 1, tiles)), [
        255,
        0,
        0,
        255,
        0,
        0,
        255,
        255,
      ]);
      final updated = {...tiles, Offset.zero: green};
      expect(await render(UzakGoruntu(2, 1, updated)), [
        0,
        255,
        0,
        255,
        0,
        0,
        255,
        255,
      ]);
      expect(
        await render(
          UzakGoruntu(
            2,
            1,
            updated,
            cursor: cursor,
            cursorVisible: true,
            cursorPosition: const Offset(.5, 0),
          ),
        ),
        [0, 255, 0, 255, 255, 255, 255, 255],
      );
      await tester.pumpWidget(const SizedBox());
      for (final image in [red, blue, green, cursor]) {
        image.dispose();
      }
    },
  );
  test('empty packet has no tiles', () {
    expect(DosemePaket.oku(Uint8List(0)), isEmpty);
  });
  test('metadata preserves viewer clock and full reset flag', () {
    final event = IzleyiciOlay(
      'parcalar',
      metin: '{"tam":true,"girdi_ms":123456789}',
    );
    expect(event.ek['tam'], isTrue);
    expect(event.ek['girdi_ms'], 123456789);
    expect(IzleyiciOlay('parcalar', metin: 'bad').ek, isEmpty);
  });
  test(
    'delta packet preserves tile coordinates and rejects truncated payload',
    () {
      final p = Uint8List(20);
      final h = ByteData.sublistView(p);
      h.setUint32(0, 64, Endian.little);
      h.setUint32(4, 128, Endian.little);
      h.setUint32(8, 1, Endian.little);
      h.setUint32(12, 1, Endian.little);
      p.setRange(16, 20, [10, 20, 30, 255]);
      final tile = DosemePaket.oku(p).single;
      expect([tile.x, tile.y, tile.gen, tile.yuk], [64, 128, 1, 1]);
      expect(tile.rgba, [10, 20, 30, 255]);
      p[16] = 99;
      expect(tile.rgba[0], 99); // packet views avoid another RGBA copy
      expect(DosemePaket.oku(Uint8List.fromList([...p, ...p])), hasLength(2));
      expect(() => DosemePaket.oku(Uint8List(15)), throwsFormatException);
      expect(() => DosemePaket.oku(Uint8List(16)), throwsFormatException);
      expect(
        () => DosemePaket.oku(Uint8List.sublistView(p, 0, 19)),
        throwsFormatException,
      );
    },
  );
}
