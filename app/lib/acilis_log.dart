import 'dart:io';

import 'package:flutter/widgets.dart';

/// Açılış tanılaması: %LOCALAPPDATA%\AfuDesk\acilis.log ve %TEMP%\afudesk_acilis.log.
/// Yalnız olay adları yazılır; kimlik, ad ya da token yazılmaz.
class AcilisLog {
  static void yaz(String olay) {
    final satir = '${DateTime.now().toIso8601String()} $olay\n';
    final hedefler = <String>[
      '${Platform.environment['LOCALAPPDATA']}/AfuDesk/acilis.log',
      '${Platform.environment['TEMP']}/afudesk_acilis.log',
    ];
    for (final yol in hedefler) {
      try {
        final f = File(yol);
        f.parent.createSync(recursive: true);
        f.writeAsStringSync(satir, mode: FileMode.append, flush: true);
      } catch (e) {
        try {
          File('${Platform.environment['TEMP']}/afudesk_acilis_hata.txt')
              .writeAsStringSync('$yol -> $e\n', mode: FileMode.append);
        } catch (_) {}
      }
    }
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
