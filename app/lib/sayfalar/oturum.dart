import 'dart:async';
import 'dart:ui' as ui;

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../dokunma.dart';
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
  bool _dosyaIzni = false;
  _Aktarim? _aktarim;
  bool _p2 = false;
  int _rtt = -1, _fps = 0, _gecikme = 0;
  // Veri relay üzerinden mi geçiyor (doğrudan yol kurulamadı)?
  bool _aktarmali = false;
  // Bağlantı koptu, kendiliğinden yeniden bağlanılıyor: gösterilecek metin.
  String? _yeniden;
  ui.Image? _kare;
  final _odak = FocusNode();
  // Dokunmatik kip
  final _dokunma = DokunmaCevirici();
  Timer? _dokunmaSaat;
  final _saat = Stopwatch()..start();
  final _yaziOdak = FocusNode();
  final _yazi = TextEditingController(text: _gozcu);
  bool _klavyeAcik = false;
  static const _gozcu = '\u200b';

  @override
  void initState() {
    super.initState();
    if (widget.motor.dokunmatik) {
      // Telefonda bilgisayar ekranı yatayda ve tam ekranda çok daha okunaklı.
      SystemChrome.setPreferredOrientations([DeviceOrientation.landscapeLeft, DeviceOrientation.landscapeRight]);
      SystemChrome.setEnabledSystemUIMode(SystemUiMode.immersiveSticky);
    }
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
          _yeniden = null;
          _asama = _Asama.bagli;
          _kontrol = o.kontrol;
          _dosyaIzni = o.dosya;
        });
        _odak.requestFocus();
        if (widget.motor.dokunmatik && o.kontrol) {
          // Uzun basışı zamanında yakalamak için düzenli yokla.
          _dokunmaSaat?.cancel();
          _dokunmaSaat = Timer.periodic(const Duration(milliseconds: 100), (_) => _gonder(_dokunma.zaman(_ms)));
        }
      case 'kol':
        setState(() => _p2 = true);
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
      case 'istatistik':
        setState(() {
          _rtt = o.rttMs;
          _fps = o.fps;
          _gecikme = o.gecikmeMs;
          _aktarmali = o.metin == 'relay';
          _yeniden = null;
        });
      case 'yeniden':
        setState(() => _yeniden = o.metin);
      case 'pano':
        // Masaüstünde çekirdek panoya zaten yazdı; dokunmatik cihazda (Android) çekirdeğin
        // pano erişimi yok, metni arayüz yazar.
        if (widget.motor.dokunmatik) Clipboard.setData(ClipboardData(text: o.metin));
        ScaffoldMessenger.of(context)
          ..hideCurrentSnackBar()
          ..showSnackBar(const SnackBar(content: Text('Pano güncellendi'), duration: Duration(seconds: 1)));
