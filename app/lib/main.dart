import 'dart:async';
import 'dart:io' show Platform;

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import 'motor.dart';
import 'hesap/hesap_servisi.dart';
import 'hesap/gercek_hesap_servisi.dart';
import 'sayfalar/oturum.dart';
import 'hesap/hesap_sayfasi.dart';
import 'sayfalar/ana.dart';
import 'sayfalar/ayarlar.dart';
import 'sayfalar/baglanti_ver.dart';
import 'src/rust/api/afudesk.dart' as rust;
import 'src/rust/frb_generated.dart';
import 'tema.dart';
import 'arka_plan/arka_plan.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await RustLib.init();
  // Kalıcı cihaz kimliği ve kayıtlı cihazlar: masaüstünde çekirdek kendi bulur
  // (%LOCALAPPDATA%\AfuDesk); Android'de uygulamaya özel klasörü Kotlin verir.
  if (Platform.isAndroid) {
    try {
      final yol = await const MethodChannel(
        'afudesk/kol',
      ).invokeMethod<String>('veriKlasoru');
      if (yol != null) rust.veriKlasoruAyarla(yol: yol);
    } catch (_) {
      // Klasör alınamazsa kayıtlı cihazlar olmadan çalışır (kodla bağlantı etkilenmez).
    }
  }
  final motor = RustMotor();
  final hesap = GercekHesapServisi.rust(motor);
  // Restore after the UI/host starts; an offline PC must still open immediately.
  unawaited(hesap.geriYukle());
  if (Platform.isWindows) {
    ArkaPlan.kanal.setMethodCallHandler((call) async {
      if (call.method == 'durdur' || call.method == 'cik') {
        try {
          await hesap.duraklat();
        } finally {
          if (call.method == 'cik') await ArkaPlan.kanal.invokeMethod<void>('kapat');
        }
      }
    });
  }
  runApp(
    AfuDeskUygulama(motor: motor, hesapServisi: hesap),
  );
}

/// Uygulama kökü: bağlantı isteklerinin onay penceresini hangi ekranda olunursa olsun
/// gösterir (kayıtlı cihazlar "Bağlantı ver" açık olmasa da bağlanmak isteyebilir).
class AfuDeskUygulama extends StatefulWidget {
  final Motor motor;
  // Servisin yaşam süresini çağıran yönetir; null eski akışı korur.
  final HesapServisi? hesapServisi;
  const AfuDeskUygulama({super.key, required this.motor, this.hesapServisi});

  @override
  State<AfuDeskUygulama> createState() => _AfuDeskUygulamaDurum();
}

class _AfuDeskUygulamaDurum extends State<AfuDeskUygulama> {
  final _gezgin = GlobalKey<NavigatorState>();
  StreamSubscription<HostOlay>? _abonelik;
  bool _istekAcik = false;

  @override
  void initState() {
    super.initState();
    final servis = widget.hesapServisi;
    if (servis is GercekHesapServisi) {
      servis.oturumAc = (cihaz, ac, {required terminalleBasla}) {
        _gezgin.currentState?.push(
          MaterialPageRoute(
            builder: (_) => OturumSayfasi(
              motor: widget.motor,
              baglantiAc: ac,
              terminalleBasla: terminalleBasla,
            ),
          ),
        );
      };
    }
    if (widget.motor.baglantiVerilebilir) {
      // Hataları "Bağlantı ver" ekranı gösterir; kökte yalnız istek/bağlantı izlenir.
      _abonelik = widget.motor.hostOlaylari.listen(
        _hostOlay,
        onError: (Object _) {},
      );
      widget.motor.arkaPlanBaslat();
    }
  }

  @override
  void dispose() {
    _abonelik?.cancel();
    super.dispose();
  }

  void _hostOlay(HostOlay o) {
    switch (o.tur) {
      case 'istek':
        _istekGoster(o);
      case 'baglandi':
        // Kayıtlı cihaz arka planda bağlandı: oturumu (ve Kes düğmesini) göster.
        if (!BaglantiVerSayfasi.acik) {
          _gezgin.currentState?.push(
            MaterialPageRoute(
              builder: (_) => BaglantiVerSayfasi(motor: widget.motor),
            ),
          );
        }
    }
  }

  Future<void> _istekGoster(HostOlay o) async {
    final baglam = _gezgin.currentState?.overlay?.context;
    if (_istekAcik || baglam == null) return;
    _istekAcik = true;
    final sonuc = await showDialog<IstekKarari?>(
      context: baglam,
      barrierDismissible: false,
      builder: (_) => IstekPenceresi(ad: o.ad, kayitli: o.kayitli),
    );
    _istekAcik = false;
    if (sonuc == null) {
      await widget.motor.hostRed();
    } else {
      await widget.motor.hostKabul(
        kontrol: sonuc.kontrol,
        pano: sonuc.pano,
        dosya: sonuc.dosya,
        oyunKolu: sonuc.oyunKolu,
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      navigatorKey: _gezgin,
      title: 'AfuDesk',
      debugShowCheckedModeBanner: false,
      theme: afuTema(),
      routes: {
        '/ayarlar': (_) => AyarlarSayfasi(hesapServisi: widget.hesapServisi),
      },
      builder: (context, child) => widget.hesapServisi == null
          ? child!
          : HesapBaglantiSeridi(servis: widget.hesapServisi!, child: child!),
      home: widget.hesapServisi == null
          ? AnaSayfa(motor: widget.motor)
          : HesapSayfasi(
              servis: widget.hesapServisi!,
              kodlaBaglan: () {
                _gezgin.currentState?.push(
                  MaterialPageRoute(
                    builder: (_) => AnaSayfa(motor: widget.motor),
                  ),
                );
              },
            ),
    );
  }
}
