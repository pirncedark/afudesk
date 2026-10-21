import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter/services.dart';
import 'package:url_launcher/url_launcher.dart';

import '../motor.dart';
import '../src/rust/api/google.dart' as rust;
import 'hesap_servisi.dart';
import 'google_hatasi.dart';
import 'oturum_kasasi.dart';
import '../arka_plan/arka_plan.dart';

typedef GoogleOturumAc =
    void Function(Cihaz cihaz, Stream<IzleyiciOlay> Function() ac);

class GercekHesapServisi extends HesapServisi {
  GercekHesapServisi({
    required this.islem,
    required this.googleBaglan,
    required this.kes,
    required Stream<HostOlay> hostOlaylari,
    required this.platformGirisi,
    required this.tarayiciAc,
    this.taniYaz = googleTaniYaz,
    this.kasa,
    this.bekle = Future<void>.delayed,
  }) {
    _host = hostOlaylari.listen((o) {
      if (o.tur == 'baglandi') {
        _gelen = Cihaz(id: 'gelen', ad: o.ad, cevrimici: true);
        _yayinla();
      } else if (o.tur == 'koptu' || o.tur == 'hata') {
        _gelen = null;
        _yayinla();
      }
    });
  }

  factory GercekHesapServisi.rust(RustMotor motor) => GercekHesapServisi(
    kasa: Platform.isWindows ? WindowsOturumKasasi() : null,
    islem: (islem, veri) async => rust.googleIslem(
      islem: islem,
      veri: islem == 'baslat' && Platform.isWindows
          ? jsonEncode({
              'desktopClientId': const String.fromEnvironment(
                'AFUDESK_GOOGLE_DESKTOP_CLIENT_ID',
              ),
            })
          : islem == 'baslat' && Platform.isAndroid
          ? jsonEncode(
              await const MethodChannel(
                'afudesk/google',
              ).invokeMapMethod<String, dynamic>('ayarlar'),
            )
          : veri,
    ),
    googleBaglan: motor.googleBaglan,
    kes: () async {
      await motor.hostKes();
      await motor.izleyiciKapat();
    },
    hostOlaylari: motor.hostOlaylari,
    platformGirisi: (nonce) async => Map<String, dynamic>.from(
      (await const MethodChannel(
        'afudesk/google',
      ).invokeMapMethod<String, dynamic>('giris', {'nonce': nonce}))!,
    ),
    tarayiciAc: (url) =>
        launchUrl(Uri.parse(url), mode: LaunchMode.externalApplication),
  );

  final Future<String> Function(String, String) islem;
  final Stream<IzleyiciOlay> Function(String) googleBaglan;
  final Future<void> Function() kes;
  final Future<Map<String, dynamic>> Function(String) platformGirisi;
  final Future<bool> Function(String) tarayiciAc;
  final Future<void> Function(GoogleHatasi) taniYaz;
  final OturumKasasi? kasa;
  final Future<void> Function(Duration) bekle;
  Duration? sonBaglanmaSuresi;
  Future<void> _kasaSirasi = Future<void>.value();
  String? durum;
  bool _kapali = false, _duraklatildi = false;
  int _oturumNesli = 0;
  int _baglantiNesli = 0;
  Timer? _geriYukleme;
  GoogleOturumAc? oturumAc;
  late final StreamSubscription<HostOlay> _host;
  final _akis = StreamController<List<Cihaz>>.broadcast();
  Timer? _heartbeat;
  Hesap? _hesap;
  Cihaz? _gelen, _giden;
  bool _mesgul = false, _yeniliyor = false;

  @override
  Hesap? get hesap => _hesap;
  @override
  bool get uzaktanErisim => _hesap != null;
  @override
  List<Cihaz> get baglantilar => [?_gelen, ?_giden];
  @override
  Stream<List<Cihaz>> get aktifBaglantilar => _akis.stream;
  void _yayinla() {
    if (_kapali) return;
    _akis.add(baglantilar);
    notifyListeners();
  }

  Future<dynamic> _cagir(String op, [String data = '']) async =>
      jsonDecode(await islem(op, data));

  void _saatBaslat() {
    _heartbeat?.cancel();
    _heartbeat = Timer.periodic(const Duration(seconds: 60), (_) => _yenile());
  }

  Future<void> _kasada(Future<void> Function() islem) {
    final sonuc = _kasaSirasi.then((_) => islem());
    _kasaSirasi = sonuc.then<void>((_) {}, onError: (Object _) {});
    return sonuc;
  }

