import 'dart:async';

import 'package:flutter/material.dart';

import '../guncelleme.dart';

/// Açılışta ve günde bir denetler; kurulum Ayarlar'dan yapılır.
class GuncelleKarti extends StatefulWidget {
  const GuncelleKarti({super.key, this.servis, this.ayarDugmesi = false});
  final Guncelleyici? servis;
  final bool ayarDugmesi;
  @override
  State<GuncelleKarti> createState() => _GuncelleKartiDurum();
}

class _GuncelleKartiDurum extends State<GuncelleKarti> {
  late final Guncelleyici _servis;
  Timer? _timer;
  @override
  void initState() {
    super.initState();
    _servis = widget.servis ?? guncelleyici;
    guncellemeKontrolIstegi.addListener(_bak);
    _bak();
    _timer = Timer.periodic(const Duration(days: 1), (_) => _bak());
  }

  @override
  void dispose() {
    _timer?.cancel();
    guncellemeKontrolIstegi.removeListener(_bak);
    super.dispose();
  }

  Future<void> _bak() async {
    try {
      await _servis.denetle();
    } catch (_) {
      // Ağ yoksa ana kullanım etkilenmez; Ayarlar'da yeniden denenebilir.
    }
  }

  @override
  Widget build(BuildContext context) => ListenableBuilder(
    listenable: _servis,
    builder: (context, _) {
      void ac() => Navigator.pushNamed(context, '/ayarlar');
      if (widget.ayarDugmesi) {
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
