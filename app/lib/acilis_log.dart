import 'dart:io';

import 'package:flutter/widgets.dart';

/// Açılış tanılaması: %LOCALAPPDATA%\AfuDesk\acilis.log (yalnız Windows).
/// Yalnız olay adları/özet hata türleri yazılır; kimlik, ad ya da token yazılmaz.
class AcilisLog {
  static File? _dosya(String ad) {
    if (!Platform.isWindows) return null;
    final ozel = Platform.environment['AFUDESK_LOG_DIR'];
    final String? kok;
    if (ozel != null && ozel.isNotEmpty) {
      kok = ozel;
    } else {
      final local = Platform.environment['LOCALAPPDATA'];
      kok = (local == null || local.isEmpty) ? null : '$local/AfuDesk';
    }
    if (kok == null) return null;
    return File('$kok/$ad');
  }

  static void yaz(String olay) {
    final f = _dosya('acilis.log');
    if (f == null) return;
    try {
      f.parent.createSync(recursive: true);
      f.writeAsStringSync(
        '${DateTime.now().toIso8601String()} $olay\n',
        mode: FileMode.append,
        flush: true,
      );
    } catch (_) {}
  }

  static void hataYaz(String kaynak, Object hata, [StackTrace? iz]) {
    yaz('hata $kaynak: ${hata.runtimeType}');
    final f = _dosya('acilis_hata.txt');
    if (f == null) return;
    try {
      f.parent.createSync(recursive: true);
      final izMetni = iz == null ? '' : '\n$iz';
      f.writeAsStringSync(
        '${DateTime.now().toIso8601String()} [$kaynak] $hata$izMetni\n',
        mode: FileMode.append,
        flush: true,
      );
    } catch (_) {}
  }
}

class AcilisGezgini extends NavigatorObserver {
  @override
  void didPush(Route<dynamic> route, Route<dynamic>? previousRoute) =>
      AcilisLog.yaz('push ${route.runtimeType} ${route.settings.name ?? ''}');
  @override
  void didPop(Route<dynamic> route, Route<dynamic>? previousRoute) =>
      AcilisLog.yaz('pop ${route.runtimeType} ${route.settings.name ?? ''}');
  @override
  void didRemove(Route<dynamic> route, Route<dynamic>? previousRoute) =>
      AcilisLog.yaz('remove ${route.runtimeType}');
}
