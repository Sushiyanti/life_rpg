param(
  [Parameter(Mandatory = $true)][string]$AppPath,
  [int]$Port = 9222,
  [int]$TimeoutSeconds = 30
)

$ErrorActionPreference = 'Stop'
$exeName = Split-Path -Leaf $AppPath
$policyPath = 'HKCU:\Software\Policies\Microsoft\Edge\WebView2\AdditionalBrowserArguments'
$argument = "--remote-debugging-port=$Port"

function Write-Diagnostic([string]$Name, [object]$Value) {
  Write-Host "WEBVIEW2_DIAGNOSTIC $Name=$Value"
}

Write-Diagnostic 'account' (whoami)
$identity = [Security.Principal.WindowsIdentity]::GetCurrent()
$principal = New-Object Security.Principal.WindowsPrincipal($identity)
Write-Diagnostic 'elevated' $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
Write-Diagnostic 'app_path' $AppPath
Write-Diagnostic 'app_executable' $exeName
Write-Diagnostic 'policy_path' $policyPath
Write-Diagnostic 'policy_argument' $argument

New-Item -Path $policyPath -Force | Out-Null
New-ItemProperty -Path $policyPath -Name $exeName -Value $argument -PropertyType String -Force | Out-Null
$policyValue = (Get-ItemProperty -Path $policyPath -Name $exeName).$exeName
Write-Diagnostic 'policy_value' $policyValue

$app = Start-Process -FilePath $AppPath -PassThru
Write-Diagnostic 'app_pid' $app.Id
$deadline = (Get-Date).AddSeconds($TimeoutSeconds)
$endpoint = "http://127.0.0.1:$Port/json/version"
$endpointResponse = $null

while ((Get-Date) -lt $deadline) {
  $processes = @(Get-CimInstance Win32_Process -Filter "Name='msedgewebview2.exe'" -ErrorAction SilentlyContinue)
  foreach ($process in $processes) {
    $commandLine = [string]$process.CommandLine
    $parent = Get-CimInstance Win32_Process -Filter "ProcessId=$($process.ParentProcessId)" -ErrorAction SilentlyContinue
    $parentName = if ($parent) { $parent.Name } else { 'unknown' }
    Write-Diagnostic "browser_pid_$($process.ProcessId)" "parent=$($process.ParentProcessId):$parentName cmd=$commandLine"
  }
  $appRecord = Get-CimInstance Win32_Process -Filter "ProcessId=$($app.Id)" -ErrorAction SilentlyContinue
  if ($appRecord) { Write-Diagnostic 'app_command_line' $appRecord.CommandLine }
  $connection = Get-NetTCPConnection -LocalAddress '127.0.0.1' -LocalPort $Port -State Listen -ErrorAction SilentlyContinue
  if ($connection) {
    Write-Diagnostic 'port_listening' "port=$Port owning_pid=$($connection.OwningProcess)"
    try {
      $endpointResponse = Invoke-RestMethod -Uri $endpoint -TimeoutSec 3
      if ($endpointResponse.webSocketDebuggerUrl -or $endpointResponse.Browser) {
        Write-Diagnostic 'json_version' (($endpointResponse | ConvertTo-Json -Compress))
        Write-Diagnostic 'cdp' 'PASS'
        exit 0
      }
    } catch {
      Write-Diagnostic 'json_version_error' $_.Exception.Message
    }
  }
  Start-Sleep -Milliseconds 500
}

Write-Diagnostic 'cdp' 'FAIL'
throw "WebView2 CDP endpoint was not available at $endpoint."
