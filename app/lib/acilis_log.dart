import 'dart:io';

import 'package:flutter/widgets.dart';

/// Açılış tanılaması: %LOCALAPPDATA%\AfuDesk\acilis.log (yalnız Windows).
/// Yalnız olay adları yazılır; kimlik, ad ya da token yazılmaz.
class AcilisLog {
  static File? _dosya;
  static bool _hazir = false;

  static void yaz(String olay) {
    try {
      if (!_hazir) {
        _hazir = true;
        final kok = Platform.environment['LOCALAPPDATA'];
        if (!Platform.isWindows || kok == null) return;
        final klasor = Directory('$kok/AfuDesk')..createSync(recursive: true);
        final f = File('${klasor.path}/acilis.log');
        if (f.existsSync() && f.lengthSync() > 262144) f.writeAsStringSync('');
        _dosya = f;
      }
      _dosya?.writeAsStringSync(
        '${DateTime.now().toIso8601String()} $olay\n',
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
