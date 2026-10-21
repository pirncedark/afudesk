import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter/services.dart';
import 'package:url_launcher/url_launcher.dart';

import '../motor.dart';
import '../src/rust/api/google.dart' as rust;
import 'hesap_servisi.dart';
import 'google_hatasi.dart';

typedef GoogleOturumAc =
    void Function(
      Cihaz cihaz,
      Stream<IzleyiciOlay> Function() ac, {
      required bool terminalleBasla,
    });

class GercekHesapServisi extends HesapServisi {
  GercekHesapServisi({
    required this.islem,
    required this.googleBaglan,
    required this.kes,
    required Stream<HostOlay> hostOlaylari,
    required this.platformGirisi,
    required this.tarayiciAc,
    this.taniYaz = googleTaniYaz,
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
    _akis.add(baglantilar);
    notifyListeners();
  }

  Future<dynamic> _cagir(String op, [String data = '']) async =>
      jsonDecode(await islem(op, data));

  @override
  Future<void> girisYap() async {
    if (_mesgul || _hesap != null) return;
    _mesgul = true;
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
      _hesap = Hesap(eposta: '', ad: identity['ad'] as String);
      _heartbeat = Timer.periodic(
        const Duration(seconds: 60),
        (_) => _yenile(),
      );
      _yayinla();
    } catch (hata) {
      final tani = GoogleHatasi.cevir(hata, asama: asama);
      // Diagnostics must not delay cancellation or keep the UI busy on failure.
      unawaited(_taniKaydet(tani));
      try {
        await _cagir('iptal');
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
    try {
      await _cagir('yenile');
    } catch (_) {
      await cikisYap();
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
  Future<void> baglan(String cihazId, {bool terminalleBasla = false}) async {
    final cihaz = (await bilgisayarlarim())
        .where((c) => c.id == cihazId && c.cevrimici)
        .firstOrNull;
    if (cihaz == null || oturumAc == null) {
      throw const HesapHatasi(
        'Bilgisayara ulaşılamadı, açık olduğunu kontrol et.',
      );
    }
    oturumAc!(cihaz, () async* {
      try {
        await for (final o in googleBaglan(cihaz.id)) {
          if (o.tur == 'kabul') {
            _giden = cihaz;
            _yayinla();
          }
          if (o.tur == 'koptu' || o.tur == 'hata') {
            _giden = null;
            _yayinla();
          }
          yield o;
        }
      } finally {
        _giden = null;
        _yayinla();
      }
    }, terminalleBasla: terminalleBasla);
  }

  @override
  Future<void> baglantiyiKes() async {
    await kes();
    // The host's actual disconnect event clears the incoming banner.
    _giden = null;
    _yayinla();
  }

  @override
  Future<void> cikisYap() async {
    _heartbeat?.cancel();
    _heartbeat = null;
    _hesap = null;
    try {
      await _cagir('cikis');
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
    _heartbeat?.cancel();
    _host.cancel();
    _akis.close();
    super.dispose();
  }
}
