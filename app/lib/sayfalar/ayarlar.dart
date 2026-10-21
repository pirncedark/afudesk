import 'package:flutter/material.dart';

import '../guncelleme.dart';
import '../hesap/hesap_servisi.dart';

class AyarlarSayfasi extends StatefulWidget {
  const AyarlarSayfasi({super.key, this.servis, this.hesapServisi});
  final Guncelleyici? servis;
  final HesapServisi? hesapServisi;

  @override
  State<AyarlarSayfasi> createState() => _AyarlarSayfasiDurum();
}

class _AyarlarSayfasiDurum extends State<AyarlarSayfasi> {
  late final Guncelleyici _servis;
  String _surum = 'Okunuyor…';
  String? _hata, _bilgi, _hesapHatasi;
  bool _mesgul = false, _cikiliyor = false;
  double? _ilerleme;

  @override
  void initState() {
    super.initState();
    _servis = widget.servis ?? guncelleyici;
    _servis.addListener(_degisti);
    widget.hesapServisi?.addListener(_degisti);
    _surumuOku();
  }

  @override
  void dispose() {
    _servis.removeListener(_degisti);
    widget.hesapServisi?.removeListener(_degisti);
    super.dispose();
  }

  void _degisti() {
    if (mounted) setState(() {});
  }

  Future<void> _surumuOku() async {
    try {
      final surum = await _servis.surum();
      if (mounted) setState(() => _surum = surum);
    } catch (_) {
      if (mounted) {
        setState(() => _surum = 'Sürüm alınamadı, Ayarlar’ı yeniden aç.');
      }
    }
  }

  Future<void> _denetle() async {
    if (_mesgul) return;
    setState(() {
      _mesgul = true;
      _hata = null;
      _bilgi = null;
    });
    try {
      await _servis.denetle();
    } catch (_) {
      if (mounted) {
        setState(() => _hata = 'Güncellemeler denetlenemedi, yeniden dene.');
      }
    } finally {
      if (mounted) setState(() => _mesgul = false);
    }
  }

  Future<void> _kur() async {
    final yayin = _servis.yayin;
    if (_mesgul || yayin == null) return;
    setState(() {
      _mesgul = true;
      _hata = null;
      _bilgi = null;
      _ilerleme = 0;
    });
    try {
      await _servis.kur(yayin, (p) {
        if (mounted) setState(() => _ilerleme = p);
      });
      if (mounted) {
        setState(() => _bilgi = 'Kurulum açıldı, tamamlamak için onayla.');
      }
    } on GuncellemeSayfasiAcildi {
      if (mounted) {
        setState(
          () => _bilgi = 'Güncelleme sayfası açıldı, yeni sürümü oradan indir.',
        );
      }
    } catch (_) {
      if (mounted) {
        setState(() => _hata = 'Güncelleme tamamlanamadı, yeniden dene.');
      }
    } finally {
      if (mounted) {
        setState(() {
          _mesgul = false;
          _ilerleme = null;
        });
      }
    }
  }

  Future<void> _cikis() async {
    if (_cikiliyor) return;
    setState(() {
      _cikiliyor = true;
      _hesapHatasi = null;
    });
    try {
      await widget.hesapServisi!.cikisYap();
    } catch (_) {
      if (mounted) {
        setState(() => _hesapHatasi = 'Çıkış yapılamadı, yeniden dene.');
      }
    } finally {
      if (mounted) setState(() => _cikiliyor = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final yayin = _servis.yayin;
    final denetim = _servis.sonDenetim;
    final hesap = widget.hesapServisi?.hesap;
    final durum =
        _hata ??
        _bilgi ??
        (_mesgul
            ? (_ilerleme == null
                  ? 'Denetleniyor…'
                  : _ilerleme! >= 1
                  ? 'Kurulum hazırlanıyor…'
                  : 'İndiriliyor… %${(_ilerleme! * 100).round()}')
            : yayin != null
            ? 'Yeni sürüm var: ${yayin.surum}'
            : denetim != null
            ? 'Güncelsin'
            : null);
    return PopScope(
      canPop: !_mesgul && !_cikiliyor,
      child: Scaffold(
        appBar: AppBar(title: const Text('Ayarlar')),
        body: SafeArea(
          child: Center(
            child: ConstrainedBox(
              constraints: const BoxConstraints(maxWidth: 600),
              child: ListView(
                padding: const EdgeInsets.all(24),
                children: [
                  ListTile(title: const Text('Sürüm'), subtitle: Text(_surum)),
                  const Divider(),
                  ListTile(
                    title: const Text('Güncelleme'),
                    subtitle: durum == null
                        ? null
                        : Semantics(liveRegion: true, child: Text(durum)),
                  ),
                  if (_mesgul) LinearProgressIndicator(value: _ilerleme),
                  FilledButton.icon(
                    onPressed: _mesgul || _cikiliyor
                        ? null
                        : yayin == null
                        ? _denetle
                        : _kur,
                    icon: Icon(
                      yayin == null ? Icons.refresh : Icons.system_update,
                    ),
                    label: Text(
                      yayin == null ? 'Güncellemeleri denetle' : 'Güncelle',
                    ),
                  ),
                  if (yayin != null)
                    TextButton(
                      onPressed: _mesgul || _cikiliyor ? null : _denetle,
                      child: const Text('Yeniden denetle'),
                    ),
                  if (denetim != null)
                    ListTile(
                      title: const Text('Son denetim'),
                      subtitle: Text(
                        '${denetim.day}.${denetim.month}.${denetim.year} '
                        '${denetim.hour.toString().padLeft(2, '0')}:'
                        '${denetim.minute.toString().padLeft(2, '0')}',
                      ),
                    ),
                  if (hesap != null) ...[
                    const Divider(),
                    ListTile(
                      title: const Text('Hesap'),
                      subtitle: Text(hesap.eposta),
                    ),
                    TextButton(
                      onPressed: _mesgul || _cikiliyor ? null : _cikis,
                      child: const Text('Çıkış yap'),
                    ),
                  ],
                  if (_hesapHatasi != null) Text(_hesapHatasi!),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}
