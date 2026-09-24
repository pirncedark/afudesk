import 'package:flutter/material.dart';
import 'package:url_launcher/url_launcher.dart';

import '../tema.dart';

/// Afu ekosistemindeki diğer uygulamalar.
class AfuUygulama {
  final String ad;
  final String aciklama;
  final IconData simge;
  final String adres;
  const AfuUygulama(this.ad, this.aciklama, this.simge, this.adres);
}

const afuUygulamalari = [
  AfuUygulama('AfuDM', 'İndirme yöneticisi (Windows)', Icons.download_rounded,
      'https://github.com/pirncedark/AfuDM/releases/latest'),
  AfuUygulama('AfuTube', 'Video indirici (Android)', Icons.smart_display_rounded,
      'https://github.com/pirncedark/AfuDM/releases?q=afutube'),
  AfuUygulama('AfuRemote', 'Telefondan TV kumandası', Icons.settings_remote_rounded,
      'https://github.com/pirncedark/AfuRemote/releases/latest'),
];

class AfuUygulamalar extends StatelessWidget {
  /// Testlerde bağlantı açmak yerine kaydetmek için.
  final void Function(String adres)? ac;
  const AfuUygulamalar({super.key, this.ac});

  @override
  Widget build(BuildContext context) {
    return Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
      const Text('Afu Uygulamaları', style: TextStyle(fontWeight: FontWeight.w700, fontSize: 15)),
      const SizedBox(height: 10),
      Wrap(spacing: 10, runSpacing: 10, children: [
        for (final u in afuUygulamalari)
          ActionChip(
            key: Key('afu_${u.ad}'),
            avatar: Icon(u.simge, size: 18, color: Renk.vurgu),
            label: Text(u.ad),
            tooltip: u.aciklama,
            onPressed: () => ac != null
                ? ac!(u.adres)
                : launchUrl(Uri.parse(u.adres), mode: LaunchMode.externalApplication),
          ),
      ]),
    ]);
  }
}
