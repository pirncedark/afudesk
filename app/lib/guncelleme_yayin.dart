bool yeniSurum(String aday, String mevcut) {
  List<int>? sayilar(String s) {
    final m = RegExp(r'^v?(\d+)\.(\d+)\.(\d+)(?:\+\d+)?$').firstMatch(s);
    return m == null ? null : [for (var i = 1; i <= 3; i++) int.parse(m[i]!)];
  }

  final a = sayilar(aday), b = sayilar(mevcut);
  if (a == null || b == null) return false;
  for (var i = 0; i < 3; i++) {
    if (a[i] != b[i]) return a[i] > b[i];
  }
  return false;
}

class Guncelleme {
  final String surum;
  final Uri paket, ozet;
  final int boyut;
  const Guncelleme(this.surum, this.paket, this.ozet, this.boyut);

  static Guncelleme? yayindan(
    Map<String, dynamic> json,
    String mevcut,
    String dosya,
  ) {
    if (json['draft'] != false ||
        json['prerelease'] != false ||
        !yeniSurum(json['tag_name'] as String? ?? '', mevcut)) {
      return null;
    }
    final assets = (json['assets'] as List).cast<Map<String, dynamic>>();
    Map<String, dynamic>? bul(String ad) {
      for (final a in assets) {
        if (a['name'] == ad) return a;
      }
      return null;
    }

    final p = bul(dosya), h = bul('$dosya.sha256');
    if (p == null || h == null) return null;
    Uri adres(Map<String, dynamic> a) {
      final u = Uri.parse(a['browser_download_url'] as String);
      if (u.scheme != 'https' ||
          u.host != 'github.com' ||
          !u.path.startsWith('/pirncedark/afudesk/releases/download/')) {
        throw const FormatException('Yayın adresi geçersiz');
      }
      return u;
    }

    final boyut = p['size'] as int;
    if (boyut <= 0 || boyut > 1024 * 1024 * 1024) return null;
    return Guncelleme(json['tag_name'] as String, adres(p), adres(h), boyut);
  }
}
