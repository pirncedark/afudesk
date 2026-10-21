[CmdletBinding(SupportsShouldProcess = $true, ConfirmImpact = 'Medium')]
param()
$ErrorActionPreference = 'Stop'
$sid = [Security.Principal.WindowsIdentity]::GetCurrent().User.Value
$taskName = 'AfuDesk-Background-' + $sid
if (-not $PSCmdlet.ShouldProcess($taskName, 'AfuDesk otomatik başlatma görevini kaldır')) { return }
$scheduler = New-Object -ComObject 'Schedule.Service'
$scheduler.Connect()
$folder = $scheduler.GetFolder('\')
try {
    $null = $folder.GetTask($taskName)
} catch {
    if ($_.Exception.HResult -eq -2147024894) {
        Write-Output 'AfuDesk otomatik başlatma görevi zaten kaldırılmış.'
        return
    }
    throw
}
$folder.DeleteTask($taskName, 0)
Write-Output 'AfuDesk otomatik başlatma görevi kaldırıldı.'
