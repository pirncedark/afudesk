import 'dart:ui' as ui;

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';

/// Aynı oturumun kırpılmış karelerini gösterir; yeni bağlantı oluşturmaz.
class TerminalModu extends StatelessWidget {
  final ui.Image? kare;
  final UzakGoruntu? goruntu;
  final Widget yaziAlani;
  final bool ctrl;
  final bool klavyeAcik;
  final VoidCallback kapat, klavye, ctrlDegistir, kopyayiKes, yapistir;
  final ValueChanged<String> tus;
  final void Function(Offset konum, Size alan) konum;
  final ValueChanged<int> fare;
  final ValueChanged<PointerScrollEvent> kaydir;

  const TerminalModu({
    super.key,
    this.kare,
    this.goruntu,
    required this.yaziAlani,
    required this.ctrl,
    required this.klavyeAcik,
    required this.kapat,
    required this.klavye,
    required this.ctrlDegistir,
    required this.kopyayiKes,
    required this.yapistir,
    required this.tus,
    required this.konum,
    required this.fare,
    required this.kaydir,
  });

  @override
  Widget build(BuildContext context) {
    Widget dugme(String ad, String etiket, VoidCallback islem) => Padding(
      padding: const EdgeInsets.symmetric(horizontal: 2),
      child: OutlinedButton(
        key: Key('terminal_$ad'),
        style: OutlinedButton.styleFrom(
          minimumSize: const Size(44, 40),
          padding: const EdgeInsets.symmetric(horizontal: 10),
          backgroundColor: ad == 'ctrl' && ctrl
              ? Theme.of(context).colorScheme.primaryContainer
              : null,
        ),
        onPressed: islem,
        child: Text(etiket),
      ),
    );
    return PopScope(
      canPop: false,
      onPopInvokedWithResult: (didPop, _) {
        if (!didPop) kapat();
      },
      child: Scaffold(
        key: const Key('terminal_modu'),
        backgroundColor: Colors.black,
        resizeToAvoidBottomInset: true,
        body: SafeArea(
          child: Column(
            children: [
              Expanded(
                child: LayoutBuilder(
                  builder: (context, c) {
                    final k = goruntu;
                    final gen = k?.width ?? kare?.width;
                    final yuk = k?.height ?? kare?.height;
                    if (gen == null || yuk == null) {
                      return const Center(child: CircularProgressIndicator());
                    }
                    final alan = Size(c.maxWidth, c.maxWidth * yuk / gen);
                    // reverse keeps the prompt visible when the software keyboard shrinks
                    // the viewport; the user can scroll back to earlier lines.
                    return SingleChildScrollView(
                      key: const Key('terminal_kaydirma'),
                      reverse: true,
                      child: Listener(
                        onPointerHover: (e) => konum(e.localPosition, alan),
                        onPointerDown: (e) {
                          konum(e.localPosition, alan);
                          if (e.kind == PointerDeviceKind.mouse) {
                            fare(e.buttons);
                          }
                        },
                        onPointerMove: (e) {
                          if (e.kind != PointerDeviceKind.mouse) return;
                          konum(e.localPosition, alan);
                          fare(e.buttons);
                        },
                        onPointerUp: (e) {
                          if (e.kind == PointerDeviceKind.mouse) {
                            fare(e.buttons);
                          }
                        },
                        onPointerCancel: (_) => fare(0),
                        onPointerSignal: (e) {
                          if (e is PointerScrollEvent) kaydir(e);
                        },
                        child: GestureDetector(
                          supportedDevices: const {
                            ui.PointerDeviceKind.touch,
                            ui.PointerDeviceKind.stylus,
                          },
                          onTapUp: (e) {
                            konum(e.localPosition, alan);
                            fare(kPrimaryMouseButton);
                            fare(0);
                          },
                          child: SizedBox(
                            width: alan.width,
                            height: alan.height,
                            child: k != null
                                ? k.widget(
                                    key: const Key('terminal_kare'),
                                    fit: BoxFit.fill,
                                  )
                                : RawImage(
                                    key: const Key('terminal_kare'),
                                    image: kare,
                                    fit: BoxFit.fill,
                                  ),
                          ),
                        ),
                      ),
                    );
                  },
                ),
              ),
              Material(
                child: Focus(
                  canRequestFocus: false,
                  descendantsAreFocusable: false,
                  child: Row(
                    children: [
                      IconButton(
                        tooltip: 'Masaüstüne dön',
                        onPressed: kapat,
                        icon: const Icon(Icons.arrow_back),
                      ),
                      IconButton.filled(
                        key: const Key('terminal_klavye'),
                        tooltip: 'Klavye',
                        onPressed: klavye,
                        icon: Icon(
                          klavyeAcik ? Icons.keyboard_hide : Icons.keyboard,
                        ),
                      ),
                      Expanded(
                        child: SingleChildScrollView(
                          scrollDirection: Axis.horizontal,
                          child: Row(
                            children: [
                              dugme('esc', 'Esc', () => tus('Escape')),
                              dugme('tab', 'Tab', () => tus('Tab')),
                              dugme('ctrl', 'Ctrl', ctrlDegistir),
                              dugme('yukari', '↑', () => tus('ArrowUp')),
                              dugme('asagi', '↓', () => tus('ArrowDown')),
                              dugme('sol', '←', () => tus('ArrowLeft')),
                              dugme('sag', '→', () => tus('ArrowRight')),
                              dugme('enter', 'Enter', () => tus('Enter')),
                              dugme('ctrl_c', 'Ctrl+C', kopyayiKes),
                              dugme('yapistir', 'Yapıştır', yapistir),
                            ],
                          ),
                        ),
                      ),
                    ],
                  ),
                ),
              ),
              yaziAlani,
            ],
          ),
        ),
      ),
    );
  }
}

