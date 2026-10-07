import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'package:crypto/crypto.dart';
import 'package:flutter/services.dart';
import 'package:flutter/foundation.dart';
import 'package:url_launcher/url_launcher.dart' as url_launcher;
import 'guncelleme_yayin.dart';
export 'guncelleme_yayin.dart';

const guncelleKanali = MethodChannel('afudesk/guncelle');

final guncellemeKontrolIstegi = ValueNotifier<int>(0);

class GuncellemeSayfasiAcildi implements Exception {
  const GuncellemeSayfasiAcildi();
}

List<String> windowsPrepareArgumanlari(String script) => [
  '-NoProfile', '-NonInteractive', '-WindowStyle', 'Hidden',
  '-ExecutionPolicy', 'Bypass', '-File', script, '-Prepare',
];

class Guncelleyici {
  Future<Guncelleme?> kontrol() async {
    if (!Platform.isWindows && !Platform.isAndroid) return null;
    final mevcut = await guncelleKanali.invokeMethod<String>('surum');
    if (mevcut == null) return null;
    final client = HttpClient()
      ..connectionTimeout = const Duration(seconds: 15);
    try {
      final r = await _yanit(
        client,
        Uri.parse(
          'https://api.github.com/repos/pirncedark/afudesk/releases/latest',
        ),
      );
      final body = await utf8.decoder
          .bind(r)
          .join()
          .timeout(const Duration(seconds: 20));
      return Guncelleme.yayindan(
        jsonDecode(body) as Map<String, dynamic>,
        mevcut,
        Platform.isAndroid ? 'AfuDesk-android.apk' : 'AfuDesk-windows-x64.zip',
      );
    } finally {
      client.close(force: true);
    }
  }

  Future<HttpClientResponse> _yanit(HttpClient c, Uri u) async {
    final q = await c.getUrl(u).timeout(const Duration(seconds: 20));
    q.headers.set(HttpHeaders.userAgentHeader, 'AfuDesk-updater');
    final r = await q.close().timeout(const Duration(seconds: 20));
    if (r.statusCode != 200) throw const HttpException('İndirme başarısız');
    return r;
  }

  Future<void> kur(Guncelleme g, void Function(double) ilerleme) async {
    final client = HttpClient()
      ..connectionTimeout = const Duration(seconds: 20);
    final Directory klasor;
    if (Platform.isAndroid) {
      klasor = Directory(
        (await guncelleKanali.invokeMethod<String>('klasor'))!,
      );
    } else {
      klasor = await Directory.systemTemp.createTemp('afudesk-update-');
    }
    final dosya = File(
      '${klasor.path}/${Platform.isAndroid ? 'AfuDesk.apk' : 'AfuDesk.zip'}',
    );
    try {
      final h = await _yanit(client, g.ozet);
      final metin = await utf8.decoder
          .bind(h)
          .join()
          .timeout(const Duration(seconds: 20));
      final beklenen = metin.trim().split(RegExp(r'\s+')).first.toLowerCase();
      if (!RegExp(r'^[a-f0-9]{64}$').hasMatch(beklenen)) {
        throw const FormatException('Özet geçersiz');
      }
      final r = await _yanit(client, g.paket);
      final sink = dosya.openWrite();
      var alinan = 0;
      try {
        await for (final parca in r.timeout(const Duration(seconds: 45))) {
          alinan += parca.length;
          if (alinan > g.boyut) {
            throw const FormatException('Paket boyutu geçersiz');
          }
          sink.add(parca);
          ilerleme(alinan / g.boyut);
        }
      } finally {
        await sink.close();
      }
      if (alinan != g.boyut ||
          (await sha256.bind(dosya.openRead()).first).toString() != beklenen) {
        throw const FormatException('Paket doğrulanamadı');
      }
      if (Platform.isAndroid) {
        await guncelleKanali.invokeMethod<void>('kur', dosya.path);
      } else {
        await _windowsKur(dosya, klasor);
      }
    } catch (_) {
      if (await dosya.exists()) await dosya.delete();
      rethrow;
    } finally {
      client.close(force: true);
    }
  }

