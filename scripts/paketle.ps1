# AfuDesk Windows paketini üretir: derler, zip'ler, SHA-256 yazar.
# Kullanım: powershell -File scripts/paketle.ps1
$ErrorActionPreference = 'Stop'
$kok = Split-Path $PSScriptRoot -Parent
$flutter = Join-Path $env:USERPROFILE 'tools\f344\flutter\bin\flutter.bat'
if (-not (Test-Path $flutter)) { $flutter = 'flutter' }

Push-Location (Join-Path $kok 'app')
& $flutter build windows --release
if ($LASTEXITCODE -ne 0) { throw 'Derleme başarısız' }
Pop-Location

$kaynak = Join-Path $kok 'app\build\windows\x64\runner\Release'
$cikti = Join-Path $kok 'dist'
New-Item -ItemType Directory -Force $cikti | Out-Null
$zip = Join-Path $cikti 'AfuDesk-windows-x64.zip'
if (Test-Path $zip) { Remove-Item $zip }
$gecici = Join-Path $cikti 'AfuDesk'
if (Test-Path $gecici) { Remove-Item -Recurse -Force $gecici }
Copy-Item -Recurse $kaynak $gecici
Compress-Archive -Path $gecici -DestinationPath $zip
Remove-Item -Recurse -Force $gecici
$sha = (Get-FileHash $zip -Algorithm SHA256).Hash.ToLower()
"$sha  AfuDesk-windows-x64.zip" | Set-Content -Encoding ascii (Join-Path $cikti 'AfuDesk-windows-x64.zip.sha256')
"Paket: $zip"
"SHA-256: $sha"
