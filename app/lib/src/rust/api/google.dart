// ignore_for_file: invalid_use_of_internal_member, unused_import, unnecessary_import

import '../frb_generated.dart';
import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart';

Future<String> googleIslem({required String islem, required String veri}) =>
    RustLib.instance.api.crateApiGoogleGoogleIslem(islem: islem, veri: veri);
