import 'dart:async';

import 'package:flutter/material.dart';

import '../tema.dart';
import 'hesap_servisi.dart';

/// Eski kodla bağlantı akışı [kodlaBaglan] üzerinden aynen erişilebilir.
class HesapSayfasi extends StatefulWidget {
  final HesapServisi servis;
  final VoidCallback kodlaBaglan;
  const HesapSayfasi({super.key, required this.servis, required this.kodlaBaglan});
  @override
  State<HesapSayfasi> createState() => _HesapSayfasiDurum();
}

class _HesapSayfasiDurum extends State<HesapSayfasi> {
  List<Cihaz> _cihazlar = [];
  bool _mesgul = false;
  bool _ayarlar = false;
  String? _hata;
  Timer? _tazeleme;
  int _istek = 0;
  HesapServisi get servis => widget.servis;

  @override
  void initState() {
    super.initState();
    servis.addListener(_degisti);
    _yenile();
    _tazeleme = Timer.periodic(const Duration(seconds: 15), (_) => _yenile());
  }

  @override
  void dispose() {
    _tazeleme?.cancel();
    servis.removeListener(_degisti);
    super.dispose();
  }

  void _degisti() {
    if (!mounted) return;
    setState(() {});
    _yenile();
  }

  Future<void> _yenile() async {
    final istek = ++_istek;
    if (servis.hesap == null) {
      if (mounted) setState(() { _cihazlar = []; _ayarlar = false; });
      return;
    }
    try {
      final liste = await servis.bilgisayarlarim();
      if (mounted && istek == _istek) setState(() => _cihazlar = liste);
    } catch (_) {
      if (mounted && istek == _istek) {
        setState(() => _hata = 'Bilgisayarlar alınamadı, yenilemeyi dene.');
      }
    }
  }

  Future<void> _yap(Future<void> Function() islem) async {
    if (_mesgul) return;
    setState(() { _mesgul = true; _hata = null; });
    try {
      await islem();
    } catch (hata) {
      if (mounted) setState(() => _hata = hesapHataMesaji(hata));
    } finally {
      if (mounted) setState(() => _mesgul = false);
    }
  }

  Future<void> _baglan(Cihaz cihaz) async {
    if (cihaz.pinGerekli) {
      final sonuc = await showDialog<bool>(context: context,
        builder: (_) => _PinPenceresi(servis: servis, cihaz: cihaz));
      if (sonuc != true || !mounted) return;
    }
    await servis.baglan(cihaz.id);
  }

  Future<void> _erisim(bool acik) async {
    if (acik && !servis.pinKurulu) {
      final sonuc = await showDialog<bool>(context: context,
        builder: (_) => _PinPenceresi(servis: servis));
      if (sonuc != true || !mounted) return;
    }
    await servis.uzaktanErisimAyarla(acik);
  }