case 'dosya':
        _dosyaOlayi(o);

      case 'koptu':
      case 'hata':
        _bitir(o.metin);
    }
  }

  // --- dosya gönderme ---

  Future<void> _dosyaGonder() async {
    final yol = await widget.motor.dosyaSec();
    if (yol == null || !mounted) return;
    setState(() => _aktarim = _Aktarim(dosyaAdi(yol)));
    await widget.motor.dosyaGonder(yol);
  }

  void _dosyaOlayi(IzleyiciOlay o) {
    if (!o.bitti) {
      setState(() => _aktarim = _Aktarim(o.ad, gonderilen: o.gonderilen, toplam: o.toplam));
      return;
    }
    setState(() => _aktarim = null);
    final basarili = o.metin.isEmpty;
    ScaffoldMessenger.of(context)
      ..hideCurrentSnackBar()
      ..showSnackBar(SnackBar(
        key: const Key('oturum_dosya_sonuc'),
        content: Text(basarili ? '${o.ad} gönderildi.' : '${o.ad} gönderilemedi: ${o.metin}'),
        backgroundColor: basarili ? null : Renk.tehlike,
        duration: Duration(seconds: basarili ? 3 : 6),
      ));
  }

  Widget _dosyaCubugu(_Aktarim a) {
    final oran = a.toplam > 0 ? (a.gonderilen / a.toplam).clamp(0.0, 1.0) : null;
    final metin = oran == null
        ? '${a.ad} hazırlanıyor…'
        : '${a.ad} — %${(oran * 100).floor()} (${boyutMetni(a.gonderilen)} / ${boyutMetni(a.toplam)})';
    return Material(
      color: Renk.yuzey,
      child: SafeArea(
        top: false,
        child: Padding(
          padding: const EdgeInsets.fromLTRB(16, 8, 16, 10),
          child: Column(mainAxisSize: MainAxisSize.min, crossAxisAlignment: CrossAxisAlignment.start, children: [
            Row(children: [
              const Icon(Icons.upload_file_rounded, size: 18, color: Renk.soluk),
              const SizedBox(width: 6),
              Expanded(
                child: Text(metin,
                    key: const Key('oturum_dosya_metin'),
                    overflow: TextOverflow.ellipsis,
                    style: const TextStyle(fontSize: 13, fontFeatures: [FontFeature.tabularFigures()])),
              ),
            ]),
            const SizedBox(height: 6),
            LinearProgressIndicator(key: const Key('oturum_dosya_ilerleme'), value: oran),
          ]),
        ),
      ),
    );
  }

  @override
  void dispose() {
    if (widget.motor.dokunmatik) {
      SystemChrome.setPreferredOrientations(DeviceOrientation.values);
      SystemChrome.setEnabledSystemUIMode(SystemUiMode.edgeToEdge);
    }
    _dokunmaSaat?.cancel();
    _yaziOdak.dispose();
    _yazi.dispose();
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

  int get _ms => _saat.elapsedMilliseconds;

  /// Dokunmatik: ekran dışına taşan parmak kenara kırpılır.
  Offset? _normalizeKirp(Offset yerel, Size alan) {
    final k = _kare;
    if (k == null) return null;
    final olcek = (alan.width / k.width).clamp(0.0, alan.height / k.height);
    final g = k.width * olcek, y = k.height * olcek;
    final sol = (alan.width - g) / 2, ust = (alan.height - y) / 2;
    return Offset(((yerel.dx - sol) / g).clamp(0.0, 1.0), ((yerel.dy - ust) / y).clamp(0.0, 1.0));
  }

  void _gonder(List<Girdi> g) {
    if (g.isEmpty) return;
    for (final x in g) {
      widget.motor.girdi(x);
    }
    // İmleç işaretini güncelle.
    if (g.any((x) => x.tur == 'konum') && mounted) setState(() {});
  }

  /// Görünmez yazı alanı hep tek bir gözcü karakter tutar: silinirse Backspace,
  /// eklenen her şey metin olarak gider.
  void _yaziDegisti(String v) {
    if (v.isEmpty) {
      _ozelTus('Backspace');
    } else {
      final yeni = v.replaceAll(_gozcu, '');
      if (yeni.isNotEmpty) _gonder([Girdi('metin', metin: yeni)]);
    }
    _yazi.value = const TextEditingValue(text: _gozcu, selection: TextSelection.collapsed(offset: 1));
  }

  void _ozelTus(String ad) => _gonder([Girdi('tus', ad: ad, basili: true), Girdi('tus', ad: ad, basili: false)]);

  Widget _dokunmatikCubuk() {
    Widget t(String etiket, String ad, {IconData? simge}) => Padding(
          padding: const EdgeInsets.symmetric(horizontal: 3),
          child: OutlinedButton(
            key: Key('ozel_$ad'),
            style: OutlinedButton.styleFrom(minimumSize: const Size(44, 40), padding: const EdgeInsets.symmetric(horizontal: 10)),
            onPressed: () => _ozelTus(ad),
            child: simge != null ? Icon(simge, size: 18) : Text(etiket),
          ),
        );
    return Container(
      color: Renk.yuzey,
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 6),
      child: Row(children: [
        FilledButton.icon(
          key: const Key('oturum_klavye'),
          style: FilledButton.styleFrom(minimumSize: const Size(0, 40)),
          onPressed: () {
            setState(() => _klavyeAcik = !_klavyeAcik);
            if (_klavyeAcik) {
              _yaziOdak.requestFocus();
              SystemChannels.textInput.invokeMethod<void>('TextInput.show');
            } else {
              _yaziOdak.unfocus();
            }
          },
          icon: Icon(_klavyeAcik ? Icons.keyboard_hide : Icons.keyboard),
          label: const Text('Klavye'),
        ),
        const SizedBox(width: 6),
        Expanded(
          child: SingleChildScrollView(
            scrollDirection: Axis.horizontal,
            child: Row(children: [
              t('Esc', 'Escape'),
              t('Tab', 'Tab'),
              t('', 'Enter', simge: Icons.keyboard_return),
              t('', 'Backspace', simge: Icons.backspace_outlined),
              t('', 'ArrowLeft', simge: Icons.arrow_back),
              t('', 'ArrowUp', simge: Icons.arrow_upward),
              t('', 'ArrowDown', simge: Icons.arrow_downward),
              t('', 'ArrowRight', simge: Icons.arrow_forward),
              t('Win', 'Meta'),
            ]),
          ),
        ),
        SizedBox(
          width: 1,
          height: 1,
          child: Opacity(
            opacity: 0,
            child: TextField(
              key: const Key('oturum_yazi'),
              focusNode: _yaziOdak,
              controller: _yazi,
              autocorrect: false,
              enableSuggestions: false,
              onChanged: _yaziDegisti,
              onSubmitted: (_) {
                _ozelTus('Enter');
                _yaziOdak.requestFocus();
              },
            ),
          ),
        ),
      ]),
    );
  }

  Widget _dokunmatikEkran(ui.Image? k) {
    return Column(children: [
      Expanded(
        child: Container(
          color: Colors.black,
          child: LayoutBuilder(builder: (context, c) {
            final alan = Size(c.maxWidth, c.maxHeight);
            final imlec = _imlecKonumu(alan);
            return Listener(
              key: const Key('oturum_ekran'),
              onPointerDown: (e) {
                if (!_kontrol) return;
                final n = _normalizeKirp(e.localPosition, alan);
                if (n != null) _gonder(_dokunma.bas(e.pointer, n.dx, n.dy, _ms));
              },
              onPointerMove: (e) {
                if (!_kontrol) return;
                final n = _normalizeKirp(e.localPosition, alan);
                if (n != null) _gonder(_dokunma.hareket(e.pointer, n.dx, n.dy, _ms));
              },
              onPointerUp: (e) {
                if (_kontrol) _gonder(_dokunma.birak(e.pointer, _ms));
              },
              onPointerCancel: (e) {
                if (_kontrol) _gonder(_dokunma.iptal(e.pointer));
              },
              child: Stack(children: [
                Positioned.fill(
                  child: k == null
                      ? const Center(child: CircularProgressIndicator())
                      : RawImage(key: const Key('oturum_kare'), image: k, fit: BoxFit.contain, filterQuality: FilterQuality.medium),
                ),
                if (imlec != null)
                  Positioned(
                    key: const Key('oturum_imlec'),
                    left: imlec.dx - 9,
                    top: imlec.dy - 9,
                    child: IgnorePointer(
                      child: Container(
                        width: 18,
                        height: 18,
                        decoration: BoxDecoration(
                          shape: BoxShape.circle,
                          border: Border.all(color: Colors.white, width: 2),
                          color: Renk.vurgu.withValues(alpha: 0.5),
                        ),
                      ),
                    ),
                  ),
              ]),
            );
          }),
        ),
      ),
      if (_kontrol) SafeArea(top: false, child: _dokunmatikCubuk()),
    ]);
  }

  Offset? _imlecKonumu(Size alan) {
    final k = _kare, x = _dokunma.imlecX, y = _dokunma.imlecY;
    if (k == null || x == null || y == null) return null;
    final olcek = (alan.width / k.width).clamp(0.0, alan.height / k.height);
    final g = k.width * olcek, yy = k.height * olcek;
    return Offset((alan.width - g) / 2 + x * g, (alan.height - yy) / 2 + y * yy);
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
    // Dar ekranda (telefon dikey) çip ve düğme yalnız simge: başlık sığsın.
    final dar = MediaQuery.sizeOf(context).width < 520;
    final izinMetni = _kontrol ? 'Kontrol açık' : 'Yalnız izleme';
    final izinSimge = Icon(_kontrol ? Icons.mouse : Icons.visibility, size: 16);
    return Scaffold(
      appBar: AppBar(
        toolbarHeight: widget.motor.dokunmatik ? 44 : null,
        title: Row(mainAxisSize: MainAxisSize.min, children: [
          Flexible(child: Text(_karsiAd.isEmpty ? 'Uzak ekran' : _karsiAd, overflow: TextOverflow.ellipsis)),
          if (_asama == _Asama.bagli && _p2) const Padding(padding: EdgeInsets.only(left: 8), child: Chip(key: Key('oturum_p2'), label: Text('🎮 P2'))),
        ]),
        actions: [
          if (_asama == _Asama.bagli && _yeniden != null)
            Padding(
              padding: const EdgeInsets.only(right: 8),
              child: Center(
                child: Text(_yeniden!,
                    key: const Key('oturum_yeniden'), style: const TextStyle(fontSize: 12, color: Renk.tehlike)),
              ),
            )
          else if (_asama == _Asama.bagli && _rtt >= 0)
            Padding(
              padding: const EdgeInsets.only(right: 8),
              child: Tooltip(
                message: _aktarmali
                    ? 'Doğrudan bağlantı kurulamadı; görüntü aktarma sunucusu üzerinden geliyor'
                    : 'Ekran yakalamadan sende görünene kadar geçen süre',
                child: Text('${_gecikme > 0 ? _gecikme : _rtt} ms · $_fps fps${_aktarmali ? ' · aktarmalı' : ''}',
                    key: const Key('oturum_istatistik'),
                    style: TextStyle(
                        fontSize: 12,
                        fontFeatures: const [FontFeature.tabularFigures()],
                        color: _gecikme > 0
                            ? (_gecikme < 100 ? Renk.basari : (_gecikme < 250 ? Renk.soluk : Renk.tehlike))
                            : (_rtt < 80 ? Renk.basari : (_rtt < 200 ? Renk.soluk : Renk.tehlike)))),
              ),
            ),
          if (_asama == _Asama.bagli && !widget.motor.dokunmatik)
            Padding(
              padding: const EdgeInsets.only(right: 8),
              child: Tooltip(
                message: _dosyaIzni
                    ? (_aktarim == null ? 'Karşı tarafa dosya gönder' : 'Bir dosya gönderiliyor')
                    : 'Karşı taraf dosya almaya izin vermedi',
                child: dar
                    ? IconButton(
                        key: const Key('oturum_dosya_gonder'),
                        onPressed: _dosyaIzni && _aktarim == null ? _dosyaGonder : null,
                        icon: const Icon(Icons.upload_file_rounded),
                      )
                    : OutlinedButton.icon(
                        key: const Key('oturum_dosya_gonder'),
                        style: OutlinedButton.styleFrom(minimumSize: const Size(0, 38)),
                        onPressed: _dosyaIzni && _aktarim == null ? _dosyaGonder : null,
                        icon: const Icon(Icons.upload_file_rounded, size: 18),
                        label: const Text('Dosya gönder'),
                      ),
              ),
            ),
          if (_asama == _Asama.bagli)
            Padding(
              padding: const EdgeInsets.only(right: 8),
              child: dar
                  ? Tooltip(message: izinMetni, child: Chip(key: const Key('oturum_kontrol'), label: izinSimge))
                  : Chip(key: const Key('oturum_kontrol'), avatar: izinSimge, label: Text(izinMetni)),
            ),
          if (_asama != _Asama.bitti)
            Padding(
              padding: const EdgeInsets.only(right: 12),
              child: dar
                  ? IconButton.filled(
                      key: const Key('oturum_kes'),
                      tooltip: 'Bağlantıyı kes',
                      style: IconButton.styleFrom(backgroundColor: Renk.tehlike),
                      onPressed: () => Navigator.pop(context),
                      icon: const Icon(Icons.link_off, size: 18),
                    )
                  : FilledButton.icon(
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
      bottomNavigationBar: _asama == _Asama.bagli && _aktarim != null ? _dosyaCubugu(_aktarim!) : null,
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
        if (widget.motor.dokunmatik) return _dokunmatikEkran(k);
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

class _Aktarim {
  final String ad;
  final int gonderilen;
  final int toplam;
  const _Aktarim(this.ad, {this.gonderilen = 0, this.toplam = 0});
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
