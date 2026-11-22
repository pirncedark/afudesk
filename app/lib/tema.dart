import 'dart:io' show Platform;
import 'dart:ui' show ImageFilter;

import 'package:flutter/foundation.dart' show kIsWeb;
import 'package:flutter/material.dart';

/// Afu Gold Glass renk sabitleri (mavi-mor yok; açık ve koyu paletler AA uyumlu).
class Renk {
  // Gold Glass altın ailesi (ortak)
  static const altinBirincil = Color(0xFFB89452);
  static const altinAcik = Color(0xFFDCC28A);
  static const altinYumusak = Color(0xFFF2E5C9);

  // Koyu palet (#141518 zemin, rgba(26,28,34,.68) cam, rgba(242,229,201,.14) kenar)
  static const zemin = Color(0xFF141518);
  static const yuzey = Color(0xFF1A1C22);
  static const camKoyu = Color(0xAD1A1C22);
  static const yuzey2 = Color(0xFF22252C);
  static const cizgi = Color(0x24F2E5C9);
  static const kenarKoyuGuclu = Color(0x5CDCC28A);
  static const metinKoyu = Color(0xFFF5F2EB);
  static const vurgu = altinAcik; // #DCC28A — koyuda >= 10:1 kontrast
  static const vurguMetinKoyu = Color(0xFF141518);
  static const basari = Color(0xFF46C98B);
  static const tehlike = Color(0xFFFF6B6B);
  static const soluk = Color(0xFFB9B2A2);

  // Açık palet (#F5F5F7 zemin, rgba(255,255,255,.72) cam, rgba(184,148,82,.24) kenar)
  static const acikZemin = Color(0xFFF5F5F7);
  static const acikYuzey = Color(0xFFFFFFFF);
  static const camAcik = Color(0xB8FFFFFF);
  static const acikYuzey2 = Color(0xFFEAE8E3);
  static const acikCizgi = Color(0x3DB89452);
  static const kenarAcikGuclu = Color(0x6B7A5C24);
  static const metinAcik = Color(0xFF161513);
  static const acikVurgu = Color(0xFF7A5C24); // #F5F5F7 ve #FFFFFF üstünde >= 5.8:1 AA
  static const acikVurguMetin = Color(0xFFFFFFFF);
  static const acikBasari = Color(0xFF166534);
  static const acikTehlike = Color(0xFFB42318);
  static const acikSoluk = Color(0xFF524E46);
}

@immutable
class GoldGlassPalet extends ThemeExtension<GoldGlassPalet> {
  final Color zemin;
  final Color camYuzey;
  final Color katiYuzey;
  final Color yuzey2;
  final Color camKenar;
  final Color kenarGuclu;
  final Color metin;
  final Color soluk;
  final Color vurgu;
  final Color vurguMetin;
  final Color basari;
  final Color tehlike;
  final Color altIsik;
  final Color ortamIsikA;
  final Color ortamIsikB;

  const GoldGlassPalet({
    required this.zemin,
    required this.camYuzey,
    required this.katiYuzey,
    required this.yuzey2,
    required this.camKenar,
    required this.kenarGuclu,
    required this.metin,
    required this.soluk,
    required this.vurgu,
    required this.vurguMetin,
    required this.basari,
    required this.tehlike,
    required this.altIsik,
    required this.ortamIsikA,
    required this.ortamIsikB,
  });

  static const acik = GoldGlassPalet(
    zemin: Renk.acikZemin,
    camYuzey: Renk.camAcik,
    katiYuzey: Renk.acikYuzey,
    yuzey2: Renk.acikYuzey2,
    camKenar: Renk.acikCizgi,
    kenarGuclu: Renk.kenarAcikGuclu,
    metin: Renk.metinAcik,
    soluk: Renk.acikSoluk,
    vurgu: Renk.acikVurgu,
    vurguMetin: Renk.acikVurguMetin,
    basari: Renk.acikBasari,
    tehlike: Renk.acikTehlike,
    altIsik: Color(0x33B89452),
    ortamIsikA: Color(0x30DCC28A),
    ortamIsikB: Color(0x24B89452),
  );