  @override
  Widget build(BuildContext context) {
    final hesap = servis.hesap;
    return Scaffold(
      appBar: AppBar(title: Text(hesap == null ? 'AfuDesk' :
        _ayarlar ? 'Ayarlar' : 'Bilgisayarlarım'), actions: [
        if (hesap != null) IconButton(
          tooltip: _ayarlar ? 'Bilgisayarlarım' : 'Ayarlar',
          onPressed: _mesgul ? null : () => setState(() => _ayarlar = !_ayarlar),
          icon: Icon(_ayarlar ? Icons.computer : Icons.settings_outlined)),
      ]),
      body: SafeArea(child: Center(child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 760),
        child: ListView(padding: const EdgeInsets.all(24), children: [
          if (_hata != null) Padding(padding: const EdgeInsets.only(bottom: 16),
            child: Semantics(liveRegion: true,
              child: Text(_hata!, style: const TextStyle(color: Renk.tehlike)))),
          if (hesap == null) ...[
            const Icon(Icons.devices_rounded, size: 64, color: Renk.vurgu),
            const SizedBox(height: 24),
            const Text('Kendi bilgisayarlarına her yerden ulaş.', textAlign: TextAlign.center),
            const SizedBox(height: 24),
            FilledButton(onPressed: _mesgul ? null : () => _yap(servis.girisYap),
              child: const Text('Google ile giriş yap')),
          ] else if (_ayarlar) ...[
            Text(hesap.ad, style: Theme.of(context).textTheme.titleLarge),
            Text(hesap.eposta),
            SwitchListTile(title: const Text('Onaysız bağlantı'),
              subtitle: const Text('Yalnız bu hesaptaki cihazlar için.'),
              value: servis.onaysizBaglanti,
              onChanged: _mesgul ? null : (acik) => _yap(() => servis.onaysizBaglantiAyarla(acik))),
            const Text('Bu hesaptaki cihazlar'),
            for (final cihaz in _cihazlar) ListTile(title: Text(cihaz.ad),
              trailing: TextButton(onPressed: _mesgul ? null : () => _yap(() => servis.cihazCikar(cihaz.id)),
                child: const Text('Çıkar'))),
            TextButton(onPressed: _mesgul ? null : () => _yap(servis.cikisYap),
              child: const Text('Hesaptan çık')),
          ] else ...[
            Card(child: SwitchListTile(title: const Text('Uzaktan erişime izin ver'),
              subtitle: const Text('Bu cihaz'), value: servis.uzaktanErisim,
              onChanged: _mesgul ? null : (acik) => _yap(() => _erisim(acik)))),
            const SizedBox(height: 16),
            if (_cihazlar.isEmpty) const Text('Diğer bilgisayarında da aynı Google hesabıyla giriş yap.'),
            for (final cihaz in _cihazlar) Card(child: ListTile(
              leading: Icon(Icons.circle, size: 12,
                color: cihaz.cevrimici ? Renk.basari : Renk.soluk),
              title: Text(cihaz.ad), subtitle: Text(cihaz.cevrimici ? 'Çevrimiçi' : 'Çevrimdışı'),
              enabled: cihaz.cevrimici && !_mesgul,
              onTap: cihaz.cevrimici && !_mesgul ? () => _yap(() => _baglan(cihaz)) : null)),
            TextButton(onPressed: _mesgul ? null : () => _yap(_yenile), child: const Text('Yenile')),
          ],
          if (_mesgul) const LinearProgressIndicator(),
          const SizedBox(height: 24),
          TextButton(onPressed: _mesgul ? null : widget.kodlaBaglan,
            child: const Text('Kodla bağlantı')),
        ]),
      ))),
    );
  }
}

String hesapHataMesaji(Object hata) => hata is HesapHatasi ? hata.mesaj :
  'İşlem tamamlanamadı, yeniden dene.';

class _PinPenceresi extends StatefulWidget {
  final HesapServisi servis;
  final Cihaz? cihaz;
  const _PinPenceresi({required this.servis, this.cihaz});
  @override
  State<_PinPenceresi> createState() => _PinPenceresiDurum();
}

class _PinPenceresiDurum extends State<_PinPenceresi> {
  final _pin = TextEditingController();
  final _tekrar = TextEditingController();
  bool _hatirla = false;
  bool _mesgul = false;
  String? _hata;
  bool get kurulum => widget.cihaz == null;
  @override
  void dispose() {
    _pin.dispose();
    _tekrar.dispose();
    super.dispose();
  }

  Future<void> _kaydet() async {
    if (_mesgul) return;
    setState(() { _mesgul = true; _hata = null; });
    try {
      if (kurulum) {
        if (!RegExp(r'^[0-9]{6,}$').hasMatch(_pin.text)) {
          throw const HesapHatasi('PIN için en az 6 rakam gir.');
        }
        if (_pin.text != _tekrar.text) {
          throw const HesapHatasi('PIN eşleşmiyor, iki alana aynı PIN’i gir.');
        }
        await widget.servis.pinKur(_pin.text);
      } else if (!await widget.servis.pinDogrula(widget.cihaz!.id, _pin.text, _hatirla)) {
        throw const HesapHatasi('PIN yanlış, yeniden dene.');
      }
      if (mounted) Navigator.pop(context, true);
    } catch (hata) {
      if (mounted) setState(() => _hata = hesapHataMesaji(hata));
    } finally {
      if (mounted) setState(() => _mesgul = false);
    }
  }

