import 'package:flutter/material.dart';
import 'dart:io';
import 'package:flutter/services.dart';
import '../arka_plan/arka_plan.dart';
import '../aktif_sayac.dart';

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
  static const _paylasKanali = MethodChannel('afudesk/paylas');
  static const _paylasimMetni = 'AfuDesk ile telefondan bilgisayarını kullan: https://github.com/pirncedark/afudesk/releases/latest';
  late final Guncelleyici _servis;
  String _surum = 'Okunuyor…';
  String? _hata, _bilgi, _hesapHatasi;
  bool _mesgul = false, _cikiliyor = false;
  double? _ilerleme;
  bool _arkaPlan = false, _arkaPlanMesgul = true;
  bool _acilistaDenetle = true, _sayacaKatil = true;
  bool _sayacYukleniyor = true;
  Map<String, dynamic>? _sayac;

  @override
  void initState() {
    super.initState();
    _servis = widget.servis ?? guncelleyici;
    _servis.addListener(_degisti);
    widget.hesapServisi?.addListener(_degisti);
    _surumuOku();
    _tercihleriOku();
    if (Platform.isWindows) _arkaPlanOku();
  }

  Future<void> _tercihleriOku() async {
    try {
      _acilistaDenetle = await acilistaDenetleAcik();
      _sayacaKatil = (await yerelAyarOku('sayacaKatil')) != 'false';
      if (_sayacaKatil && (Platform.isAndroid || Platform.isWindows)) {
        await _sayaciYukle();
      } else { _sayacYukleniyor = false; }
    } catch (_) { _sayac = null; }
    if (mounted) setState(() {});
  }

  Future<void> _sayaciYukle() async {
    try {
      final ver = await _servis.surum();
      final sonuc = await AktifSayac().yukle(katil: _sayacaKatil, surum: ver);
      if (mounted) setState(() { _sayac = sonuc.containsKey('aktif7') ? sonuc : null; _sayacYukleniyor = false; });
    } catch (_) { if (mounted) setState(() { _sayac = null; _sayacYukleniyor = false; }); }
  }

  Future<void> _sayacTercihi(bool acik) async {
    setState(() { _sayacaKatil = acik; _sayacYukleniyor = acik; if (!acik) _sayac = null; });
    try {
      await yerelAyarYaz('sayacaKatil', '$acik');
      if (acik) await _sayaciYukle();
    } catch (_) { if (mounted) setState(() => _sayac = null); }
  }

  Future<void> _linkPaylas() async {
    try {
      if (Platform.isAndroid) {
        await _paylasKanali.invokeMethod<void>('metin', _paylasimMetni);
      } else {
        await Clipboard.setData(const ClipboardData(text: _paylasimMetni));
        if (mounted) ScaffoldMessenger.of(context).showSnackBar(const SnackBar(content: Text('Bağlantı kopyalandı.')));
      }
    } catch (_) { if (mounted) ScaffoldMessenger.of(context).showSnackBar(const SnackBar(content: Text('Paylaşım açılamadı. Yeniden dene.'))); }
  }

  Future<void> _apkPaylas() async {
    try { await _paylasKanali.invokeMethod<void>('apk'); }
    catch (_) { if (mounted) ScaffoldMessenger.of(context).showSnackBar(const SnackBar(content: Text('Paylaşım açılamadı. Yeniden dene.'))); }
  }

  Future<void> _arkaPlanOku() async {
    try {
      final acik = await ArkaPlan.oku();
      if (mounted) setState(() => _arkaPlan = acik);
    } catch (_) {
      if (mounted) setState(() => _hata = 'Başlangıç ayarı okunamadı, Ayarlar’ı yeniden aç.');
    } finally {
      if (mounted) setState(() => _arkaPlanMesgul = false);
    }
  }

  Future<void> _arkaPlanAyarla(bool acik) async {
    setState(() => _arkaPlanMesgul = true);
    try {
      await ArkaPlan.ayarla(acik);
      if (mounted) {
        setState(() {
          _arkaPlan = acik;
          _bilgi = acik ? 'AfuDesk arka planda hazır; pencereyi kapatınca tepside kalır.' : 'Otomatik başlangıç kapatıldı.';
          _hata = null;
        });
      }
    } catch (_) {
      if (mounted) setState(() => _hata = 'Başlangıç ayarı değiştirilemedi, yeniden dene.');
    } finally {
      if (mounted) setState(() => _arkaPlanMesgul = false);
    }
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
                  _kart('Güncelleme', [
                    ListTile(title: Text('Sürüm $_surum'), subtitle: durum == null ? null : Semantics(liveRegion: true, child: Text(durum))),
                    if (_mesgul) LinearProgressIndicator(value: _ilerleme),
                    Padding(padding: const EdgeInsets.symmetric(horizontal: 16), child: FilledButton.icon(
                      onPressed: _mesgul || _cikiliyor ? null : yayin == null ? _denetle : _kur,
                      icon: Icon(yayin == null ? Icons.refresh : Icons.system_update),
                      label: Text(yayin == null ? 'Güncellemeleri denetle' : 'Güncelle'),
                    )),
                    SwitchListTile(title: const Text('Açılışta kontrol et'), value: _acilistaDenetle,
                      onChanged: (v) async { setState(() => _acilistaDenetle = v); try { await acilistaDenetleKaydet(v); } catch (_) { if (mounted) setState(() => _acilistaDenetle = !v); } }),
                  ]),
                  _kart('Paylaş', [
                    const ListTile(title: Text("AfuDesk'i arkadaşına gönder")),
                    Padding(padding: const EdgeInsets.symmetric(horizontal: 16), child: Wrap(spacing: 8, children: [
                      OutlinedButton.icon(onPressed: _linkPaylas, icon: const Icon(Icons.link), label: const Text('Link gönder')),
                      if (Platform.isAndroid) OutlinedButton.icon(onPressed: _apkPaylas, icon: const Icon(Icons.share), label: const Text('APK gönder')),
                    ])),
                    if (Platform.isAndroid) const Padding(padding: EdgeInsets.all(16), child: Text('APK, bu telefondaki sürümdür.')),
                  ]),
                  _kart('Topluluk', [
                    if (_sayacYukleniyor && _sayacaKatil) const ListTile(title: Text('Son 7 günde … kişi AfuDesk kullandı')),
                    if (_sayac != null) ListTile(title: Text('Son 7 günde ${_sayac!['aktif7']} kişi AfuDesk kullandı')),
                    SwitchListTile(title: const Text('Sayaca katıl'), value: _sayacaKatil, onChanged: _sayacTercihi),
                    const Padding(padding: EdgeInsets.fromLTRB(16, 0, 16, 16), child: Text('Yalnız rastgele bir numara gönderilir; ad, konum veya hesap bilgisi gitmez.')),
                  ]),
                  if (Platform.isWindows) SwitchListTile(
                    title: const Text('Bilgisayar açılınca hazır olsun'),
                    subtitle: const Text('Windows oturumun açıldığında arka planda bekler.'),
                    value: _arkaPlan, onChanged: _arkaPlanMesgul ? null : _arkaPlanAyarla,
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

  Widget _kart(String baslik, List<Widget> children) => Card(
    shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(16)),
    margin: const EdgeInsets.only(bottom: 16),
    child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
      Padding(padding: const EdgeInsets.fromLTRB(16, 14, 16, 0), child: Text(baslik, style: Theme.of(context).textTheme.labelLarge?.copyWith(color: Theme.of(context).colorScheme.onSurfaceVariant))),
      ...children,
    ]),
  );
}
