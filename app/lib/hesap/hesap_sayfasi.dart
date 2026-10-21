import 'dart:async';

import 'package:flutter/material.dart';

import '../tema.dart';
import '../bilesenler/guncelle_karti.dart';
import 'hesap_servisi.dart';
import 'google_hatasi.dart';
import 'gercek_hesap_servisi.dart';

/// Eski kodla bağlantı akışı [kodlaBaglan] üzerinden aynen erişilebilir.
class HesapSayfasi extends StatefulWidget {
  final HesapServisi servis;
  final VoidCallback kodlaBaglan;
  const HesapSayfasi({
    super.key,
    required this.servis,
    required this.kodlaBaglan,
  });
  @override
  State<HesapSayfasi> createState() => _HesapSayfasiDurum();
}

class _HesapSayfasiDurum extends State<HesapSayfasi> {
  List<Cihaz> _cihazlar = [];
  bool _mesgul = false;
  bool _ayarlar = false;
  String? _hata;
  String? _ayrinti;
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
      if (mounted) {
        setState(() {
          _cihazlar = [];
          _ayarlar = false;
        });
      }
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
    setState(() {
      _mesgul = true;
      _hata = null;
      _ayrinti = null;
    });
    try {
      await islem();
    } catch (hata) {
      if (mounted) {
        setState(() {
          _hata = hesapHataMesaji(hata);
          _ayrinti = hata is GoogleHatasi ? hata.ayrinti : null;
        });
      }
    } finally {
      if (mounted) setState(() => _mesgul = false);
    }
  }

  Future<void> _baglan(Cihaz cihaz) => servis.baglan(cihaz.id);
  Future<void> _erisim(bool acik) => servis.uzaktanErisimAyarla(acik);

  @override
  Widget build(BuildContext context) {
    final hesap = servis.hesap;
    return Scaffold(
      appBar: AppBar(
        title: Text(
          hesap == null
              ? 'AfuDesk'
              : _ayarlar
              ? 'Ayarlar'
              : 'Bilgisayarlarım',
        ),
        actions: [
          if (hesap != null)
            IconButton(
              tooltip: _ayarlar ? 'Bilgisayarlarım' : 'Hesap cihazları',
              onPressed: _mesgul
                  ? null
                  : () => setState(() => _ayarlar = !_ayarlar),
              icon: Icon(_ayarlar ? Icons.computer : Icons.manage_accounts_outlined),
            ),
          const GuncelleKarti(ayarDugmesi: true),
        ],
      ),
      body: SafeArea(
        child: Center(
          child: ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 760),
            child: ListView(
              padding: const EdgeInsets.all(24),
              children: [
                if (servis is GercekHesapServisi && (servis as GercekHesapServisi).durum != null)
                  Semantics(liveRegion: true,
                    child: Text((servis as GercekHesapServisi).durum!)),
                if (_hata != null)
                  Padding(
                    padding: const EdgeInsets.only(bottom: 16),
                    child: Semantics(
                      liveRegion: true,
                      child: Text(
                        _hata!,
                        style: const TextStyle(color: Renk.tehlike),
                      ),
                    ),
                  ),
                if (_ayrinti != null)
                  ExpansionTile(
                    title: const Text('Ayrıntı'),
                    children: [SelectableText(_ayrinti!)],
                  ),
                if (hesap == null) ...[
                  const Icon(
                    Icons.devices_rounded,
                    size: 64,
                    color: Renk.vurgu,
                  ),
                  const SizedBox(height: 24),
                  const Text(
                    'Kendi bilgisayarlarına her yerden ulaş.',
                    textAlign: TextAlign.center,
                  ),
                  const SizedBox(height: 24),
                  FilledButton(
                    onPressed: _mesgul ? null : () => _yap(servis.girisYap),
                    child: const Text('Google ile giriş yap'),
                  ),
                ] else if (_ayarlar) ...[
                  Text(hesap.ad, style: Theme.of(context).textTheme.titleLarge),
                  Text(hesap.eposta),
                  const Text('Bu hesaptaki cihazlar'),
                  for (final cihaz in _cihazlar)
                    ListTile(
                      title: Text(cihaz.ad),
                      trailing: TextButton(
                        onPressed: _mesgul
                            ? null
                            : () => _yap(() => servis.cihazCikar(cihaz.id)),
                        child: const Text('Çıkar'),
                      ),
                    ),
                  TextButton(
                    onPressed: _mesgul ? null : () => _yap(servis.cikisYap),
                    child: const Text('Hesaptan çık'),
                  ),
                ] else ...[
                  Card(
                    child: SwitchListTile(
                      title: const Text('Uzaktan erişime izin ver'),
                      subtitle: const Text('Bu cihaz'),
                      value: servis.uzaktanErisim,
                      onChanged: _mesgul
                          ? null
                          : (acik) => _yap(() => _erisim(acik)),
                    ),
                  ),
                  const SizedBox(height: 16),
                  if (_cihazlar.isEmpty)
                    const Text(
                      'Diğer bilgisayarında da aynı Google hesabıyla giriş yap.',
                    ),
                  for (final cihaz in _cihazlar)
                    Card(
                      child: ListTile(
                        leading: Icon(
                          Icons.circle,
                          size: 12,
                          color: cihaz.cevrimici ? Renk.basari : Renk.soluk,
                        ),
                        title: Text(cihaz.ad),
                        subtitle: Text(
                          cihaz.cevrimici ? 'Çevrimiçi' : 'Çevrimdışı',
                        ),
                        enabled: cihaz.cevrimici && !_mesgul,
                        onTap: cihaz.cevrimici && !_mesgul
                            ? () => _yap(() => _baglan(cihaz))
                            : null,
                      ),
                    ),
                  TextButton(
                    onPressed: _mesgul ? null : () => _yap(_yenile),
                    child: const Text('Yenile'),
                  ),
                ],
                if (_mesgul) const LinearProgressIndicator(),
                const SizedBox(height: 24),
                TextButton(
                  onPressed: _mesgul ? null : widget.kodlaBaglan,
                  child: const Text('Kodla bağlantı'),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

String hesapHataMesaji(Object hata) =>
    hata is HesapHatasi ? hata.mesaj : GoogleHatasi.cevir(hata).mesaj;

/// Navigator'ın üstünde kalır; başka sayfaya geçiş göstergeleri gizlemez.
class HesapBaglantiSeridi extends StatelessWidget {
  final HesapServisi servis;
  final Widget child;
  const HesapBaglantiSeridi({
    super.key,
    required this.servis,
    required this.child,
  });
  @override
  Widget build(BuildContext context) => StreamBuilder<List<Cihaz>>(
    stream: servis.aktifBaglantilar,
    initialData: servis.baglantilar,
    builder: (context, snapshot) => Column(
      children: [
        if ((snapshot.data ?? []).isNotEmpty)
          Material(
            color: Renk.yuzey2,
            child: SafeArea(
              bottom: false,
              child: Padding(
                padding: const EdgeInsets.symmetric(
                  horizontal: 16,
                  vertical: 6,
                ),
                child: Row(
                  children: [
                    Expanded(
                      child: Text(
                        'Uzaktan bağlı: ${snapshot.data!.map((c) => c.ad).join(', ')}',
                      ),
                    ),
                    _KesDugmesi(servis: servis),
                  ],
                ),
              ),
            ),
          ),
        Expanded(child: child),
      ],
    ),
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
  Widget build(BuildContext context) => Column(
    mainAxisSize: MainAxisSize.min,
    children: [
      TextButton(
        onPressed: _mesgul
            ? null
            : () async {
                setState(() {
                  _mesgul = true;
                  _hata = null;
                });
                try {
                  await widget.servis.baglantiyiKes();
                } catch (_) {
                  if (mounted) {
                    setState(
                      () => _hata = 'Bağlantı kesilemedi, yeniden dene.',
                    );
                  }
                } finally {
                  if (mounted) setState(() => _mesgul = false);
                }
              },
        child: const Text('Kes'),
      ),
      if (_hata != null) Text(_hata!),
    ],
  );
}