  @override
  Widget build(BuildContext context) => PopScope(
    canPop: !_mesgul,
    child: AlertDialog(
      title: Text(kurulum ? 'PIN belirle' : '${widget.cihaz!.ad} için PIN'),
      content: SingleChildScrollView(child: Column(mainAxisSize: MainAxisSize.min, children: [
        if (kurulum) const Text('Bu cihazı korumak için en az 6 rakam belirle.'),
        TextField(key: const Key('hesap_pin'), controller: _pin, autofocus: true,
          enabled: !_mesgul, obscureText: true, keyboardType: TextInputType.number,
          enableSuggestions: false, autocorrect: false,
          decoration: const InputDecoration(labelText: 'PIN')),
        if (kurulum) ...[
          const SizedBox(height: 12),
          TextField(key: const Key('hesap_pin_tekrar'), controller: _tekrar,
            enabled: !_mesgul, obscureText: true, keyboardType: TextInputType.number,
            enableSuggestions: false, autocorrect: false,
            decoration: const InputDecoration(labelText: 'PIN tekrar')),
        ] else CheckboxListTile(contentPadding: EdgeInsets.zero,
          title: const Text('Bu cihazda hatırla'), value: _hatirla,
          onChanged: _mesgul ? null : (deger) => setState(() => _hatirla = deger ?? false)),
        if (_hata != null) Text(_hata!, style: const TextStyle(color: Renk.tehlike)),
      ])),
      actions: [
        TextButton(onPressed: _mesgul ? null : () => Navigator.pop(context), child: const Text('İptal')),
        FilledButton(onPressed: _mesgul ? null : _kaydet, child: Text(kurulum ? 'Kaydet' : 'Bağlan')),
      ],
    ),
  );
}

/// Navigator'ın üstünde kalır; başka sayfaya geçiş göstergeleri gizlemez.
class HesapBaglantiSeridi extends StatelessWidget {
  final HesapServisi servis;
  final Widget child;
  const HesapBaglantiSeridi({super.key, required this.servis, required this.child});
  @override
  Widget build(BuildContext context) => StreamBuilder<List<Cihaz>>(
    stream: servis.aktifBaglantilar,
    initialData: servis.baglantilar,
    builder: (context, snapshot) => Column(children: [
      if ((snapshot.data ?? []).isNotEmpty) Material(
        color: Renk.yuzey2,
        child: SafeArea(bottom: false, child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 6),
          child: Row(children: [
            Expanded(child: Text('Uzaktan bağlı: ${snapshot.data!.map((c) => c.ad).join(', ')}')),
            _KesDugmesi(servis: servis),
          ]),
        )),
      ),
      Expanded(child: child),
    ]),
  );
}

class _KesDugmesi extends StatefulWidget {
  final HesapServisi servis;
  const _KesDugmesi({required this.servis});
  @override
  State<_KesDugmesi> createState() => _KesDugmesiDurum();
}

class _KesDugmesiDurum extends State<_KesDugmesi> {
  bool _mesgul = false;
  String? _hata;
  @override
  Widget build(BuildContext context) => Column(mainAxisSize: MainAxisSize.min, children: [
    TextButton(onPressed: _mesgul ? null : () async {
      setState(() { _mesgul = true; _hata = null; });
      try {
        await widget.servis.baglantiyiKes();
      } catch (_) {
        if (mounted) setState(() => _hata = 'Bağlantı kesilemedi, yeniden dene.');
      } finally {
        if (mounted) setState(() => _mesgul = false);
      }
    }, child: const Text('Kes')),
    if (_hata != null) Text(_hata!),
  ]);
}
