import 'dart:async';
import 'dart:ui' as ui;

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../klavye.dart';
import '../motor.dart';
import '../tema.dart';

/// Uzak ekran oturumu: bağlanır, onayı bekler, kareleri çizer, girdiyi gönderir.
class OturumSayfasi extends StatefulWidget {
  final Motor motor;
  final String kod;
  final String parola;
  const OturumSayfasi({super.key, required this.motor, required this.kod, required this.parola});

  @override
  State<OturumSayfasi> createState() => _OturumDurum();
}

enum _Asama { baglaniyor, onayBekleniyor, bagli, bitti }

class _OturumDurum extends State<OturumSayfasi> {
  StreamSubscription<IzleyiciOlay>? _abonelik;
  _Asama _asama = _Asama.baglaniyor;
  String _karsiAd = '';
  String _mesaj = '';
  bool _kontrol = false;
  ui.Image? _kare;
  final _odak = FocusNode();

  @override
  void initState() {
    super.initState();
    _abonelik = widget.motor
        .baglan(kod: widget.kod, parola: widget.parola, ad: widget.motor.cihazAdi())
        .listen(_olay, onError: (Object e) => _bitir(hataMetni(e)), onDone: () {
      if (_asama != _Asama.bitti) _bitir('Bağlantı kapandı.');
    });
  }

  void _bitir(String m) {
    if (!mounted) return;
    setState(() {
      _asama = _Asama.bitti;
      _mesaj = m;
    });
  }

  void _olay(IzleyiciOlay o) {
    if (!mounted) return;
    switch (o.tur) {
      case 'bekliyor':
        setState(() {
          _asama = _Asama.onayBekleniyor;
          _karsiAd = o.ad;
        });
      case 'kabul':
        setState(() {
          _asama = _Asama.bagli;
          _kontrol = o.kontrol;
        });
        _odak.requestFocus();
      case 'kare':
        ui.decodeImageFromPixels(o.rgba, o.genislik, o.yukseklik, ui.PixelFormat.rgba8888, (img) {
          if (!mounted) {
            img.dispose();
            return;
          }
          final eski = _kare;
          setState(() => _kare = img);
          // Eski görüntü bu karede hâlâ çizim katmanında olabilir: kare çizildikten sonra bırak.
          WidgetsBinding.instance.addPostFrameCallback((_) {
            eski?.dispose();
            widget.motor.kareCizildi();
          });
        });
      case 'koptu':
      case 'hata':
        _bitir(o.metin);
    }
  }

  @override
  void dispose() {
    _abonelik?.cancel();
    widget.motor.izleyiciKapat();
    final son = _kare;
    _kare = null;
    WidgetsBinding.instance.addPostFrameCallback((_) => son?.dispose());
    _odak.dispose();
    super.dispose();
  }

  // --- girdi ---

  Offset? _normalize(Offset yerel, Size alan) {
    final k = _kare;
    if (k == null) return null;
    // Görüntü alanın ortasına en-boy oranı korunarak yerleşir (BoxFit.contain).
    final olcek = (alan.width / k.width).clamp(0.0, alan.height / k.height);
    final g = k.width * olcek, y = k.height * olcek;
    final sol = (alan.width - g) / 2, ust = (alan.height - y) / 2;
    final nx = (yerel.dx - sol) / g, ny = (yerel.dy - ust) / y;
    if (nx < 0 || nx > 1 || ny < 0 || ny > 1) return null;
    return Offset(nx, ny);
  }

  void _konum(Offset yerel, Size alan) {
    if (!_kontrol) return;
    final n = _normalize(yerel, alan);
    if (n != null) widget.motor.girdi(Girdi('konum', x: n.dx, y: n.dy));
  }

  String _tus(int dugmeler) =>
      dugmeler & kSecondaryMouseButton != 0 ? 'sag' : (dugmeler & kMiddleMouseButton != 0 ? 'orta' : 'sol');

  int _sonDugme = 0;

