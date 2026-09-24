import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../motor.dart';
import '../tema.dart';
import 'oturum.dart';

class BaglanSayfasi extends StatefulWidget {
  final Motor motor;
  const BaglanSayfasi({super.key, required this.motor});

  @override
  State<BaglanSayfasi> createState() => _BaglanDurum();
}

class _BaglanDurum extends State<BaglanSayfasi> {
  final _kod = TextEditingController();
  final _parola = TextEditingController();
  String? _kodHata;
  String? _parolaHata;

  Future<void> _yapistir() async {
    final v = await Clipboard.getData(Clipboard.kTextPlain);
    if (v?.text != null) setState(() => _kod.text = v!.text!.trim());
  }

  void _baglan() {
    final kod = _kod.text.replaceAll(RegExp(r'\s'), '');
    final parola = _parola.text.trim();
    setState(() {
      _kodHata = kod.isEmpty
          ? 'Bağlantı kodunu yapıştır.'
          : (!kod.startsWith('AFU') ? 'Bu bir AfuDesk kodu değil (AFU2. ile başlamalı).' : null);
      _parolaHata = parola.isEmpty ? 'Parolayı yaz.' : null;
    });
    if (_kodHata != null || _parolaHata != null) return;
    Navigator.push(
      context,
      MaterialPageRoute(builder: (_) => OturumSayfasi(motor: widget.motor, kod: kod, parola: parola)),
    );
  }

  @override
  void dispose() {
    _kod.dispose();
    _parola.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('Bağlan')),
      body: Center(
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 560),
          child: ListView(padding: const EdgeInsets.all(24), children: [
            const Text('Sana gönderilen kodu ve parolayı gir.', style: TextStyle(color: Renk.soluk)),
            const SizedBox(height: 16),
            TextField(
              key: const Key('baglan_kod'),
              controller: _kod,
              minLines: 3,
              maxLines: 5,
              style: const TextStyle(fontFamily: 'Consolas', fontSize: 12),
              decoration: InputDecoration(
                labelText: 'Bağlantı kodu',
                hintText: 'AFU2.…',
                errorText: _kodHata,
                suffixIcon: IconButton(
                  key: const Key('baglan_yapistir'),
                  tooltip: 'Yapıştır',
                  icon: const Icon(Icons.content_paste_rounded),
                  onPressed: _yapistir,
                ),
              ),
            ),
            const SizedBox(height: 14),
            TextField(
              key: const Key('baglan_parola'),
              controller: _parola,
              obscureText: false,
              keyboardType: TextInputType.number,
              style: const TextStyle(fontSize: 20, letterSpacing: 4),
              decoration: InputDecoration(labelText: 'Parola', errorText: _parolaHata),
              onSubmitted: (_) => _baglan(),
            ),
            const SizedBox(height: 20),
            FilledButton.icon(
              key: const Key('baglan_dugme'),
              onPressed: _baglan,
              icon: const Icon(Icons.login_rounded),
              label: const Text('Bağlan'),
            ),
          ]),
        ),
      ),
    );
  }
}
