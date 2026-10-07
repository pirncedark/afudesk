import 'dart:async';
import 'dart:io';
import 'package:flutter/material.dart';
import '../guncelleme.dart';

class GuncelleKarti extends StatefulWidget {
  const GuncelleKarti({super.key, this.servis});
  final Guncelleyici? servis;
  static final Set<String> ertelenen = {};
  @override
  State<GuncelleKarti> createState() => _GuncelleKartiDurum();
}

class _GuncelleKartiDurum extends State<GuncelleKarti>
    with WidgetsBindingObserver {
  late final Guncelleyici _servis;
  bool _gizli = false;
  Guncelleme? _yayin;
  Timer? _timer;
  bool _mesgul = false, _kontrol = false;
  double _ilerleme = 0;
  String? _hata;
  @override
  void initState() {
    super.initState();
    _servis = widget.servis ?? Guncelleyici();
    guncellemeKontrolIstegi.addListener(_bak);
    WidgetsBinding.instance.addObserver(this);
    _bak();
    _timer = Timer.periodic(const Duration(hours: 6), (_) => _bak());
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    _timer?.cancel();
    guncellemeKontrolIstegi.removeListener(_bak);
    super.dispose();
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    if (state == AppLifecycleState.resumed) _bak();
  }

  Future<void> _bak() async {
    if (_mesgul || _kontrol) return;
    _kontrol = true;
    try {
      final yayin = await _servis.kontrol();
      if (mounted) {
        setState(() {
          _yayin = yayin;
          _gizli = yayin != null && GuncelleKarti.ertelenen.contains(yayin.surum);
        });
      }
    } catch (_) {
      // Ağ yoksa ana kullanım etkilenmez; sonraki kontrolde yeniden denenir.
    } finally {
      _kontrol = false;
    }
  }

  Future<void> _kur() async {
    if (_mesgul || _yayin == null) return;
    setState(() {
      _mesgul = true;
      _hata = null;
      _ilerleme = 0;
    });
    try {
      await _servis.kur(_yayin!, (p) {
        if (mounted) setState(() => _ilerleme = p);
      });
    } on GuncellemeSayfasiAcildi {
      if (mounted) {
        setState(() => _hata = 'Güncelleme sayfası açıldı. Yeni sürümü oradan indir.');
      }
    } catch (_) {
      if (mounted) {
        setState(() => _hata = 'Güncelleme tamamlanamadı. Yeniden dene.');
      }
    } finally {
      if (mounted) setState(() => _mesgul = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    if (_yayin == null || _gizli) return const SizedBox.shrink();
    return Card(
      key: const Key('guncelle_karti'),
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Text(
              'Yeni sürüm hazır: ${_yayin!.surum}',
              style: const TextStyle(fontWeight: FontWeight.bold),
            ),
            const SizedBox(height: 8),
            Text(
              _hata ??
                  (_mesgul
                      ? (_ilerleme >= 1
                            ? 'Kurulum hazırlanıyor…'
                            : 'İndiriliyor… %${(_ilerleme * 100).round()}')
                      : (Platform.isAndroid
                            ? 'Güncelleme indirilecek; açılan ekranda kurulumu onayla.'
                            : 'Güncelleme indirilip kurulacak; uygulama yeniden açılacak.')),
            ),
            if (_mesgul) LinearProgressIndicator(value: _ilerleme),
            const SizedBox(height: 8),
            Row(
              children: [
                Expanded(child: FilledButton.icon(
              key: const Key('guncelle_dugme'),
              onPressed: _mesgul ? null : _kur,
              icon: const Icon(Icons.system_update),
              label: const Text('Güncelle'),
                )),
                TextButton(
                  key: const Key('guncelle_sonra'),
                  onPressed: _mesgul ? null : () {
                    GuncelleKarti.ertelenen.add(_yayin!.surum);
                    setState(() => _gizli = true);
                  },
                  child: const Text('Sonra'),
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }
}