  KeyEventResult _klavye(FocusNode _, KeyEvent e) {
    if (!_kontrol || e is KeyRepeatEvent) return _kontrol ? KeyEventResult.handled : KeyEventResult.ignored;
    final ad = tusAdi(e.logicalKey);
    if (ad == null) return KeyEventResult.ignored;
    widget.motor.girdi(Girdi('tus', ad: ad, basili: e is KeyDownEvent));
    return KeyEventResult.handled;
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: Text(_karsiAd.isEmpty ? 'Uzak ekran' : _karsiAd),
        actions: [
          if (_asama == _Asama.bagli)
            Padding(
              padding: const EdgeInsets.only(right: 8),
              child: Chip(
                key: const Key('oturum_kontrol'),
                avatar: Icon(_kontrol ? Icons.mouse : Icons.visibility, size: 16),
                label: Text(_kontrol ? 'Kontrol açık' : 'Yalnız izleme'),
              ),
            ),
          if (_asama != _Asama.bitti)
            Padding(
              padding: const EdgeInsets.only(right: 12),
              child: FilledButton.icon(
                key: const Key('oturum_kes'),
                style: FilledButton.styleFrom(backgroundColor: Renk.tehlike, minimumSize: const Size(0, 38)),
                onPressed: () => Navigator.pop(context),
                icon: const Icon(Icons.link_off, size: 18),
                label: const Text('Kes'),
              ),
            ),
        ],
      ),
      body: _govde(),
    );
  }

  Widget _govde() {
    switch (_asama) {
      case _Asama.baglaniyor:
        return const _Bilgi(anahtar: Key('oturum_baglaniyor'), yukleniyor: true, metin: 'Bağlanılıyor…');
      case _Asama.onayBekleniyor:
        return _Bilgi(
            anahtar: const Key('oturum_onay'),
            yukleniyor: true,
            metin: '$_karsiAd onayı bekleniyor…\nKarşı tarafın ekranında “Kabul et”e basması gerekiyor.');
      case _Asama.bitti:
        return Center(
          child: Column(mainAxisSize: MainAxisSize.min, children: [
            const Icon(Icons.link_off, size: 40, color: Renk.soluk),
            const SizedBox(height: 12),
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 24),
              child: Text(_mesaj, key: const Key('oturum_mesaj'), textAlign: TextAlign.center),
            ),
            const SizedBox(height: 16),
            FilledButton(onPressed: () => Navigator.pop(context), child: const Text('Geri dön')),
          ]),
        );
      case _Asama.bagli:
        final k = _kare;
        return Container(
          color: Colors.black,
          child: LayoutBuilder(builder: (context, c) {
            final alan = Size(c.maxWidth, c.maxHeight);
            return Focus(
              focusNode: _odak,
              autofocus: true,
              onKeyEvent: _klavye,
              child: MouseRegion(
                cursor: _kontrol ? SystemMouseCursors.precise : SystemMouseCursors.basic,
                child: Listener(
                  key: const Key('oturum_ekran'),
                  onPointerHover: (e) => _konum(e.localPosition, alan),
                  onPointerMove: (e) => _konum(e.localPosition, alan),
                  onPointerDown: (e) {
                    _odak.requestFocus();
                    if (!_kontrol) return;
                    _konum(e.localPosition, alan);
                    _sonDugme = e.buttons;
                    widget.motor.girdi(Girdi('fare', ad: _tus(e.buttons), basili: true));
                  },
                  onPointerUp: (e) {
                    if (!_kontrol) return;
                    widget.motor.girdi(Girdi('fare', ad: _tus(_sonDugme), basili: false));
                  },
                  onPointerSignal: (e) {
                    if (!_kontrol || e is! PointerScrollEvent) return;
                    final dy = e.scrollDelta.dy.sign.toInt(), dx = e.scrollDelta.dx.sign.toInt();
                    if (dx != 0 || dy != 0) widget.motor.girdi(Girdi('kaydir', dx: dx, dy: dy));
                  },
                  child: SizedBox.expand(
                    child: k == null
                        ? const Center(child: CircularProgressIndicator())
                        : RawImage(key: const Key('oturum_kare'), image: k, fit: BoxFit.contain, filterQuality: FilterQuality.medium),
                  ),
                ),
              ),
            );
          }),
        );
    }
  }
}

class _Bilgi extends StatelessWidget {
  final Key anahtar;
  final bool yukleniyor;
  final String metin;
  const _Bilgi({required this.anahtar, required this.yukleniyor, required this.metin});

  @override
  Widget build(BuildContext context) => Center(
        key: anahtar,
        child: Column(mainAxisSize: MainAxisSize.min, children: [
          if (yukleniyor) const CircularProgressIndicator(),
          const SizedBox(height: 16),
          Text(metin, textAlign: TextAlign.center, style: const TextStyle(color: Renk.soluk)),
        ]),
      );
}
