import 'package:flutter/material.dart';

/// AfuDesk renkleri: koyu lacivert zemin, mavi vurgu (Afu ailesiyle uyumlu).
class Renk {
  static const zemin = Color(0xFF12161D);
  static const yuzey = Color(0xFF1B212B);
  static const yuzey2 = Color(0xFF232B37);
  static const cizgi = Color(0xFF2E3848);
  static const vurgu = Color(0xFF5B9DFF);
  static const basari = Color(0xFF46C98B);
  static const tehlike = Color(0xFFFF6B6B);
  static const soluk = Color(0xFF9AA5B4);
}

ThemeData afuTema() {
  final sema = ColorScheme.fromSeed(
    seedColor: Renk.vurgu,
    brightness: Brightness.dark,
    surface: Renk.yuzey,
  ).copyWith(primary: Renk.vurgu, error: Renk.tehlike);
  return ThemeData(
    useMaterial3: true,
    colorScheme: sema,
    scaffoldBackgroundColor: Renk.zemin,
    appBarTheme: const AppBarTheme(backgroundColor: Renk.zemin, elevation: 0, centerTitle: false),
    cardTheme: CardThemeData(
      color: Renk.yuzey,
      elevation: 0,
      shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(14), side: const BorderSide(color: Renk.cizgi)),
    ),
    inputDecorationTheme: InputDecorationTheme(
      filled: true,
      fillColor: Renk.yuzey2,
      border: OutlineInputBorder(borderRadius: BorderRadius.circular(10), borderSide: BorderSide.none),
    ),
    filledButtonTheme: FilledButtonThemeData(
      style: FilledButton.styleFrom(
        minimumSize: const Size(0, 48),
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(10)),
        textStyle: const TextStyle(fontSize: 15, fontWeight: FontWeight.w600),
      ),
    ),
    outlinedButtonTheme: OutlinedButtonThemeData(
      style: OutlinedButton.styleFrom(
        minimumSize: const Size(0, 48),
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(10)),
      ),
    ),
  );
}
