import 'dart:math' as math;

import 'package:flutter/material.dart';

import '../oyun_kolu.dart';

/// Yatay oturum ekranında parmaklarla aynı anda kullanılabilen yarı saydam kol.
class OyunKoluKatmani extends StatefulWidget {
  final ValueChanged<OyunKoluDurumu> onChanged;
  const OyunKoluKatmani({super.key, required this.onChanged});

  @override
  State<OyunKoluKatmani> createState() => _OyunKoluKatmaniState();
}

class _OyunKoluKatmaniState extends State<OyunKoluKatmani> {
  final Set<String> _basili = {};
  double _lx = 0, _ly = 0, _rx = 0, _ry = 0;

  void _bildir() => widget.onChanged(OyunKoluDurumu(
        dugmeler: OyunKoluDurumu.dugmeMaskesi(_basili),
        solX: OyunKoluDurumu.eksen(_lx), solY: OyunKoluDurumu.eksen(_ly),
        sagX: OyunKoluDurumu.eksen(_rx), sagY: OyunKoluDurumu.eksen(_ry),
        solTetik: OyunKoluDurumu.tetik(_basili.contains('lt') ? 1 : 0),
        sagTetik: OyunKoluDurumu.tetik(_basili.contains('rt') ? 1 : 0),
      ));

  void _tus(String name, bool down) {
    if (down) { _basili.add(name); } else { _basili.remove(name); }
    _bildir();
  }

  Widget _tusDugmesi(String name, String label, {double size = 58, IconData? icon}) =>
      _BasiliTus(key: ValueKey('kol_$name'), name: name, label: label, size: size, icon: icon, onChanged: _tus);

  Widget _cubuk(String ad, ValueChanged<Offset> onMove) => _AnalogCubuk(key: ValueKey('kol_cubuk_$ad'), ad: ad, onMove: onMove);

  @override
  Widget build(BuildContext context) => IgnorePointer(
        ignoring: false,
        child: SafeArea(
          child: Stack(children: [
            Positioned(left: 12, bottom: 10, child: _cubuk('sol', (p) { _lx = p.dx; _ly = p.dy; _bildir(); })),
            Positioned(right: 12, bottom: 10, child: _cubuk('sag', (p) { _rx = p.dx; _ry = p.dy; _bildir(); })),
            Positioned(left: 14, top: 6, child: Row(children: [_tusDugmesi('lb', 'LB'), const SizedBox(width: 6), _tusDugmesi('lt', 'LT', size: 58)])),
            Positioned(right: 14, top: 6, child: Row(children: [_tusDugmesi('rt', 'RT'), const SizedBox(width: 6), _tusDugmesi('rb', 'RB')])),
            Positioned(left: 14, top: 72, child: SizedBox(width: 156, height: 156, child: Stack(children: [
              Positioned(top: 0, left: 50, child: _tusDugmesi('up', '▲')),
              Positioned(left: 0, top: 50, child: _tusDugmesi('left', '◀')),
              Positioned(right: 0, top: 50, child: _tusDugmesi('right', '▶')),
              Positioned(bottom: 0, left: 50, child: _tusDugmesi('down', '▼')),
            ]))),
            Positioned(right: 14, top: 72, child: SizedBox(width: 156, height: 156, child: Stack(children: [
              Positioned(top: 0, left: 50, child: _tusDugmesi('y', 'Y')),
              Positioned(left: 0, top: 50, child: _tusDugmesi('x', 'X')),
              Positioned(right: 0, top: 50, child: _tusDugmesi('b', 'B')),
              Positioned(bottom: 0, left: 50, child: _tusDugmesi('a', 'A')),
            ]))),
            Positioned(left: 0, right: 0, bottom: 12, child: Center(child: Row(mainAxisSize: MainAxisSize.min, children: [
              _tusDugmesi('back', 'BACK', size: 64), const SizedBox(width: 8), _tusDugmesi('start', 'START', size: 64),
            ]))),
          ]),
        ),
      );
}

class _BasiliTus extends StatelessWidget {
  final String name, label;
  final double size;
  final IconData? icon;
  final void Function(String, bool) onChanged;
  const _BasiliTus({super.key, required this.name, required this.label, required this.size, required this.onChanged, this.icon});
  @override
  Widget build(BuildContext context) => Listener(
        key: Key('oyun_kolu_tus_$name'), behavior: HitTestBehavior.opaque,
        onPointerDown: (_) => onChanged(name, true),
        onPointerUp: (_) => onChanged(name, false),
        onPointerCancel: (_) => onChanged(name, false),
        child: Container(
          width: size, height: size, alignment: Alignment.center,
          decoration: BoxDecoration(color: Colors.black.withValues(alpha: 0.48), shape: BoxShape.circle, border: Border.all(color: Colors.white.withValues(alpha: 0.75), width: 2)),
          child: icon == null ? Text(label, style: const TextStyle(color: Colors.white, fontWeight: FontWeight.bold, fontSize: 15)) : Icon(icon, size: 22, color: Colors.white),
        ),
      );
}

class _AnalogCubuk extends StatefulWidget {
  final String ad;
  final ValueChanged<Offset> onMove;
  const _AnalogCubuk({super.key, required this.ad, required this.onMove});
  @override
  State<_AnalogCubuk> createState() => _AnalogCubukState();
}

class _AnalogCubukState extends State<_AnalogCubuk> {
  Offset _value = Offset.zero;
  int? _pointer;
  void _move(PointerEvent e) {
    final box = context.findRenderObject()! as RenderBox;
    final p = box.globalToLocal(e.position) - Offset(box.size.width / 2, box.size.height / 2);
    final radius = box.size.shortestSide * 0.35;
    final d = p.distance;
    _value = d > radius ? p / d : p / radius;
    widget.onMove(Offset(_value.dx.clamp(-1.0, 1.0), -_value.dy.clamp(-1.0, 1.0)));
    setState(() {});
  }
  @override
  Widget build(BuildContext context) => Listener(
        key: Key('oyun_kolu_analog_${widget.ad}'), behavior: HitTestBehavior.opaque,
        onPointerDown: (e) { _pointer = e.pointer; _move(e); },
        onPointerMove: (e) { if (e.pointer == _pointer) _move(e); },
        onPointerUp: (e) { if (e.pointer == _pointer) { _pointer = null; _value = Offset.zero; widget.onMove(Offset.zero); setState(() {}); } },
        onPointerCancel: (_) { _pointer = null; _value = Offset.zero; widget.onMove(Offset.zero); setState(() {}); },
        child: SizedBox(width: 126, height: 126, child: DecoratedBox(
          decoration: BoxDecoration(shape: BoxShape.circle, color: Colors.black.withValues(alpha: 0.38), border: Border.all(color: Colors.white.withValues(alpha: 0.65), width: 2)),
          child: Center(child: Transform.translate(offset: Offset(_value.dx * 34, _value.dy * 34), child: Container(width: 50, height: 50, decoration: BoxDecoration(shape: BoxShape.circle, color: Colors.white.withValues(alpha: 0.58), border: Border.all(color: Colors.white, width: 2))))),
        )),
      );
}
