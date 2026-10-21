import 'dart:ui' as ui;

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';

/// Aynı oturumun kırpılmış karelerini gösterir; yeni bağlantı oluşturmaz.
class TerminalModu extends StatelessWidget {
  final ui.Image? kare;
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
    required this.kare,
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
                    final k = kare;
                    if (k == null)
                      return const Center(child: CircularProgressIndicator());
                    final alan = Size(
                      c.maxWidth,
                      c.maxWidth * k.height / k.width,
                    );
                    // reverse keeps the prompt visible when the software keyboard shrinks
                    // the viewport; the user can scroll back to earlier lines.
                    return SingleChildScrollView(
                      key: const Key('terminal_kaydirma'),
                      reverse: true,
                      child: Listener(
                        onPointerHover: (e) => konum(e.localPosition, alan),
                        onPointerDown: (e) {
                          konum(e.localPosition, alan);
                          if (e.kind == PointerDeviceKind.mouse)
                            fare(e.buttons);
                        },
                        onPointerMove: (e) {
                          if (e.kind != PointerDeviceKind.mouse) return;
                          konum(e.localPosition, alan);
                          fare(e.buttons);
                        },
                        onPointerUp: (e) {
                          if (e.kind == PointerDeviceKind.mouse)
                            fare(e.buttons);
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
                            child: RawImage(
                              key: const Key('terminal_kare'),
                              image: k,
                              fit: BoxFit.fill,
                              filterQuality: FilterQuality.medium,
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
