import 'dart:async';
import 'dart:io';

import 'package:flutter/material.dart';

import '../guncelleme.dart';
import '../aktif_sayac.dart';

/// Açılışta, uygulamaya dönüşte ve günde bir denetler; kurulum Ayarlar'dan yapılır.
class GuncelleKarti extends StatefulWidget {
  const GuncelleKarti({super.key, this.servis, this.ayarDugmesi = false, this.automaticCheck});
  final Guncelleyici? servis;
  final bool ayarDugmesi;
  final bool? automaticCheck;
  @override
  State<GuncelleKarti> createState() => _GuncelleKartiDurum();
}

class _GuncelleKartiDurum extends State<GuncelleKarti>
    with WidgetsBindingObserver {
  late final Guncelleyici _servis;
  Timer? _timer;
  Timer? _donusBekleme;
  bool _kontrolSuruyor = false;
  bool _ayarOkundu = false;
  @override
  void initState() {
    super.initState();
    _servis = widget.servis ?? guncelleyici;
    _ayarYukle();
    WidgetsBinding.instance.addObserver(this);
    guncellemeKontrolIstegi.addListener(_bak);
    _bak();
    _timer = Timer.periodic(const Duration(days: 1), (_) => _bak());
  }

  Future<void> _ayarYukle() async {
    var acik = true;
    try { acik = widget.automaticCheck ?? await acilistaDenetleAcik(); } catch (_) { }
    _ayarOkundu = true;
    if (mounted && acik) {
      _bak();
      if (Platform.isAndroid || Platform.isWindows) {
        unawaited(() async {
          try {
            if (await yerelAyarOku('sayacaKatil') == 'false') return;
            final surum = await _servis.surum();
            await AktifSayac().yukle(katil: true, surum: surum);
          } catch (_) { }
        }());
      }
    }
  }

  @override
  void dispose() {
    _timer?.cancel();
    _donusBekleme?.cancel();
    WidgetsBinding.instance.removeObserver(this);
    guncellemeKontrolIstegi.removeListener(_bak);
    super.dispose();
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    if (state == AppLifecycleState.resumed && _donusBekleme == null) {
      _bak();
    }
  }

  Future<void> _bak() async {
    if (!_ayarOkundu) return;
    try { if (!(widget.automaticCheck ?? await acilistaDenetleAcik())) return; } catch (_) { }
    if (_kontrolSuruyor) return;
    _kontrolSuruyor = true;
    try {
      await _servis.denetle();
      if (!mounted) return;
      _donusBekleme?.cancel();
      _donusBekleme = Timer(const Duration(minutes: 5), () {
        _donusBekleme = null;
      });
    } catch (_) {
      // Sonraki uygulamaya dönüşte ağ denetimini yeniden dene.
      _donusBekleme?.cancel();
      _donusBekleme = null;
    } finally {
      _kontrolSuruyor = false;
    }
  }

  @override
  Widget build(BuildContext context) => ListenableBuilder(
    listenable: _servis,
    builder: (context, _) {
      void ac() => Navigator.pushNamed(context, '/ayarlar');
      if (widget.ayarDugmesi) {
        if (_servis.yayin != null) {
          return TextButton.icon(
            onPressed: ac,
            icon: const Icon(Icons.system_update),
            label: const Text('Güncelle'),
          );
        }
        return IconButton(
          tooltip: 'Ayarlar',
          onPressed: ac,
          icon: Badge(
            isLabelVisible: _servis.yayin != null,
            child: const Icon(Icons.settings_outlined),
          ),
        );
      }
      if (_servis.yayin == null) return const SizedBox.shrink();
      return Card(
        key: const Key('guncelle_karti'),
        child: ListTile(
          dense: true,
          leading: const Icon(Icons.system_update),
          title: const Text('Yeni sürüm hazır — dokun'),
          trailing: const Icon(Icons.chevron_right),
          onTap: ac,
        ),
      );
    },
  );
}