  Future<void> _kimligiAl(Map identity) async {
    final nesil = _oturumNesli;
    final oturum = identity['oturum'];
    if (oturum is String && kasa != null) {
      await _kasada(() async {
        if (!_kapali && nesil == _oturumNesli) await kasa!.yaz(oturum);
      });
    }
    if (_kapali || nesil != _oturumNesli) return;
    _hesap = Hesap(eposta: '', ad: identity['ad'] as String);
    durum = null;
    _saatBaslat();
    _yayinla();
  }

  Future<void> geriYukle() async {
    if (kasa == null || _mesgul || _hesap != null || _kapali || _duraklatildi) return;
    _mesgul = true;
    final nesil = _oturumNesli;
    var etkinlesti = false;
    try {
      final kayit = await kasa!.oku();
      if (kayit == null || _kapali) return;
      final identity = await _cagir('geriYukle', kayit) as Map;
      etkinlesti = true;
      if (nesil != _oturumNesli || _kapali) return;
      await _kimligiAl(identity);
    } catch (e) {
      if (_kapali || nesil != _oturumNesli) return;
      // Persistence can fail after native authorisation succeeded. Revoke that
      // invisible identity before retrying, but retain the encrypted grant.
      if (etkinlesti) {
        try { await _cagir('duraklat'); } catch (_) {}
      }
      if ('$e'.contains('(G6)')) {
        await _kasada(() => kasa!.sil());
        durum = 'Google ile yeniden giriş yap';
      } else {
        durum = 'Bilgisayar hazır olamadı, internet bağlantını kontrol et.';
        _geriYukleme?.cancel();
        _geriYukleme = Timer(const Duration(seconds: 30), geriYukle);
      }
      _yayinla();
    } finally { _mesgul = false; }
  }

  Future<void> duraklat() async {
    _oturumNesli++;
    _duraklatildi = true;
    _baglantiNesli++;
    _heartbeat?.cancel();
    _geriYukleme?.cancel();
    try {
      await _cagir('duraklat');
    } finally {
      try {
        await kes();
      } finally {
        _hesap = null;
        _gelen = null;
        _giden = null;
        durum = 'Bağlantılar durduruldu; devam etmek için uygulamayı yeniden aç.';
        _yayinla();
      }
    }
  }

  @override
  Future<void> girisYap() async {
    if (_mesgul || _hesap != null) return;
    _duraklatildi = false;
    _geriYukleme?.cancel();
    _mesgul = true;
    final nesil = _oturumNesli;
    var asama = 'baslat';
    try {
      final start = await _cagir('baslat') as Map;
      String data = '';
      if (start['nonce'] is String) {
        asama = 'android';
        data = jsonEncode(await platformGirisi(start['nonce'] as String));
      } else if (!await tarayiciAc(start['url'] as String)) {
        throw const HesapHatasi('Tarayıcı açılamadı, yeniden dene.');
      }
      asama = 'tamamla';
      final identity = await _cagir('tamamla', data) as Map;
      if (nesil != _oturumNesli || _kapali) return;
      await _kimligiAl(identity);
    } catch (hata) {
      final tani = GoogleHatasi.cevir(hata, asama: asama);
      // Diagnostics must not delay cancellation or keep the UI busy on failure.
      unawaited(_taniKaydet(tani));
      try {
        // A failure to persist the completed grant must not leave an invisible
        // host authorised while the UI reports failed sign-in.
        await _cagir(asama == 'tamamla' ? 'cikis' : 'iptal');
      } catch (_) {
        /* Keep the original failure. */
      }
      if (hata is HesapHatasi && hata is! GoogleHatasi) rethrow;
      throw tani;
    } finally {
      _mesgul = false;
    }
  }

  Future<void> _taniKaydet(GoogleHatasi tani) async {
    try {
      await taniYaz(tani);
    } catch (_) {
      /* Storage cannot hide sign-in failure. */
    }
  }

