import 'package:flutter/material.dart';

import '../bilesenler/afu_uygulamalar.dart';
import '../motor.dart';
import '../tema.dart';
import 'baglan.dart';
import 'baglanti_ver.dart';
import 'oturum.dart';

/// Ana ekran: iki büyük seçenek, kayıtlı cihazlar ve Afu uygulamaları.
class AnaSayfa extends StatefulWidget {
  final Motor motor;
  const AnaSayfa({super.key, required this.motor});

  @override
  State<AnaSayfa> createState() => _AnaSayfaDurum();
}

class _AnaSayfaDurum extends State<AnaSayfa> {
  Motor get motor => widget.motor;
  List<KayitliCihaz> _kayitli = const [];

  @override
  void initState() {
    super.initState();
    _kayitli = motor.kayitliCihazlar();
  }

  /// Başka ekrandan dönünce liste yenilenir (yeni kayıt / kaldırılan cihaz).
  Future<void> _git(Widget sayfa) async {
    await Navigator.push(context, MaterialPageRoute(builder: (_) => sayfa));
    if (mounted) setState(() => _kayitli = motor.kayitliCihazlar());
  }

  void _unut(KayitliCihaz c) {
    motor.kayitliUnut(c.kimlik);
    setState(() => _kayitli = motor.kayitliCihazlar());
    ScaffoldMessenger.of(context)
      ..hideCurrentSnackBar()
      ..showSnackBar(SnackBar(content: Text('${c.ad} unutuldu. Yeniden bağlanmak için kod gerekecek.')));
  }

  Widget _kayitliCihazlar() {
    if (_kayitli.isEmpty) return const SizedBox.shrink();
    return Column(key: const Key('kayitli_liste'), crossAxisAlignment: CrossAxisAlignment.stretch, children: [
      const SizedBox(height: 24),
      const Text('Kayıtlı cihazlar', style: TextStyle(fontSize: 17, fontWeight: FontWeight.w700)),
      const SizedBox(height: 4),
      const Text('Kod ve parola olmadan bağlan. Karşı taraf yine onay verir.',
          style: TextStyle(color: Renk.soluk, fontSize: 13)),
      const SizedBox(height: 8),
      for (final (i, c) in _kayitli.indexed)
        Card(
          child: ListTile(
            leading: const Icon(Icons.computer_rounded, color: Renk.vurgu),
            title: Text(c.ad, key: Key('kayitli_ad_$i')),
            subtitle: Text('Son görülme: ${goreliZaman(c.sonGorulme)}'),
            trailing: Wrap(spacing: 6, children: [
              TextButton(key: Key('kayitli_unut_$i'), onPressed: () => _unut(c), child: const Text('Unut')),
              FilledButton(
                key: Key('kayitli_baglan_$i'),
                onPressed: () => _git(OturumSayfasi(motor: motor, kayitliKimlik: c.kimlik)),
                child: const Text('Bağlan'),
              ),
            ]),
          ),
        ),
    ]);
  }

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
                const Text('Tek kodla uzak masaüstü — hesap ve modem ayarı gerekmez.',
                    style: TextStyle(color: Renk.soluk)),
                const SizedBox(height: 24),
                LayoutBuilder(builder: (context, c) {
                  final dar = c.maxWidth < 560;
                  final kartlar = [
                    _SecenekKarti(
                      anahtar: const Key('secenek_ver'),
                      simge: Icons.screen_share_rounded,
                      baslik: 'Bağlantı ver',
                      aciklama: motor.baglantiVerilebilir
                          ? 'Ekranını paylaş. Sana bir kod ve parola verilir, bunları karşı tarafa gönder.'
                          : 'Bu cihazdan ekran paylaşımı yakında. Şimdilik bir bilgisayara bağlanabilirsin.',
                      onTap: motor.baglantiVerilebilir ? () => _git(BaglantiVerSayfasi(motor: motor)) : null,
                    ),
                    _SecenekKarti(
                      anahtar: const Key('secenek_baglan'),
                      simge: Icons.cast_connected_rounded,
                      baslik: 'Bağlan',
                      aciklama: 'Sana gönderilen kodu ve parolayı gir, karşı tarafın ekranına bağlan.',
                      onTap: () => _git(BaglanSayfasi(motor: motor)),
                    ),
                  ];
                  if (!motor.baglantiVerilebilir) kartlar.setAll(0, [kartlar[1], kartlar[0]]);
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
                _kayitliCihazlar(),
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
  /// null: devre dışı (ör. telefonda ekran paylaşımı henüz yok).
  final VoidCallback? onTap;
  const _SecenekKarti(
      {required this.anahtar, required this.simge, required this.baslik, required this.aciklama, required this.onTap});

  @override
  Widget build(BuildContext context) {
    final kapali = onTap == null;
    return Opacity(opacity: kapali ? 0.55 : 1, child: Card(
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
    ));
  }
}
