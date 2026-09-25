// Arayüz ile Rust çekirdeği arasındaki sözleşme. Testler SahteMotor kullanır.
import 'dart:async';
import 'dart:io' show Platform;
import 'dart:typed_data';

import 'package:file_selector/file_selector.dart' as fs;

import 'src/rust/api/afudesk.dart' as rust;
import 'oyun_kolu.dart';

/// Host tarafı olayı.
class HostOlay {
  final String tur, kod, parola, erisim, ad, metin, yol;
  final bool kontrol, pano, dosya, oyunKolu;
  const HostOlay(this.tur, {this.kod = '', this.parola = '', this.erisim = '', this.ad = '', this.metin = '',
    this.kontrol = false, this.pano = false, this.dosya = false, this.yol = '', this.oyunKolu = false});
}

class IzleyiciOlay {
  final String tur; // bekliyor | kabul | kare | istatistik | koptu | hata | pano | dosya
  final int rttMs;
  final int gecikmeMs;
  final int fps;
  final String ad;
  final String metin;
  final bool kontrol;
  final bool pano;
  final int genislik;
  final int yukseklik;
  final Uint8List rgba;
  /// kabul: karşı taraf dosya almaya izin verdi mi.
  final bool dosya;
  final bool oyunKolu;
  /// dosya: ilerleme (bayt). `bitti` ise `metin` boşsa başarılı, doluysa hata.
  final int gonderilen;
  final int toplam;
  final bool bitti;
  final int slot, buyuk, kucuk;
  IzleyiciOlay(this.tur,
      {this.rttMs = 0,
      this.gecikmeMs = 0,
      this.fps = 0,
      this.ad = '',
      this.metin = '',
      this.kontrol = false,
      this.pano = false,
      this.genislik = 0,
      this.yukseklik = 0,
      this.dosya = false,
      this.oyunKolu = false,
      this.gonderilen = 0,
      this.toplam = 0,
      this.bitti = false,
      this.slot = 0,
      this.buyuk = 0,
      this.kucuk = 0,
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
Future<void> hostKabul({required bool kontrol, bool pano = false, bool dosya = false, bool oyunKolu = false});
  Future<void> hostRed();
  Future<void> hostKes();
  Future<void> hostDurdur();
  Stream<IzleyiciOlay> baglan({required String kod, required String parola, required String ad});
  void kareCizildi();
  void girdi(Girdi g);
  void izleyiciKol(OyunKoluDurumu durum);
  /// Gönderilecek dosyayı kullanıcıya seçtirir; vazgeçilirse null.
  Future<String?> dosyaSec();
  /// Dosyayı karşı tarafa gönderir; ilerleme `baglan` akışına 'dosya' olayı olarak gelir.
  Future<void> dosyaGonder(String yol);
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
          kontrol: o.kontrol,
          pano: o.pano,
          dosya: o.dosya,
          yol: o.yol,
          oyunKolu: o.oyunKolu));

  @override
  Future<void> hostKabul({required bool kontrol, bool pano = false, bool dosya = false, bool oyunKolu = false}) =>
      rust.hostKabul(kontrol: kontrol, pano: pano, dosya: dosya, oyunKolu: oyunKolu);
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
          pano: o.pano,
          genislik: o.genislik,
          yukseklik: o.yukseklik,
          rttMs: o.rttMs,
          fps: o.fps,
          gecikmeMs: o.gecikmeMs,
          dosya: o.dosya,
          oyunKolu: o.oyunKolu,
          gonderilen: o.gonderilen.toInt(),
          toplam: o.toplam.toInt(),
          bitti: o.bitti,
          slot: o.slot,
          buyuk: o.buyuk,
          kucuk: o.kucuk,
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
  void izleyiciKol(OyunKoluDurumu d) => rust.izleyiciKol(slot: 0, dugmeler: d.dugmeler, solX: d.solX, solY: d.solY, sagX: d.sagX, sagY: d.sagY, solTetik: d.solTetik, sagTetik: d.sagTetik);

  @override
  Future<String?> dosyaSec() async => (await fs.openFile(confirmButtonText: 'Gönder'))?.path;

  @override
  Future<void> dosyaGonder(String yol) => rust.izleyiciDosyaGonder(yol: yol);

  @override
  Future<void> izleyiciKapat() => rust.izleyiciKapat();
}

/// Yolun son parçası (Windows ve POSIX ayırıcıları).
String dosyaAdi(String yol) => yol.split(RegExp(r'[\\/]')).lastWhere((p) => p.isNotEmpty, orElse: () => yol);

/// İnsan okunur boyut: 512 B, 1,5 KB, 3,0 MB, 1,2 GB.
String boyutMetni(int bayt) {
  if (bayt < 1024) return '$bayt B';
  const birimler = ['KB', 'MB', 'GB', 'TB'];
  var d = bayt / 1024;
  var i = 0;
  while (d >= 1024 && i < birimler.length - 1) {
    d /= 1024;
    i++;
  }
  return '${d.toStringAsFixed(1).replaceAll('.', ',')} ${birimler[i]}';
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