  Future<void> _yenile() async {
    if (_yeniliyor || _mesgul || _hesap == null) return;
    _yeniliyor = true;
    final nesil = _oturumNesli;
    try {
      final identity = await _cagir('yenile');
      if (_kapali || nesil != _oturumNesli) return;
      if (identity is Map && identity['ad'] is String) {
        await _kimligiAl(identity);
      } else if (durum != null) {
        durum = null;
        _yayinla();
      }
    } catch (e) {
      if (_kapali || nesil != _oturumNesli) return;
      if ('$e'.contains('(G6)')) {
        await cikisYap();
        durum = 'Google ile yeniden giriş yap';
      } else {
        durum = 'Bağlantı yenilenemedi, internet bağlantını kontrol et.';
      }
      if (!_kapali) _yayinla();
    } finally {
      _yeniliyor = false;
    }
  }

  @override
  Future<List<Cihaz>> bilgisayarlarim() async => (await _cagir('liste') as List)
      .map(
        (v) => Cihaz(
          id: v['id'] as String,
          ad: v['ad'] as String,
          cevrimici: v['cevrimici'] as bool,
        ),
      )
      .toList();

  @override
  Future<void> baglan(String cihazId) async {
    final cihaz = (await bilgisayarlarim())
        .where((c) => c.id == cihazId && c.cevrimici)
        .firstOrNull;
    if (cihaz == null || oturumAc == null) {
      throw const HesapHatasi(
        'Bilgisayara ulaşılamadı, açık olduğunu kontrol et.',
      );
    }
    final nesil = ++_baglantiNesli;
    oturumAc!(cihaz, () async* {
      var deneme = 0;
      final saat = Stopwatch()..start();
      sonBaglanmaSuresi = null;
      try {
        while (nesil == _baglantiNesli && !_kapali) {
          var tekrar = false;
          try {
            await for (final o in googleBaglan(cihaz.id)) {
              if (nesil != _baglantiNesli || _kapali) return;
              if (o.tur == 'yeniden') {
                // Stop the core's longer resume loop; this account stream owns
                // exactly three fresh connection attempts with 1/2/4s waits.
                tekrar = true;
                await kes();
                break;
              }
              if (o.tur == 'hata' && deneme > 0) {
                tekrar = true;
                break;
              }
              if (o.tur == 'istatistik') deneme = 0;
              if (o.tur == 'kabul') {
                sonBaglanmaSuresi ??= saat.elapsed;
                _giden = cihaz;
                _yayinla();
              }
              if (o.tur == 'koptu' || o.tur == 'hata') {
                _giden = null;
                _yayinla();
              }
              yield o;
            }
          } catch (_) {
            if (deneme == 0) rethrow;
            tekrar = true;
          }
          if (!tekrar || nesil != _baglantiNesli || _kapali) return;
          _giden = null;
          _yayinla();
          final gecikme = yenidenBaglanmaBeklemesi(deneme++);
          if (gecikme == null) {
            yield IzleyiciOlay('koptu', metin: 'Bağlantı kurulamadı, bilgisayarın açık olduğunu kontrol et.');
            return;
          }
          yield IzleyiciOlay('yeniden', metin: 'Bağlantı koptu, yeniden bağlanılıyor… ($deneme/3)');
          await bekle(gecikme);
        }
      } finally {
        _giden = null;
        _yayinla();
      }
    });
  }

  @override
  Future<void> baglantiyiKes() async {
    _baglantiNesli++;
    await kes();
    // The host's actual disconnect event clears the incoming banner.
    _giden = null;
    _yayinla();
  }

  @override
  Future<void> cikisYap() async {
    _oturumNesli++;
    _baglantiNesli++;
    _geriYukleme?.cancel();
    _heartbeat?.cancel();
    _heartbeat = null;
    _hesap = null;
    durum = null;
    try {
      try {
        await _kasada(() async { await kasa?.sil(); });
      } finally {
        // Local revocation must run even if deleting the encrypted file fails.
        await _cagir('cikis');
      }
    } finally {
      await kes();
      _giden = null;
      _yayinla();
      if (Platform.isAndroid) {
        try {
          await const MethodChannel(
            'afudesk/google',
          ).invokeMethod<void>('cikis');
        } catch (_) {}
      }
    }
  }

  @override
  Future<void> uzaktanErisimAyarla(bool acik) => acik ? girisYap() : cikisYap();
  @override
  Future<void> cihazCikar(String cihazId) async {
    await _cagir('cikar', cihazId);
  }

  @override
  void dispose() {
    _oturumNesli++;
    _kapali = true;
    _baglantiNesli++;
    _geriYukleme?.cancel();
    _heartbeat?.cancel();
    _host.cancel();
    _akis.close();
    super.dispose();
  }
}
