// Arayüz ile Rust çekirdeği arasındaki sözleşme. Testler SahteMotor kullanır.
import 'dart:async';
import 'dart:io' show Platform;
import 'dart:typed_data';

import 'src/rust/api/afudesk.dart' as rust;

/// Host tarafı olayı.
class HostOlay {
  final String tur; // hazir | istek | baglandi | koptu | hata
  final String kod;
  final String parola;
  final String erisim;
  final String ad;
  final String metin;
  final bool kontrol;
  const HostOlay(this.tur,
      {this.kod = '',
      this.parola = '',
      this.erisim = '',
      this.ad = '',
      this.metin = '',
      this.kontrol = false});
}

/// İzleyici tarafı olayı.
class IzleyiciOlay {
  final String tur; // bekliyor | kabul | kare | koptu
  final String ad;
  final String metin;
  final bool kontrol;
  final int genislik;
  final int yukseklik;
  final Uint8List rgba;
  IzleyiciOlay(this.tur,
      {this.ad = '',
      this.metin = '',
      this.kontrol = false,
      this.genislik = 0,
      this.yukseklik = 0,
      Uint8List? rgba})
      : rgba = rgba ?? Uint8List(0);
}

/// Karşı tarafa gönderilen girdi.
class Girdi {
  final String tur; // konum | fare | kaydir | tus | metin
  final double x, y;
  final String ad;
  final bool basili;
  final int dx, dy;
  final String metin;
  const Girdi(this.tur,
      {this.x = 0,
      this.y = 0,
      this.ad = '',
      this.basili = false,
      this.dx = 0,
      this.dy = 0,
      this.metin = ''});
  @override
  String toString() => 'Girdi($tur, x=$x, y=$y, ad=$ad, basili=$basili, dx=$dx, dy=$dy)';
}

abstract class Motor {
  String cihazAdi();
  /// Bu cihaz ekranını paylaşabilir mi (şimdilik yalnız masaüstü).
  bool get baglantiVerilebilir;
  /// Dokunmatik kontrol kipi (telefon/tablet).
  bool get dokunmatik;
  Stream<HostOlay> hostBaslat({required String ad, String parola = '', bool upnp = true});
  Future<void> hostKabul({required bool kontrol});
  Future<void> hostRed();
  Future<void> hostKes();
  Future<void> hostDurdur();
  Stream<IzleyiciOlay> baglan({required String kod, required String parola, required String ad});
  void kareCizildi();
  void girdi(Girdi g);
  Future<void> izleyiciKapat();
}

/// Gerçek motor: Rust çekirdeği (flutter_rust_bridge).
class RustMotor implements Motor {
  @override
  bool get baglantiVerilebilir => Platform.isWindows || Platform.isLinux || Platform.isMacOS;
  @override
  bool get dokunmatik => Platform.isAndroid || Platform.isIOS;

  @override
  String cihazAdi() => rust.cihazAdi();

  @override
  Stream<HostOlay> hostBaslat({required String ad, String parola = '', bool upnp = true}) =>
      rust.hostBaslat(ad: ad, parola: parola, upnp: upnp).map((o) => HostOlay(o.tur,
          kod: o.kod,
          parola: o.parola,
          erisim: o.erisim,
          ad: o.ad,
          metin: o.metin,
          kontrol: o.kontrol));

  @override
  Future<void> hostKabul({required bool kontrol}) => rust.hostKabul(kontrol: kontrol);
  @override
  Future<void> hostRed() => rust.hostRed();
  @override
  Future<void> hostKes() => rust.hostKes();
  @override
  Future<void> hostDurdur() => rust.hostDurdur();

  @override
  Stream<IzleyiciOlay> baglan({required String kod, required String parola, required String ad}) =>
      rust.izleyiciBaglan(kod: kod, parola: parola, ad: ad).map((o) => IzleyiciOlay(o.tur,
          ad: o.ad,
          metin: o.metin,
          kontrol: o.kontrol,
          genislik: o.genislik,
          yukseklik: o.yukseklik,
          rgba: o.rgba));

  @override
  void kareCizildi() => rust.kareCizildi();

  @override
  void girdi(Girdi g) {
    rust.izleyiciGirdi(
        g: rust.GirdiOlayi(
            tur: g.tur,
            x: g.x,
            y: g.y,
            ad: g.ad,
            basili: g.basili,
            dx: g.dx,
            dy: g.dy,
            metin: g.metin));
  }

  @override
  Future<void> izleyiciKapat() => rust.izleyiciKapat();
}

/// Rust hata mesajını kullanıcıya gösterilecek hale getirir.
String hataMetni(Object e) {
  var s = e.toString();
  for (final on in ['AnyhowException(', 'Exception: ']) {
    if (s.startsWith(on)) s = s.substring(on.length);
  }
  if (s.endsWith(')') && e.toString().startsWith('AnyhowException(')) {
    s = s.substring(0, s.length - 1);
  }
  // anyhow bağlam zinciri: yalnız ilk satır kullanıcıya.
  return s.split('\n').first.trim();
}
