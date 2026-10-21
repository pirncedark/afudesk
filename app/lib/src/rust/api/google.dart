import '../frb_generated.dart';

Future<String> googleIslem({required String islem, required String veri}) =>
    RustLib.instance.api.crateApiGoogleGoogleIslem(islem: islem, veri: veri);
