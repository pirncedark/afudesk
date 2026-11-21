$ErrorActionPreference = 'Stop'

$betik = Join-Path $PSScriptRoot 'client_id_kontrol.ps1'

$pwshKomut = Get-Command pwsh -ErrorAction SilentlyContinue
if ($pwshKomut) {
    $shellYolu = $pwshKomut.Source
    Write-Output "Shell: $shellYolu"
} else {
    $shellYolu = (Get-Command powershell -ErrorAction Stop).Source
    Write-Output "Shell: $shellYolu (pwsh bulunamadi, yedek)"
}

$durumlar = @(
    @{ Ad = 'bos';           Deger = '';                              BeklenenKod = 1 },
    @{ Ad = 'gecersiz';      Deger = 'gecersiz-istemci-kimligi-999';  BeklenenKod = 1 },
    @{ Ad = 'sahte_gecerli'; Deger = 'x.apps.googleusercontent.com';  BeklenenKod = 0 }
)

$gecen = 0
$basarisiz = 0

foreach ($d in $durumlar) {
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = $shellYolu
    $argList = @('-NoProfile')
    if ($env:OS -eq 'Windows_NT') {
        $argList += @('-ExecutionPolicy', 'Bypass')
    }
    $argList += @('-File', ('"' + $betik + '"'))
    $psi.Arguments = $argList -join ' '
    $psi.UseShellExecute = $false
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.CreateNoWindow = $true

    if ([string]::IsNullOrEmpty($d.Deger)) {
        if ($psi.EnvironmentVariables.ContainsKey('AFUDESK_GOOGLE_DESKTOP_CLIENT_ID')) {
            $psi.EnvironmentVariables.Remove('AFUDESK_GOOGLE_DESKTOP_CLIENT_ID')
        }
    } else {
        $psi.EnvironmentVariables['AFUDESK_GOOGLE_DESKTOP_CLIENT_ID'] = $d.Deger
    }

    $proc = [System.Diagnostics.Process]::Start($psi)
    $stdoutTask = $proc.StandardOutput.ReadToEndAsync()
    $stderrTask = $proc.StandardError.ReadToEndAsync()
    $proc.WaitForExit()
    $stdout = $stdoutTask.GetAwaiter().GetResult()
    $stderr = $stderrTask.GetAwaiter().GetResult()
    $kod = $proc.ExitCode
    $proc.Dispose()

    $birlesik = "$stdout`n$stderr"
    $sizintiVar = (-not [string]::IsNullOrEmpty($d.Deger)) -and ($birlesik.IndexOf($d.Deger, [System.StringComparison]::OrdinalIgnoreCase) -ge 0)
    $hataMesajiEksik = ($d.BeklenenKod -eq 1) -and [string]::IsNullOrWhiteSpace($stderr)

    if (($kod -eq $d.BeklenenKod) -and (-not $sizintiVar) -and (-not $hataMesajiEksik)) {
        Write-Output "[PASS] $($d.Ad): beklenen=$($d.BeklenenKod) gercek=$kod"
        $gecen++
    } else {
        $detay = @()
        if ($kod -ne $d.BeklenenKod) { $detay += "kod uyusmadi" }
        if ($sizintiVar) { $detay += "deger loga sizdi" }
        if ($hataMesajiEksik) { $detay += "hata mesaji bos" }
        Write-Output "[FAIL] $($d.Ad): beklenen=$($d.BeklenenKod) gercek=$kod ($($detay -join ', '))"
        $ilkHataSatiri = ($stderr -split "`r?`n" | Where-Object { -not [string]::IsNullOrWhiteSpace($_) } | Select-Object -First 1)
        if ($ilkHataSatiri) {
            Write-Output "       stderr: $ilkHataSatiri"
        }
        $basarisiz++
    }
}

Write-Output "SONUC: $gecen/$($durumlar.Count) gecti, $basarisiz basarisiz"
if ($basarisiz -gt 0) {
    exit 1
}
exit 0