  static const koyu = GoldGlassPalet(
    zemin: Renk.zemin,
    camYuzey: Renk.camKoyu,
    katiYuzey: Renk.yuzey,
    yuzey2: Renk.yuzey2,
    camKenar: Renk.cizgi,
    kenarGuclu: Renk.kenarKoyuGuclu,
    metin: Renk.metinKoyu,
    soluk: Renk.soluk,
    vurgu: Renk.vurgu,
    vurguMetin: Renk.vurguMetinKoyu,
    basari: Renk.basari,
    tehlike: Renk.tehlike,
    altIsik: Color(0x2CDCC28A),
    ortamIsikA: Color(0x22B89452),
    ortamIsikB: Color(0x16F2E5C9),
  );

  static GoldGlassPalet of(BuildContext context) {
    final tema = Theme.of(context);
    return tema.extension<GoldGlassPalet>() ??
        (tema.brightness == Brightness.dark ? koyu : acik);
  }

  @override
  GoldGlassPalet copyWith({
    Color? zemin,
    Color? camYuzey,
    Color? katiYuzey,
    Color? yuzey2,
    Color? camKenar,
    Color? kenarGuclu,
    Color? metin,
    Color? soluk,
    Color? vurgu,
    Color? vurguMetin,
    Color? basari,
    Color? tehlike,
    Color? altIsik,
    Color? ortamIsikA,
    Color? ortamIsikB,
  }) => GoldGlassPalet(
    zemin: zemin ?? this.zemin,
    camYuzey: camYuzey ?? this.camYuzey,
    katiYuzey: katiYuzey ?? this.katiYuzey,
    yuzey2: yuzey2 ?? this.yuzey2,
    camKenar: camKenar ?? this.camKenar,
    kenarGuclu: kenarGuclu ?? this.kenarGuclu,
    metin: metin ?? this.metin,
    soluk: soluk ?? this.soluk,
    vurgu: vurgu ?? this.vurgu,
    vurguMetin: vurguMetin ?? this.vurguMetin,
    basari: basari ?? this.basari,
    tehlike: tehlike ?? this.tehlike,
    altIsik: altIsik ?? this.altIsik,
    ortamIsikA: ortamIsikA ?? this.ortamIsikA,
    ortamIsikB: ortamIsikB ?? this.ortamIsikB,
  );

  @override
  GoldGlassPalet lerp(covariant ThemeExtension<GoldGlassPalet>? other, double t) {
    if (other is! GoldGlassPalet) return this;
    return GoldGlassPalet(
      zemin: Color.lerp(zemin, other.zemin, t)!,
      camYuzey: Color.lerp(camYuzey, other.camYuzey, t)!,
      katiYuzey: Color.lerp(katiYuzey, other.katiYuzey, t)!,
      yuzey2: Color.lerp(yuzey2, other.yuzey2, t)!,
      camKenar: Color.lerp(camKenar, other.camKenar, t)!,
      kenarGuclu: Color.lerp(kenarGuclu, other.kenarGuclu, t)!,
      metin: Color.lerp(metin, other.metin, t)!,
      soluk: Color.lerp(soluk, other.soluk, t)!,
      vurgu: Color.lerp(vurgu, other.vurgu, t)!,
      vurguMetin: Color.lerp(vurguMetin, other.vurguMetin, t)!,
      basari: Color.lerp(basari, other.basari, t)!,
      tehlike: Color.lerp(tehlike, other.tehlike, t)!,
      altIsik: Color.lerp(altIsik, other.altIsik, t)!,
      ortamIsikA: Color.lerp(ortamIsikA, other.ortamIsikA, t)!,
      ortamIsikB: Color.lerp(ortamIsikB, other.ortamIsikB, t)!,
    );
  }
}

