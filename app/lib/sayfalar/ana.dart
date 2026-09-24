import 'package:flutter/material.dart';

import '../bilesenler/afu_uygulamalar.dart';
import '../motor.dart';
import '../tema.dart';
import 'baglan.dart';
import 'baglanti_ver.dart';

/// Ana ekran: iki büyük seçenek + Afu uygulamaları.
class AnaSayfa extends StatelessWidget {
  final Motor motor;
  const AnaSayfa({super.key, required this.motor});

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: SafeArea(
        child: Center(
          child: ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 760),
            child: ListView(
              padding: const EdgeInsets.all(24),
              children: [
                Row(children: [
                  const Icon(Icons.desktop_windows_rounded, color: Renk.vurgu, size: 30),
                  const SizedBox(width: 10),
                  Text('AfuDesk', style: Theme.of(context).textTheme.headlineMedium?.copyWith(fontWeight: FontWeight.w700)),
                ]),
                const SizedBox(height: 6),
                const Text('Sunucusuz uzak masaüstü — tek kodla doğrudan bağlan.',
                    style: TextStyle(color: Renk.soluk)),
                const SizedBox(height: 24),
                LayoutBuilder(builder: (context, c) {
                  final dar = c.maxWidth < 560;
                  final kartlar = [
                    _SecenekKarti(
                      anahtar: const Key('secenek_ver'),
                      simge: Icons.screen_share_rounded,
                      baslik: 'Bağlantı ver',
                      aciklama: 'Ekranını paylaş. Sana bir kod ve parola verilir, bunları karşı tarafa gönder.',
                      onTap: () => Navigator.push(
                          context, MaterialPageRoute(builder: (_) => BaglantiVerSayfasi(motor: motor))),
                    ),
                    _SecenekKarti(
                      anahtar: const Key('secenek_baglan'),
                      simge: Icons.cast_connected_rounded,
                      baslik: 'Bağlan',
                      aciklama: 'Sana gönderilen kodu ve parolayı gir, karşı tarafın ekranına bağlan.',
                      onTap: () =>
                          Navigator.push(context, MaterialPageRoute(builder: (_) => BaglanSayfasi(motor: motor))),
                    ),
                  ];
                  return dar
                      ? Column(children: [kartlar[0], const SizedBox(height: 14), kartlar[1]])
                      // Yan yana kartlar aynı yükseklikte.
                      : IntrinsicHeight(
                          child: Row(crossAxisAlignment: CrossAxisAlignment.stretch, children: [
                            Expanded(child: kartlar[0]),
                            const SizedBox(width: 14),
                            Expanded(child: kartlar[1]),
                          ]),
                        );
                }),
                const SizedBox(height: 28),
                const AfuUygulamalar(),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

class _SecenekKarti extends StatelessWidget {
  final Key anahtar;
  final IconData simge;
  final String baslik;
  final String aciklama;
  final VoidCallback onTap;
  const _SecenekKarti(
      {required this.anahtar, required this.simge, required this.baslik, required this.aciklama, required this.onTap});

  @override
  Widget build(BuildContext context) {
    return Card(
      clipBehavior: Clip.antiAlias,
      child: InkWell(
        key: anahtar,
        onTap: onTap,
        child: Padding(
          padding: const EdgeInsets.all(20),
          child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
            Container(
              padding: const EdgeInsets.all(10),
              decoration: BoxDecoration(color: Renk.vurgu.withValues(alpha: 0.14), borderRadius: BorderRadius.circular(12)),
              child: Icon(simge, color: Renk.vurgu, size: 28),
            ),
            const SizedBox(height: 14),
            Text(baslik, style: const TextStyle(fontSize: 19, fontWeight: FontWeight.w700)),
            const SizedBox(height: 6),
            Text(aciklama, style: const TextStyle(color: Renk.soluk, height: 1.35)),
          ]),
        ),
      ),
    );
  }
}
