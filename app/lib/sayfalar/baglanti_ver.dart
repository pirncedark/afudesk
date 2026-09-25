import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../motor.dart';
import '../tema.dart';

/// Kodun geçerlilik süresi (Rust tarafıyla aynı: 10 dk).
const kodSuresi = Duration(minutes: 10);

class BaglantiVerSayfasi extends StatefulWidget {
  final Motor motor;
  const BaglantiVerSayfasi({super.key, required this.motor});

  @override
  State<BaglantiVerSayfasi> createState() => _BaglantiVerDurum();
}

enum _Asama { hazirlaniyor, bekliyor, bagli, hata }

class _BaglantiVerDurum extends State<BaglantiVerSayfasi> {
  StreamSubscription<HostOlay>? _abonelik;
  _Asama _asama = _Asama.hazirlaniyor;
  String _kod = '';
  String _parola = '';
  String _erisim = '';
  String _hata = '';
  String _bagliAd = '';
  bool _bagliKontrol = false;
  bool _bagliPano = false;
  bool _bagliDosya = false;
  final _alinanlar = <(String, String)>[];
  bool _bagliOyunKolu = false;
  bool _kolSurucusuYok = false;

  DateTime _bitis = DateTime.now();
  Timer? _sayac;
  bool _istekAcik = false;

  @override
  void initState() {
    super.initState();
    _baslat();
    _sayac = Timer.periodic(const Duration(seconds: 1), (_) {
      if (mounted && _asama == _Asama.bekliyor) setState(() {});
    });
  }

  void _baslat() {
    _abonelik?.cancel();
    setState(() {
      _asama = _Asama.hazirlaniyor;
      _hata = '';
    });
    _abonelik = widget.motor.hostBaslat(ad: widget.motor.cihazAdi()).listen(_olay, onError: (Object e) {
      if (!mounted) return;
      setState(() {
        _asama = _Asama.hata;
        _hata = hataMetni(e);
      });
    });
  }

  void _olay(HostOlay o) {
    if (!mounted) return;
    switch (o.tur) {
      case 'hazir':
        setState(() {
          _asama = _Asama.bekliyor;
          _kod = o.kod;
          _parola = o.parola;
          _erisim = o.erisim;
          _bitis = DateTime.now().add(kodSuresi);
        });
      case 'istek':
        _istekGoster(o.ad);
      case 'baglandi':
        _kolSurucusuYok = false;
        setState(() {
          _asama = _Asama.bagli;
          _bagliAd = o.ad;
          _bagliKontrol = o.kontrol;
          _bagliPano = o.pano;
          _bagliDosya = o.dosya;
          _alinanlar.clear();
          _bagliOyunKolu = o.oyunKolu;

        });
      case 'dosya':
        setState(() => _alinanlar.insert(0, (o.ad, o.yol)));
        _mesaj('Dosya alındı: ${o.ad}');
      case 'koptu':
        _mesaj(o.metin);
        // Host yeni kod üretecek ('hazir' gelecek).
        setState(() => _asama = _Asama.hazirlaniyor);
      case 'hata':
        setState(() {
          _asama = _Asama.hata;
          _hata = o.metin;
        });
      case 'uyari':
        _kolSurucusuYok = true;
        _mesaj(o.metin);
    }
  }

  Future<void> _istekGoster(String ad) async {
    if (_istekAcik) return;
    _istekAcik = true;
    final sonuc = await showDialog<IstekKarari?>(

      context: context,
      barrierDismissible: false,
      builder: (_) => IstekPenceresi(ad: ad),
    );
    _istekAcik = false;
    if (sonuc == null) {
      await widget.motor.hostRed();
    } else {
await widget.motor.hostKabul(kontrol: sonuc.kontrol, pano: sonuc.pano, dosya: sonuc.dosya, oyunKolu: sonuc.oyunKolu);
    }
  }

