$ErrorActionPreference = 'Stop'

$istemciId = $env:AFUDESK_GOOGLE_DESKTOP_CLIENT_ID
$sonEk = '.apps.googleusercontent.com'

if ([string]::IsNullOrWhiteSpace($istemciId)) {
    [Console]::Error.WriteLine('HATA: AFUDESK_GOOGLE_DESKTOP_CLIENT_ID ortam degiskeni bos veya tanimsiz. Release derlemesi Google Desktop Client ID olmadan devam edemez.')
    exit 1
}

$temiz = $istemciId.Trim()
if (
    ($temiz -ne $istemciId) -or
    ($temiz -match '\s') -or
    ($temiz.Length -le $sonEk.Length) -or
    (-not $temiz.EndsWith($sonEk, [System.StringComparison]::Ordinal))
) {
    [Console]::Error.WriteLine('HATA: AFUDESK_GOOGLE_DESKTOP_CLIENT_ID gecersiz bicimde (".apps.googleusercontent.com" ile biten gecerli bir Google OAuth istemci kimligi gerekli).')
    exit 1
}

Write-Output 'OK: AFUDESK_GOOGLE_DESKTOP_CLIENT_ID dogrulandi.'
exit 0
