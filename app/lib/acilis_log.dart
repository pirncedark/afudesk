import 'dart:io';

import 'package:flutter/widgets.dart';

/// Açılış tanılaması: %LOCALAPPDATA%\AfuDesk\acilis.log (yalnız Windows).
/// Tek oturumluktur: her açılışta üzerine yazılır, dosya büyümez.
/// Yalnız olay adları yazılır; kimlik, ad ya da token yazılmaz.
class AcilisLog {
  static bool _ilk = true;

  static void yaz(String olay) {
    final kok = Platform.environment['LOCALAPPDATA'];
    if (!Platform.isWindows || kok == null) return;
    try {
      final f = File('$kok/AfuDesk/acilis.log');
      f.parent.createSync(recursive: true);
      f.writeAsStringSync(
        '${DateTime.now().toIso8601String()} $olay\n',
        mode: _ilk ? FileMode.write : FileMode.append,
        flush: true,
      );
      _ilk = false;
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