/// Cam blur desteği: masaüstü ve web'de açık; Android'de yalnız API 31+ ile açılır.
class GoldGlassYuzey {
  static bool androidBlurDestekli = false;

  static bool get blurEtkin {
    if (kIsWeb) return true;
    try {
      if (Platform.isAndroid) return androidBlurDestekli;
    } catch (_) {
      return false;
    }
    return true;
  }
}

ThemeData afuAcikTema() => _temaKur(Brightness.light, GoldGlassPalet.acik);
ThemeData afuKoyuTema() => _temaKur(Brightness.dark, GoldGlassPalet.koyu);
ThemeData afuTema() => afuKoyuTema();

ThemeData _temaKur(Brightness parlaklik, GoldGlassPalet p) {
  final koyu = parlaklik == Brightness.dark;
  final sema = ColorScheme.fromSeed(
    seedColor: Renk.altinBirincil,
    brightness: parlaklik,
    surface: p.katiYuzey,
  ).copyWith(
    primary: p.vurgu,
    onPrimary: p.vurguMetin,
    secondary: Renk.altinBirincil,
    onSecondary: koyu ? Renk.vurguMetinKoyu : Colors.white,
    primaryContainer: koyu ? const Color(0x30B89452) : Renk.altinYumusak,
    onPrimaryContainer: p.metin,
    surface: p.katiYuzey,
    onSurface: p.metin,
    onSurfaceVariant: p.soluk,
    outline: p.kenarGuclu,
    outlineVariant: p.camKenar,
    error: p.tehlike,
    onError: Colors.white,
  );
  return ThemeData(
    useMaterial3: true,
    brightness: parlaklik,
    colorScheme: sema,
    extensions: [p],
    scaffoldBackgroundColor: p.zemin,
    appBarTheme: AppBarTheme(
      backgroundColor: p.zemin,
      foregroundColor: p.metin,
      elevation: 0,
      scrolledUnderElevation: 0,
      centerTitle: false,
    ),
    cardTheme: CardThemeData(
      color: p.katiYuzey,
      elevation: 0,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(16),
        side: BorderSide(color: p.camKenar),
      ),
    ),
    dialogTheme: DialogThemeData(
      backgroundColor: p.katiYuzey,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(20),
        side: BorderSide(color: p.camKenar),
      ),
    ),
    dividerTheme: DividerThemeData(color: p.camKenar, thickness: 1),
    inputDecorationTheme: InputDecorationTheme(
      filled: true,
      fillColor: p.yuzey2,
      border: OutlineInputBorder(
        borderRadius: BorderRadius.circular(12),
        borderSide: BorderSide(color: p.camKenar),
      ),
      enabledBorder: OutlineInputBorder(
        borderRadius: BorderRadius.circular(12),
        borderSide: BorderSide(color: p.camKenar),
      ),
      focusedBorder: OutlineInputBorder(
        borderRadius: BorderRadius.circular(12),
        borderSide: BorderSide(color: p.vurgu, width: 1.5),
      ),
    ),
    filledButtonTheme: FilledButtonThemeData(
      style: FilledButton.styleFrom(
        backgroundColor: p.vurgu,
        foregroundColor: p.vurguMetin,
        minimumSize: const Size(48, 48),
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
        textStyle: const TextStyle(fontSize: 15, fontWeight: FontWeight.w600),
      ),
    ),
    outlinedButtonTheme: OutlinedButtonThemeData(
      style: OutlinedButton.styleFrom(
        foregroundColor: p.metin,
        side: BorderSide(color: p.kenarGuclu),
        minimumSize: const Size(48, 48),
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
      ),
    ),
    textButtonTheme: TextButtonThemeData(
      style: TextButton.styleFrom(
        foregroundColor: p.vurgu,
        minimumSize: const Size(48, 48),
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
      ),
    ),
    iconButtonTheme: IconButtonThemeData(
      style: IconButton.styleFrom(minimumSize: const Size(48, 48)),
    ),
    chipTheme: ChipThemeData(
      backgroundColor: p.yuzey2,
      side: BorderSide(color: p.camKenar),
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
    ),
    switchTheme: SwitchThemeData(
      thumbColor: WidgetStateProperty.resolveWith(
        (s) => s.contains(WidgetState.selected) ? p.vurguMetin : p.soluk,
      ),
      trackColor: WidgetStateProperty.resolveWith(
        (s) => s.contains(WidgetState.selected) ? p.vurgu : p.yuzey2,
      ),
    ),
    checkboxTheme: CheckboxThemeData(
      fillColor: WidgetStateProperty.resolveWith(
        (s) => s.contains(WidgetState.selected) ? p.vurgu : null,
      ),
      checkColor: WidgetStateProperty.all(p.vurguMetin),
    ),
  );
}

