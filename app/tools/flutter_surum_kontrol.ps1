param(
    [Parameter(Position = 0)]
    [string]$WorkflowsDir = (Join-Path '.github' 'workflows'),
    [Parameter(Position = 1)]
    [string]$PubspecPath = ''
)

$ErrorActionPreference = 'Stop'

function ConvertTo-TemelSurum {
    param([string]$Metin)
    if ($Metin -match '^(\d+)\.(\d+)(?:\.(\d+))?(?:[-+].*)?$') {
        $maj = [int]$Matches[1]
        $min = [int]$Matches[2]
        $pat = if ($Matches[3]) { [int]$Matches[3] } else { 0 }
        return [version]::new($maj, $min, $pat)
    }
    return $null
}

function Test-FlutterKisitUyumlu {
    param(
        [string]$MevcutSurumMetin,
        [string]$KisitMetin
    )

    $temizKisit = $KisitMetin.Trim()
    if ([string]::IsNullOrWhiteSpace($temizKisit) -or ($temizKisit -eq 'any')) {
        return $true
    }

    $mevcut = ConvertTo-TemelSurum -Metin $MevcutSurumMetin
    if ($null -eq $mevcut) {
        [Console]::Error.WriteLine("HATA: Is akislarindaki Flutter surumu ($MevcutSurumMetin) gecerli bir surum biciminde degil.")
        exit 1
    }

    $duzgun = $temizKisit -replace '(>=|<=|>|<|=|\^)\s+', '$1'
    $parcalar = @($duzgun -split '\s+' | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
    if ($parcalar.Count -eq 0) {
        return $true
    }

    foreach ($parca in $parcalar) {
        if ($parca -match '^\^(.+)$') {
            $alt = ConvertTo-TemelSurum -Metin $Matches[1]
            if ($null -eq $alt) {
                [Console]::Error.WriteLine("HATA: pubspec.yaml icindeki Flutter surum kisiti anlasilamadi: $KisitMetin")
                exit 1
            }
            if ($alt.Major -eq 0) {
                if ($alt.Minor -eq 0) {
                    $ust = [version]::new(0, 0, $alt.Build + 1)
                } else {
                    $ust = [version]::new(0, $alt.Minor + 1, 0)
                }
            } else {
                $ust = [version]::new($alt.Major + 1, 0, 0)
            }
            if (-not (($mevcut -ge $alt) -and ($mevcut -lt $ust))) {
                return $false
            }
        } elseif ($parca -match '^>=(.+)$') {
            $hedef = ConvertTo-TemelSurum -Metin $Matches[1]
            if ($null -eq $hedef) {
                [Console]::Error.WriteLine("HATA: pubspec.yaml icindeki Flutter surum kisiti anlasilamadi: $KisitMetin")
                exit 1
            }
            if (-not ($mevcut -ge $hedef)) { return $false }
        } elseif ($parca -match '^<=(.+)$') {
            $hedef = ConvertTo-TemelSurum -Metin $Matches[1]
            if ($null -eq $hedef) {
                [Console]::Error.WriteLine("HATA: pubspec.yaml icindeki Flutter surum kisiti anlasilamadi: $KisitMetin")
                exit 1
            }
            if (-not ($mevcut -le $hedef)) { return $false }
        } elseif ($parca -match '^>(.+)$') {
            $hedef = ConvertTo-TemelSurum -Metin $Matches[1]
            if ($null -eq $hedef) {
                [Console]::Error.WriteLine("HATA: pubspec.yaml icindeki Flutter surum kisiti anlasilamadi: $KisitMetin")
                exit 1
            }
            if (-not ($mevcut -gt $hedef)) { return $false }
        } elseif ($parca -match '^<(.+)$') {
            $hedef = ConvertTo-TemelSurum -Metin $Matches[1]
            if ($null -eq $hedef) {
                [Console]::Error.WriteLine("HATA: pubspec.yaml icindeki Flutter surum kisiti anlasilamadi: $KisitMetin")
                exit 1
            }
            if (-not ($mevcut -lt $hedef)) { return $false }
        } elseif ($parca -match '^=(.+)$') {
            $hedef = ConvertTo-TemelSurum -Metin $Matches[1]
            if ($null -eq $hedef) {
                [Console]::Error.WriteLine("HATA: pubspec.yaml icindeki Flutter surum kisiti anlasilamadi: $KisitMetin")
                exit 1
            }
            if (-not ($mevcut -eq $hedef)) { return $false }
        } else {
            $hedef = ConvertTo-TemelSurum -Metin $parca
            if ($null -eq $hedef) {
                [Console]::Error.WriteLine("HATA: pubspec.yaml icindeki Flutter surum kisiti anlasilamadi: $KisitMetin")
                exit 1
            }
            if (-not ($mevcut -eq $hedef)) { return $false }
        }
    }

    return $true
}

$workflowsVerildi = $PSBoundParameters.ContainsKey('WorkflowsDir')
if ((-not $workflowsVerildi) -and (-not (Test-Path -LiteralPath $WorkflowsDir))) {
    $depoKoku = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
    $WorkflowsDir = Join-Path (Join-Path $depoKoku '.github') 'workflows'
}

if (-not (Test-Path -LiteralPath $WorkflowsDir -PathType Container)) {
    [Console]::Error.WriteLine("HATA: Is akisi klasoru bulunamadi: $WorkflowsDir")
    exit 1
}

$dosyalar = @(Get-ChildItem -LiteralPath $WorkflowsDir -File | Where-Object { $_.Extension -in @('.yml', '.yaml') } | Sort-Object Name)
if ($dosyalar.Count -eq 0) {
    [Console]::Error.WriteLine("HATA: Is akisi klasorunde hic YAML dosyasi bulunamadi: $WorkflowsDir")
    exit 1
}

$bulunanlar = @()
foreach ($dosya in $dosyalar) {
    $satirlar = [System.IO.File]::ReadAllLines($dosya.FullName, [System.Text.Encoding]::UTF8)
    for ($i = 0; $i -lt $satirlar.Count; $i++) {
        $satir = $satirlar[$i]
        if ($satir -match '^\s*#') { continue }
        if ($satir -match '^\s*flutter-version\s*:\s*(.*)$') {
            $ham = $Matches[1].Trim()
            if ($ham -match "^'([^']*)'\s*(?:#.*)?$") {
                $temiz = $Matches[1].Trim()
            } elseif ($ham -match '^"([^"]*)"\s*(?:#.*)?$') {
                $temiz = $Matches[1].Trim()
            } else {
                $temiz = ($ham -replace '\s*#.*$', '').Trim()
            }
            $satirNo = $i + 1
            if ([string]::IsNullOrWhiteSpace($temiz)) {
                [Console]::Error.WriteLine("HATA: Bos flutter-version degeri bulundu: $($dosya.Name):$satirNo")
                exit 1
            }
            $bulunanlar += [PSCustomObject]@{
                Dosya = $dosya.Name
                Satir = $satirNo
                Surum = $temiz
            }
        }
    }
}

if ($bulunanlar.Count -eq 0) {
    [Console]::Error.WriteLine("HATA: Is akisi dosyalarinda hic flutter-version tanimi bulunamadi: $WorkflowsDir")
    exit 1
}

foreach ($kayit in $bulunanlar) {
    Write-Output "Bulunan flutter-version: $($kayit.Dosya):$($kayit.Satir) -> $($kayit.Surum)"
}

$benzersiz = @($bulunanlar | ForEach-Object { $_.Surum } | Sort-Object -Unique)
if ($benzersiz.Count -gt 1) {
    [Console]::Error.WriteLine("HATA: Is akislarindaki flutter-version degerleri tutarsiz: $($benzersiz -join ', '). Tum is akislari ayni Flutter surumunu kullanmalidir.")
    exit 1
}

$ortakSurum = $benzersiz[0]

$hedefPubspec = $null
$pubspecVerildi = $PSBoundParameters.ContainsKey('PubspecPath') -and (-not [string]::IsNullOrWhiteSpace($PubspecPath))
if ($pubspecVerildi) {
    if (-not (Test-Path -LiteralPath $PubspecPath -PathType Leaf)) {
        [Console]::Error.WriteLine("HATA: pubspec.yaml bulunamadi: $PubspecPath")
        exit 1
    }
    $hedefPubspec = $PubspecPath
} elseif (-not $workflowsVerildi) {
    $yerelPubspec = Join-Path 'app' 'pubspec.yaml'
    if (Test-Path -LiteralPath $yerelPubspec -PathType Leaf) {
        $hedefPubspec = $yerelPubspec
    } else {
        $depoKoku = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
        $kokPubspec = Join-Path (Join-Path $depoKoku 'app') 'pubspec.yaml'
        if (Test-Path -LiteralPath $kokPubspec -PathType Leaf) {
            $hedefPubspec = $kokPubspec
        }
    }
}

if ($hedefPubspec) {
    $pubSatirlar = [System.IO.File]::ReadAllLines($hedefPubspec, [System.Text.Encoding]::UTF8)
    $inEnv = $false
    $flutterKisiti = $null
    foreach ($pSatir in $pubSatirlar) {
        if ($pSatir -match '^\s*#') { continue }
        if ($pSatir -match '^environment\s*:\s*(?:#.*)?$') {
            $inEnv = $true
            continue
        }
        if ($inEnv) {
            if ($pSatir -match '^\S') {
                $inEnv = $false
                continue
            }
            if ($pSatir -match '^\s+flutter\s*:\s*(.+)$') {
                $hamKisit = $Matches[1].Trim()
                if ($hamKisit -match "^'([^']*)'\s*(?:#.*)?$") {
                    $flutterKisiti = $Matches[1].Trim()
                } elseif ($hamKisit -match '^"([^"]*)"\s*(?:#.*)?$') {
                    $flutterKisiti = $Matches[1].Trim()
                } else {
                    $flutterKisiti = ($hamKisit -replace '\s*#.*$', '').Trim()
                }
                break
            }
        }
    }

    if (-not [string]::IsNullOrWhiteSpace($flutterKisiti)) {
        if (-not (Test-FlutterKisitUyumlu -MevcutSurumMetin $ortakSurum -KisitMetin $flutterKisiti)) {
            [Console]::Error.WriteLine("HATA: Is akislarindaki Flutter surumu ($ortakSurum), pubspec.yaml icindeki Flutter surum kisiti ($flutterKisiti) ile uyumsuz.")
            exit 1
        }
        Write-Output "OK: pubspec.yaml Flutter surum kisiti ($flutterKisiti) ile uyumlu."
    } else {
        Write-Output 'Bilgi: pubspec.yaml icinde Flutter surum kisiti yok.'
    }
}

Write-Output "OK: Tum is akislarinda flutter-version tutarli ($ortakSurum, $($bulunanlar.Count) konum)."
exit 0
