import 'dart:async';

import 'package:afudesk/motor.dart';
import 'package:afudesk/oyun_kolu.dart';

/// Testler için motor: olayları testin kendisi basar, çağrıları kaydeder.
class SahteMotor implements Motor {
  final host = StreamController<HostOlay>.broadcast(sync: true);
  final izleyici = StreamController<IzleyiciOlay>.broadcast(sync: true);
  final cagrilar = <String>[];
  final girdiler = <Girdi>[];
  final oyunKoluDurumlari = <OyunKoluDurumu>[];
  Object? baglanHatasi;
  String? sonKod, sonParola;
  bool? sonPano;
  bool? sonDosyaIzni;
  bool? sonOyunKolu;
  String? secilecekDosya;
  final gonderilenDosyalar = <String>[];
  int kareOnayi = 0;
  @override
  bool baglantiVerilebilir = true;
  @override
  bool dokunmatik = false;

  final kayitli = <KayitliCihaz>[];
  final guvenilen = <KayitliCihaz>[];

  @override
  String cihazAdi() => 'TEST-PC';

  @override
  void arkaPlanBaslat() {
    cagrilar.add('arkaPlan');
    _durumuIzle();
  }
  @override
  Stream<HostOlay> get hostOlaylari => host.stream;
  @override
  Stream<IzleyiciOlay> kayitliBaglan({required String kimlik, required String ad}) {
    cagrilar.add('kayitliBaglan:$kimlik');
    return izleyici.stream;
  }
  @override
  List<KayitliCihaz> kayitliCihazlar() => List.of(kayitli);
  @override
  void kayitliUnut(String kimlik) {
    cagrilar.add('unut:$kimlik');
    kayitli.removeWhere((k) => k.kimlik == kimlik);
  }
  @override
  List<KayitliCihaz> guvenilenCihazlar() => List.of(guvenilen);
  @override
  void guvenilenKaldir(String kimlik) {
    cagrilar.add('kaldir:$kimlik');
    guvenilen.removeWhere((k) => k.kimlik == kimlik);
  }

  HostOlay? _sonHazir, _bagli;
  bool _izleniyor = false;

  /// Gerçek motor gibi: sayfa sonradan açılırsa son durumu (bağlı oturum / geçerli kod) görür.
  void _durumuIzle() {
    if (_izleniyor) return;
    _izleniyor = true;
    host.stream.listen((o) {
      if (o.tur == 'hazir') _sonHazir = o;
      if (o.tur == 'baglandi') _bagli = o;
      if (o.tur == 'koptu') _bagli = null;
    }, onError: (Object _) {});
  }

  @override
  Stream<HostOlay> hostBaslat({required String ad, String parola = '', bool upnp = true}) async* {
    cagrilar.add('hostBaslat:$ad');
    final son = _bagli ?? _sonHazir;
    if (son != null) yield son;
    yield* host.stream;
  }

  @override
Future<void> hostKabul({required bool kontrol, bool pano = false, bool dosya = false, bool oyunKolu = false}) async {
    sonPano = pano;
    sonDosyaIzni = dosya;
    sonOyunKolu = oyunKolu;
    cagrilar.add('kabul:$kontrol:$oyunKolu');
  }
  @override
  Future<void> hostRed() async => cagrilar.add('red');
  @override
  Future<void> hostKes() async => cagrilar.add('kes');
  @override
  Future<void> hostDurdur() async => cagrilar.add('durdur');

  @override
  Stream<IzleyiciOlay> baglan({required String kod, required String parola, required String ad}) {
    sonKod = kod;
    sonParola = parola;
    cagrilar.add('baglan');
    // Gerçek motor gibi: hata akışa 'hata' olayı olarak gelir.
    if (baglanHatasi != null) return Stream.value(IzleyiciOlay('hata', metin: hataMetni(baglanHatasi!)));
    return izleyici.stream;
  }

  @override
  void kareCizildi() => kareOnayi++;
  @override
  void girdi(Girdi g) => girdiler.add(g);
  @override
  void izleyiciKol(OyunKoluDurumu durum) => oyunKoluDurumlari.add(durum);
  @override
  Future<String?> dosyaSec() async => secilecekDosya;
  @override
  Future<void> dosyaGonder(String yol) async => gonderilenDosyalar.add(yol);
  @override
  Future<void> izleyiciKapat() async => cagrilar.add('izleyiciKapat');
}