/// Gold Glass ortam ışıklı sayfa zemini (mavi-mor yok; sıcak altın radial ışık).
class GoldGlassZemin extends StatelessWidget {
  final Widget child;
  const GoldGlassZemin({super.key, required this.child});

  @override
  Widget build(BuildContext context) {
    final p = GoldGlassPalet.of(context);
    return DecoratedBox(
      decoration: BoxDecoration(
        color: p.zemin,
        gradient: RadialGradient(
          center: const Alignment(-0.85, -0.85),
          radius: 1.15,
          colors: [p.ortamIsikA, p.zemin],
        ),
      ),
      child: DecoratedBox(
        decoration: BoxDecoration(
          gradient: RadialGradient(
            center: const Alignment(0.9, 0.75),
            radius: 1.05,
            colors: [p.ortamIsikB, Colors.transparent],
          ),
        ),
        child: Material(type: MaterialType.transparency, child: child),
      ),
    );
  }
}

/// Gold Glass cam kart: destekleyen yerde backdrop blur, Android < 31'de yarı saydam yüzey + altın vurgu.
class GoldGlassKart extends StatelessWidget {
  final Widget child;
  final double yaricap;
  final EdgeInsetsGeometry? margin;
  final VoidCallback? onTap;
  final Key? tiklamaAnahtari;

  const GoldGlassKart({
    super.key,
    required this.child,
    this.yaricap = 16,
    this.margin,
    this.onTap,
    this.tiklamaAnahtari,
  });

  @override
  Widget build(BuildContext context) {
    final p = GoldGlassPalet.of(context);
    final blur = GoldGlassYuzey.blurEtkin;
    final yuzey = blur ? p.camYuzey : p.camYuzey.withValues(alpha: 0.92);
    final ic = DecoratedBox(
      decoration: BoxDecoration(
        color: yuzey,
        borderRadius: BorderRadius.circular(yaricap),
        border: Border.all(color: p.camKenar),
        gradient: RadialGradient(
          center: const Alignment(0, 1.05),
          radius: 1.15,
          colors: [p.altIsik, Colors.transparent],
        ),
      ),
      child: Material(
        type: MaterialType.transparency,
        child: (onTap == null && tiklamaAnahtari == null)
            ? child
            : InkWell(
                key: tiklamaAnahtari,
                borderRadius: BorderRadius.circular(yaricap),
                onTap: onTap,
                child: child,
              ),
      ),
    );

    final kart = ClipRRect(
      borderRadius: BorderRadius.circular(yaricap),
      child: blur
          ? BackdropFilter(
              filter: ImageFilter.blur(sigmaX: 14, sigmaY: 14),
              child: ic,
            )
          : ic,
    );

    return margin == null ? kart : Padding(padding: margin!, child: kart);
  }
}

/// prefers-reduced-motion desteği: animasyonlar kapalıysa Duration.zero döner.
Duration afuAnimasyonSuresi(BuildContext context, [Duration normal = const Duration(milliseconds: 220)]) =>
    MediaQuery.disableAnimationsOf(context) ? Duration.zero : normal;
