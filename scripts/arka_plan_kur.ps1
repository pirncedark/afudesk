# Hazırlık betiği: yalnız kullanıcı bilinçli olarak çalıştırdığında kurar.
# Oturum kapalıyken veya kilit ekranında yakalama desteği sağlamaz.
[CmdletBinding(SupportsShouldProcess = $true, ConfirmImpact = 'Medium')]
param(
    [Parameter(Mandatory = $true)]
    [string]$ExePath
)
$ErrorActionPreference = 'Stop'
$resolvedExe = (Resolve-Path -LiteralPath $ExePath).ProviderPath
if (-not (Test-Path -LiteralPath $resolvedExe -PathType Leaf) -or
    [IO.Path]::GetExtension($resolvedExe) -ine '.exe') {
    throw 'AfuDesk bulunamadı; kurulu uygulamanın exe dosyasını seç.'
}
$identity = [Security.Principal.WindowsIdentity]::GetCurrent()
$taskName = 'AfuDesk-Background-' + $identity.User.Value
if (-not $PSCmdlet.ShouldProcess($taskName, 'Oturum açıldığında AfuDesk uygulamasını başlat')) { return }

$scheduler = New-Object -ComObject 'Schedule.Service'
$scheduler.Connect()
$task = $scheduler.NewTask(0)
$task.RegistrationInfo.Description = 'AfuDesk: yalnız bu kullanıcının açık oturumunda başlatır.'
$task.Principal.UserId = $identity.User.Value
$task.Principal.LogonType = 3 # TASK_LOGON_INTERACTIVE_TOKEN; parola saklanmaz.
$task.Principal.RunLevel = 0 # TASK_RUNLEVEL_LUA; yönetici istemez.
$task.Settings.Enabled = $true
$task.Settings.StartWhenAvailable = $true
$task.Settings.DisallowStartIfOnBatteries = $false
$task.Settings.StopIfGoingOnBatteries = $false
$task.Settings.ExecutionTimeLimit = 'PT0S'
$task.Settings.MultipleInstances = 2 # TASK_INSTANCES_IGNORE_NEW
$trigger = $task.Triggers.Create(9) # TASK_TRIGGER_LOGON
$trigger.UserId = $identity.User.Value
$trigger.Enabled = $true
$action = $task.Actions.Create(0) # TASK_ACTION_EXEC
$action.Path = $resolvedExe
$action.WorkingDirectory = [IO.Path]::GetDirectoryName($resolvedExe)
$action.Arguments = '--arka-plan'
# Eski elle kurulum yolu; uygulamadaki ayar HKCU Run kullanır. İkisini birlikte kurmayın.
$folder = $scheduler.GetFolder('\')
$null = $folder.RegisterTaskDefinition($taskName, $task, 6, $identity.User.Value, $null, 3)
Write-Output 'AfuDesk bir sonraki oturum açılışında başlayacak.'