/// Immutable tile snapshot. Only changed tiles are uploaded to the GPU.
class UzakGoruntu {
  final int width, height;
  final Map<Offset, ui.Image> tiles;
  final ui.Image? cursor;
  final Offset cursorPosition, hotspot;
  final bool cursorVisible;
  const UzakGoruntu(
    this.width,
    this.height,
    this.tiles, {
    this.cursor,
    this.cursorPosition = Offset.zero,
    this.hotspot = Offset.zero,
    this.cursorVisible = false,
  });
  Widget widget({Key? key, BoxFit fit = BoxFit.contain}) => CustomPaint(
    key: key,
    painter: _ParcaBoyaci(this, fit),
    size: Size.infinite,
  );
  void dispose() {
    for (final image in tiles.values.toSet()) {
      image.dispose();
    }
  }
}

class _ParcaBoyaci extends CustomPainter {
  final UzakGoruntu goruntu;
  final BoxFit fit;
  _ParcaBoyaci(this.goruntu, this.fit);
  @override
  void paint(Canvas canvas, Size size) {
    final src = Size(goruntu.width.toDouble(), goruntu.height.toDouble());
    if (src.isEmpty || size.isEmpty) return;
    final fitted = applyBoxFit(fit, src, size);
    final dst = Alignment.center.inscribe(
      fitted.destination,
      Offset.zero & size,
    );
    canvas.save();
    canvas.clipRect(dst);
    canvas.translate(dst.left, dst.top);
    canvas.scale(dst.width / src.width, dst.height / src.height);
    final paint = Paint()..filterQuality = FilterQuality.none;
    for (final tile in goruntu.tiles.entries) {
      canvas.drawImage(tile.value, tile.key, paint);
    }
    if (goruntu.cursorVisible && goruntu.cursor != null) {
      canvas.drawImage(
        goruntu.cursor!,
        Offset(
          goruntu.cursorPosition.dx * src.width - goruntu.hotspot.dx,
          goruntu.cursorPosition.dy * src.height - goruntu.hotspot.dy,
        ),
        paint,
      );
    }
    canvas.restore();
  }

  @override
  bool shouldRepaint(covariant _ParcaBoyaci oldDelegate) => true;
}