  void _mesaj(String m) {
    if (m.isEmpty) return;
    // Yeni bildirim eskisinin yerini alsın (üst üste sıraya girmesin).
    ScaffoldMessenger.of(context)
      ..hideCurrentSnackBar()
      ..showSnackBar(SnackBar(content: Text(m), duration: const Duration(seconds: 2)));
  }

  void _kopyala(String metin, String ne) {
    Clipboard.setData(ClipboardData(text: metin));
    _mesaj('$ne kopyalandı');
  }

  @override
  void dispose() {
    _sayac?.cancel();
    _abonelik?.cancel();
    widget.motor.hostDurdur();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('Bağlantı ver')),
      body: Center(
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 640),
          child: ListView(padding: const EdgeInsets.all(24), children: [_icerik()]),
        ),
      ),
    );
  }

  Widget _icerik() {
    switch (_asama) {
      case _Asama.hazirlaniyor:
        return const Padding(
          padding: EdgeInsets.only(top: 60),
          child: Column(children: [
            CircularProgressIndicator(),
            SizedBox(height: 16),
            Text('Bağlantı hazırlanıyor…', style: TextStyle(color: Renk.soluk)),
          ]),
        );
      case _Asama.hata:
        return Column(children: [
          const Icon(Icons.error_outline, color: Renk.tehlike, size: 40),
          const SizedBox(height: 12),
          Text(_hata, key: const Key('ver_hata'), textAlign: TextAlign.center),
          const SizedBox(height: 16),
          FilledButton(onPressed: _baslat, child: const Text('Tekrar dene')),
        ]);
      case _Asama.bagli:
        return Card(
          child: Padding(
            padding: const EdgeInsets.all(22),
            child: Column(children: [
              const Icon(Icons.visibility_rounded, color: Renk.basari, size: 40),
              const SizedBox(height: 12),
              Text('$_bagliAd ekranını görüyor',
                  key: const Key('ver_bagli'), style: const TextStyle(fontSize: 18, fontWeight: FontWeight.w700)),
              const SizedBox(height: 6),
              Text(_bagliKontrol ? 'Fare ve klavyeyi kullanabiliyor.' : 'Yalnız izliyor; kontrol edemez.',
                  style: const TextStyle(color: Renk.soluk)),
              if (_bagliPano)
                const Padding(
                  padding: EdgeInsets.only(top: 4),
                  child: Text('Pano paylaşılıyor.', key: Key('ver_pano'), style: TextStyle(color: Renk.soluk)),
                ),
const SizedBox(height: 4),
              Text(_bagliDosya ? 'Sana dosya gönderebilir (İndirilenler\\AfuDesk).' : 'Dosya gönderemez.',
                  key: const Key('ver_dosya_izni'), style: const TextStyle(color: Renk.soluk)),
              if (_alinanlar.isNotEmpty) ...[
                const SizedBox(height: 14),
                Container(
                  key: const Key('ver_alinanlar'),
                  width: double.infinity,
                  padding: const EdgeInsets.all(10),
                  decoration: BoxDecoration(color: Renk.yuzey2, borderRadius: BorderRadius.circular(8)),
                  child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
                    const Text('Alınan dosyalar', style: TextStyle(fontWeight: FontWeight.w700)),
                    for (final (ad, yol) in _alinanlar)
                      Padding(
                        padding: const EdgeInsets.only(top: 6),
                        child: Row(children: [
                          const Icon(Icons.download_done_rounded, size: 18, color: Renk.basari),
                          const SizedBox(width: 6),
                          Expanded(
                            child: Tooltip(
                              message: yol,
                              child: Text(ad, overflow: TextOverflow.ellipsis),
                            ),
                          ),
                        ]),
                      ),
                  ]),
                ),
              ],
Text('Oyun kolu: ${_kolSurucusuYok ? 'sürücü yok' : (_bagliOyunKolu ? 'açık' : 'kapalı')}', key: const Key('ver_oyun_kolu')),

              const SizedBox(height: 18),
              FilledButton.icon(
                key: const Key('ver_kes'),
                style: FilledButton.styleFrom(backgroundColor: Renk.tehlike),
                onPressed: () => widget.motor.hostKes(),
                icon: const Icon(Icons.link_off),
                label: const Text('Bağlantıyı kes'),
              ),
            ]),
          ),
        );
      case _Asama.bekliyor:
        final kalan = _bitis.difference(DateTime.now());
        final dk = kalan.inMinutes.clamp(0, 99);
        final sn = (kalan.inSeconds % 60).clamp(0, 59).toString().padLeft(2, '0');
        return Column(crossAxisAlignment: CrossAxisAlignment.stretch, children: [
          const Text('Bu iki bilgiyi bağlanacak kişiye gönder:', style: TextStyle(color: Renk.soluk)),
          const SizedBox(height: 14),
          Card(
            child: Padding(
              padding: const EdgeInsets.all(16),
              child: Column(crossAxisAlignment: CrossAxisAlignment.stretch, children: [
                const Text('1. Bağlantı kodu', style: TextStyle(fontWeight: FontWeight.w700)),
                const SizedBox(height: 8),
                Container(
                  padding: const EdgeInsets.all(12),
                  decoration: BoxDecoration(color: Renk.yuzey2, borderRadius: BorderRadius.circular(8)),
                  child: SelectableText(_kod,
                      key: const Key('ver_kod'),
                      maxLines: 4,
                      style: const TextStyle(fontFamily: 'Consolas', fontSize: 12, color: Renk.soluk)),
                ),
                const SizedBox(height: 10),
                FilledButton.icon(
                  key: const Key('ver_kod_kopyala'),
                  onPressed: () => _kopyala(_kod, 'Kod'),
                  icon: const Icon(Icons.copy_rounded),
                  label: const Text('Kodu kopyala'),
                ),
              ]),
            ),
          ),
          const SizedBox(height: 12),
          Card(
            child: Padding(
              padding: const EdgeInsets.all(16),
              child: Row(children: [
                Expanded(
                  child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
                    const Text('2. Parola', style: TextStyle(fontWeight: FontWeight.w700)),
                    const SizedBox(height: 4),
                    SelectableText(_parola,
                        key: const Key('ver_parola'),
                        style: const TextStyle(fontSize: 30, fontWeight: FontWeight.w700, letterSpacing: 6)),
                  ]),
                ),
                OutlinedButton.icon(
                  key: const Key('ver_parola_kopyala'),
                  onPressed: () => _kopyala(_parola, 'Parola'),
                  icon: const Icon(Icons.copy_rounded, size: 18),
                  label: const Text('Kopyala'),
                ),
              ]),
            ),
          ),
          const SizedBox(height: 14),
          Row(children: [
            const Icon(Icons.timer_outlined, size: 18, color: Renk.soluk),
            const SizedBox(width: 6),
            Text('Kod $dk:$sn içinde yenilenecek', key: const Key('ver_sure'), style: const TextStyle(color: Renk.soluk)),
          ]),
          const SizedBox(height: 6),
          Row(children: [
            Icon(_erisim.startsWith('İnternet') ? Icons.public : Icons.lan_outlined, size: 18, color: Renk.soluk),
            const SizedBox(width: 6),
            Expanded(child: Text(_erisim, key: const Key('ver_erisim'), style: const TextStyle(color: Renk.soluk))),
          ]),
          if (_erisim.contains('Yalnız aynı ağdan'))
            const Padding(
              padding: EdgeInsets.only(left: 26, top: 6),
              child: Text('Farklı bir evden bağlanacaklar için modemde UPnP’yi açın ya da bağlantıyı karşı taraf versin.', key: Key('ver_ag_onerisi'), style: TextStyle(color: Renk.soluk)),
            ),
          const SizedBox(height: 18),
          const Text('Biri bağlanmak istediğinde sana sorulacak. Onay vermeden kimse ekranını göremez.',
              style: TextStyle(color: Renk.soluk, fontSize: 13)),
        ]);
    }
  }
}

