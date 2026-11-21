$ErrorActionPreference = 'Stop'

$betik = Join-Path $PSScriptRoot 'flutter_surum_kontrol.ps1'

$shellYolu = $null
$pwshKomut = Get-Command pwsh -ErrorAction SilentlyContinue
if ($pwshKomut) {
    $shellYolu = $pwshKomut.Source
} else {
    $pshomeAdaylari = @(
        (Join-Path $PSHOME 'pwsh.exe'),
        (Join-Path $PSHOME 'pwsh')
    )
    foreach ($aday in $pshomeAdaylari) {
        if (Test-Path -LiteralPath $aday) {
            $shellYolu = $aday
            break
        }
    }
    if (-not $shellYolu -and $env:LOCALAPPDATA) {
        $winAppsPwsh = Join-Path $env:LOCALAPPDATA 'Microsoft\WindowsApps\pwsh.exe'
        if (Test-Path -LiteralPath $winAppsPwsh) {
            $shellYolu = $winAppsPwsh
        }
    }
}

if ($shellYolu) {
    Write-Output "Shell: $shellYolu"
} else {
    $shellYolu = (Get-Command powershell -ErrorAction Stop).Source
    Write-Output "Shell: $shellYolu (pwsh bulunamadi, yedek)"
}

$geciciKok = Join-Path ([System.IO.Path]::GetTempPath()) ('flutter_surum_kontrol_test_' + [System.Guid]::NewGuid().ToString('N'))
$null = New-Item -ItemType Directory -Path $geciciKok -Force

$gecen = 0
$basarisiz = 0

