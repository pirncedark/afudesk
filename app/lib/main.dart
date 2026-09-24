import 'package:flutter/material.dart';

import 'motor.dart';
import 'sayfalar/ana.dart';
import 'src/rust/frb_generated.dart';
import 'tema.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await RustLib.init();
  runApp(AfuDeskUygulama(motor: RustMotor()));
}

class AfuDeskUygulama extends StatelessWidget {
  final Motor motor;
  const AfuDeskUygulama({super.key, required this.motor});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'AfuDesk',
      debugShowCheckedModeBanner: false,
      theme: afuTema(),
      home: AnaSayfa(motor: motor),
    );
  }
}