class IstekKarari {
  final bool kontrol;
  final bool pano;
  final bool dosya;
  final bool oyunKolu;
  const IstekKarari({required this.kontrol, required this.pano, required this.dosya, required this.oyunKolu});
}

/// Gelen bağlantı isteği. Sonuç: null = reddet, [IstekKarari] = kabul (+izinler).

class IstekPenceresi extends StatefulWidget {
  final String ad;
  final Duration sure;
  const IstekPenceresi({super.key, required this.ad, this.sure = const Duration(seconds: 60)});

  @override
  State<IstekPenceresi> createState() => _IstekPenceresiDurum();
}

class _IstekPenceresiDurum extends State<IstekPenceresi> {
  bool _kontrol = true;
  bool _pano = false;
  bool _dosya = false;
  bool _oyunKolu = true;
  late int _kalan = widget.sure.inSeconds;
  Timer? _t;

  @override
  void initState() {
    super.initState();
    _t = Timer.periodic(const Duration(seconds: 1), (_) {
      if (!mounted) return;
      setState(() => _kalan--);
      if (_kalan <= 0) {
        _t?.cancel();
        Navigator.of(context).pop(null);
      }
    });
  }

  @override
  void dispose() {
    _t?.cancel();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: Text('${widget.ad} bağlanmak istiyor',
          key: const Key('istek_baslik'), style: const TextStyle(fontSize: 20, fontWeight: FontWeight.w700)),
      content: Column(mainAxisSize: MainAxisSize.min, crossAxisAlignment: CrossAxisAlignment.start, children: [
        const Text('Kabul edersen ekranını görebilecek.'),
        const SizedBox(height: 10),
        CheckboxListTile(
          key: const Key('istek_kontrol'),
          contentPadding: EdgeInsets.zero,
          value: _kontrol,
          onChanged: (v) => setState(() => _kontrol = v ?? false),
          title: const Text('Fare ve klavye kontrolüne izin ver'),
        ),
        CheckboxListTile(
          key: const Key('istek_pano'),
          contentPadding: EdgeInsets.zero,
          value: _pano,
          onChanged: (v) => setState(() => _pano = v ?? false),
          title: const Text('Pano paylaşımı'),
          subtitle: const Text('Kopyalanan metinler iki yönde paylaşılır.', style: TextStyle(fontSize: 12)),
        ),
        CheckboxListTile(
          key: const Key('istek_dosya'),
          contentPadding: EdgeInsets.zero,
          value: _dosya,
          onChanged: (v) => setState(() => _dosya = v ?? false),
          title: const Text('Dosya almaya izin ver'),
          subtitle: const Text('Gelen dosyalar İndirilenler\\AfuDesk klasörüne kaydedilir.',
              style: TextStyle(color: Renk.soluk, fontSize: 12)),
        ),
        CheckboxListTile(
          key: const Key('istek_oyun_kolu'),
          contentPadding: EdgeInsets.zero,
          value: _oyunKolu,
          onChanged: (v) => setState(() => _oyunKolu = v ?? false),
          title: const Text('Oyun kolu'),

        ),
        Text('$_kalan sn içinde yanıt vermezsen reddedilir.', style: const TextStyle(color: Renk.soluk, fontSize: 12)),
      ]),
      actions: [
        TextButton(key: const Key('istek_red'), onPressed: () => Navigator.pop(context, null), child: const Text('Reddet')),
        FilledButton(
          key: const Key('istek_kabul'),
          onPressed: () => Navigator.pop(context, IstekKarari(kontrol: _kontrol, pano: _pano, dosya: _dosya, oyunKolu: _oyunKolu)),
          child: const Text('Kabul et'),
        ),
      ],
    );
  }
}