  Future<void> _windowsKur(File zip, Directory klasor) async {
    final config = File('${klasor.path}/config.json');
    await config.writeAsString(
      jsonEncode({
        'zip': zip.path,
        'target': File(Platform.resolvedExecutable).parent.path,
        'exe': Platform.resolvedExecutable,
        'pid': pid,
      }),
    );
    final script = File('${klasor.path}/kur.ps1');
    await script.writeAsString('\ufeff$windowsKurulum');
    final hazir = await Process.run('powershell.exe',
      windowsPrepareArgumanlari(script.path), runInShell: false);
    if (hazir.exitCode == 3) {
      final acildi = await url_launcher.launchUrl(Uri.parse(
        'https://github.com/pirncedark/afudesk/releases/latest'));
      if (acildi) throw const GuncellemeSayfasiAcildi();
      throw const FileSystemException('Güncelleme sayfası açılamadı');
    }
    if (hazir.exitCode != 0) {
      throw const FileSystemException('Kurulum hazırlanamadı');
    }
    await Process.start('powershell.exe', [
      '-NoProfile',
      '-NonInteractive',
      '-WindowStyle',
      'Hidden',
      '-ExecutionPolicy',
      'Bypass',
      '-File',
      script.path,
    ], mode: ProcessStartMode.detached);
    exit(0);
  }
}

const windowsKurulum = r'''
param([switch]$Prepare)
$ErrorActionPreference = 'Stop'
$c = Get-Content -LiteralPath (Join-Path $PSScriptRoot 'config.json') -Raw | ConvertFrom-Json
$stage = Join-Path $PSScriptRoot 'stage'
function Paket-Kaynagi {
  if (Test-Path -LiteralPath (Join-Path $stage 'afudesk.exe') -PathType Leaf) { return $stage }
  $nested = Join-Path $stage 'AfuDesk'
  if (Test-Path -LiteralPath (Join-Path $nested 'afudesk.exe') -PathType Leaf) { return $nested }
  throw 'Paket eksik'
}
$backup = Join-Path $PSScriptRoot 'backup'
$created = [System.Collections.Generic.List[string]]::new()
if ($Prepare) {
  try {
    Expand-Archive -LiteralPath $c.zip -DestinationPath $stage -Force
    $source = Paket-Kaynagi
    $probe = Join-Path $c.target ('.afu-update-' + [guid]::NewGuid().ToString())
    [IO.File]::WriteAllText($probe, '')
    Remove-Item -LiteralPath $probe
    exit 0
  } catch {
    $errorException = $_.Exception
    while ($null -ne $errorException) {
      if ($errorException -is [System.UnauthorizedAccessException] -or
          $errorException -is [System.Security.SecurityException]) { exit 3 }
      $errorException = $errorException.InnerException
    }
    exit 1
  }
}
try {
  $source = Paket-Kaynagi
  Wait-Process -Id $c.pid -Timeout 90 -ErrorAction SilentlyContinue
  if (Get-Process -Id $c.pid -ErrorAction SilentlyContinue) { throw 'Uygulama kapanmadı' }
  New-Item -ItemType Directory -Path $backup -Force | Out-Null
  $files = Get-ChildItem -LiteralPath $source -Recurse -File
  foreach ($f in $files) {
    $relative = $f.FullName.Substring($source.Length + 1)
    $target = Join-Path $c.target $relative
    if (Test-Path -LiteralPath $target) {
      $save = Join-Path $backup $relative
      New-Item -ItemType Directory -Path (Split-Path $save) -Force | Out-Null
      Copy-Item -LiteralPath $target -Destination $save
    }
  }
  foreach ($f in $files) {
    $target = Join-Path $c.target $f.FullName.Substring($source.Length + 1)
    New-Item -ItemType Directory -Path (Split-Path $target) -Force | Out-Null
    if (-not (Test-Path -LiteralPath $target)) { $created.Add($target) }
    Copy-Item -LiteralPath $f.FullName -Destination $target -Force
  }
} catch {
  foreach ($target in $created) {
    if (Test-Path -LiteralPath $target -PathType Leaf) { Remove-Item -LiteralPath $target -Force }
  }
  if (Test-Path -LiteralPath $backup) {
    Get-ChildItem -LiteralPath $backup -Recurse -File | ForEach-Object {
      Copy-Item -LiteralPath $_.FullName -Destination (Join-Path $c.target $_.FullName.Substring($backup.Length + 1)) -Force
    }
  }
  Add-Type -AssemblyName System.Windows.Forms
  [System.Windows.Forms.MessageBox]::Show('Güncelleme kurulamadı. AfuDesk açılınca yeniden dene.', 'AfuDesk') | Out-Null
}
Start-Process -FilePath $c.exe -WorkingDirectory $c.target
''';