try {
    $durumTutarsiz = Join-Path $geciciKok 'tutarsiz'
    $null = New-Item -ItemType Directory -Path $durumTutarsiz -Force
    $ciTutarsiz = @(
        'jobs:',
        '  test:',
        '    steps:',
        '      - uses: subosito/flutter-action@v2',
        '        with:',
        '          flutter-version: 3.44.9',
        '          channel: stable'
    ) -join "`n"
    $relTutarsiz = @(
        'jobs:',
        '  windows:',
        '    steps:',
        '      - uses: subosito/flutter-action@v2',
        '        with:',
        '          flutter-version: 3.47.5',
        '          channel: stable'
    ) -join "`n"
    [System.IO.File]::WriteAllText((Join-Path $durumTutarsiz 'ci.yml'), $ciTutarsiz, [System.Text.UTF8Encoding]::new($false))
    [System.IO.File]::WriteAllText((Join-Path $durumTutarsiz 'release.yml'), $relTutarsiz, [System.Text.UTF8Encoding]::new($false))

    $durumTutarli = Join-Path $geciciKok 'tutarli'
    $null = New-Item -ItemType Directory -Path $durumTutarli -Force
    $ciTutarli = @(
        'jobs:',
        '  test:',
        '    steps:',
        '      - uses: subosito/flutter-action@v2',
        '        with:',
        '          flutter-version: 3.47.5',
        '          channel: stable'
    ) -join "`n"
    $relTutarli = @(
        'jobs:',
        '  windows:',
        '    steps:',
        '      - uses: subosito/flutter-action@v2',
        '        with:',
        '          flutter-version: 3.47.5',
        '          channel: stable'
    ) -join "`n"
    [System.IO.File]::WriteAllText((Join-Path $durumTutarli 'ci.yml'), $ciTutarli, [System.Text.UTF8Encoding]::new($false))
    [System.IO.File]::WriteAllText((Join-Path $durumTutarli 'release.yml'), $relTutarli, [System.Text.UTF8Encoding]::new($false))

    $durumPubUyumsuz = Join-Path $geciciKok 'pubspec_uyumsuz'
    $wfPubUyumsuz = Join-Path $durumPubUyumsuz 'workflows'
    $null = New-Item -ItemType Directory -Path $wfPubUyumsuz -Force
    [System.IO.File]::WriteAllText((Join-Path $wfPubUyumsuz 'ci.yml'), $ciTutarsiz, [System.Text.UTF8Encoding]::new($false))
    $pubspecUyumsuz = Join-Path $durumPubUyumsuz 'pubspec.yaml'
    $pubspecIcerik = @(
        'name: sahte_uygulama',
        'environment:',
        '  sdk: ^3.12.0',
        "  flutter: '>=3.47.5'"
    ) -join "`n"
    [System.IO.File]::WriteAllText($pubspecUyumsuz, $pubspecIcerik, [System.Text.UTF8Encoding]::new($false))

    $durumPubUyumlu = Join-Path $geciciKok 'pubspec_uyumlu'
    $wfPubUyumlu = Join-Path $durumPubUyumlu 'workflows'
    $null = New-Item -ItemType Directory -Path $wfPubUyumlu -Force
    [System.IO.File]::WriteAllText((Join-Path $wfPubUyumlu 'ci.yml'), $ciTutarli, [System.Text.UTF8Encoding]::new($false))
    $pubspecUyumlu = Join-Path $durumPubUyumlu 'pubspec.yaml'
    [System.IO.File]::WriteAllText($pubspecUyumlu, $pubspecIcerik, [System.Text.UTF8Encoding]::new($false))

    $durumlar = @(
        @{ Ad = 'tutarsiz';        WorkflowsDir = $durumTutarsiz; PubspecPath = '';              BeklenenKod = 1 },
        @{ Ad = 'tutarli';         WorkflowsDir = $durumTutarli;  PubspecPath = '';              BeklenenKod = 0 },
        @{ Ad = 'pubspec_uyumsuz'; WorkflowsDir = $wfPubUyumsuz;  PubspecPath = $pubspecUyumsuz; BeklenenKod = 1 },
        @{ Ad = 'pubspec_uyumlu';  WorkflowsDir = $wfPubUyumlu;   PubspecPath = $pubspecUyumlu;  BeklenenKod = 0 }
    )

    foreach ($d in $durumlar) {
        $psi = New-Object System.Diagnostics.ProcessStartInfo
        $psi.FileName = $shellYolu
        $argList = @('-NoProfile')
        if ($env:OS -eq 'Windows_NT') {
            $argList += @('-ExecutionPolicy', 'Bypass')
        }
        $argList += @('-File', ('"' + $betik + '"'), '-WorkflowsDir', ('"' + $d.WorkflowsDir + '"'))
        if (-not [string]::IsNullOrEmpty($d.PubspecPath)) {
            $argList += @('-PubspecPath', ('"' + $d.PubspecPath + '"'))
        }
        $psi.Arguments = $argList -join ' '
        $psi.UseShellExecute = $false
        $psi.RedirectStandardOutput = $true
        $psi.RedirectStandardError = $true
        $psi.CreateNoWindow = $true

        $proc = [System.Diagnostics.Process]::Start($psi)
        $stdoutTask = $proc.StandardOutput.ReadToEndAsync()
        $stderrTask = $proc.StandardError.ReadToEndAsync()
        $proc.WaitForExit()
        $stdout = $stdoutTask.GetAwaiter().GetResult()
        $stderr = $stderrTask.GetAwaiter().GetResult()
        $kod = $proc.ExitCode
        $proc.Dispose()

        $hataMesajiUygun = $true
        if ($d.BeklenenKod -eq 1) {
            $hataMesajiUygun = ($stderr -match 'HATA:')
        }

        if (($kod -eq $d.BeklenenKod) -and $hataMesajiUygun) {
            Write-Output "[PASS] $($d.Ad): beklenen=$($d.BeklenenKod) gercek=$kod"
            $gecen++
        } else {
            $detay = @()
            if ($kod -ne $d.BeklenenKod) { $detay += 'kod uyusmadi' }
            if (-not $hataMesajiUygun) { $detay += 'HATA mesaji eksik' }
            Write-Output "[FAIL] $($d.Ad): beklenen=$($d.BeklenenKod) gercek=$kod ($($detay -join ', '))"
            $ilkHataSatiri = ($stderr -split "`r?`n" | Where-Object { -not [string]::IsNullOrWhiteSpace($_) } | Select-Object -First 1)
            if ($ilkHataSatiri) {
                Write-Output "       stderr: $ilkHataSatiri"
            }
            $basarisiz++
        }
    }

    Write-Output "SONUC: $gecen/$($durumlar.Count) gecti, $basarisiz basarisiz"
} finally {
    if (Test-Path -LiteralPath $geciciKok) {
        Remove-Item -LiteralPath $geciciKok -Recurse -Force -ErrorAction SilentlyContinue
    }
}

if ($basarisiz -gt 0) {
    exit 1
}
exit 0
