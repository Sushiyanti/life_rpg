param(
  [Parameter(Mandatory = $true)][string]$AppPath,
  [int]$Port = 9222,
  [int]$TimeoutSeconds = 30
)

$ErrorActionPreference = 'Stop'
$exeName = Split-Path -Leaf $AppPath
$userPolicyPath = 'HKCU:\Software\Policies\Microsoft\Edge\WebView2\AdditionalBrowserArguments'
$machinePolicyPath = 'HKLM:\SOFTWARE\Policies\Microsoft\Edge\WebView2\AdditionalBrowserArguments'
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
Write-Diagnostic 'user_policy_path' $userPolicyPath
Write-Diagnostic 'machine_policy_path' $machinePolicyPath
Write-Diagnostic 'policy_argument' $argument

New-Item -Path $userPolicyPath -Force | Out-Null
New-ItemProperty -Path $userPolicyPath -Name $exeName -Value $argument -PropertyType String -Force | Out-Null
New-Item -Path $machinePolicyPath -Force | Out-Null
New-ItemProperty -Path $machinePolicyPath -Name $exeName -Value $argument -PropertyType String -Force | Out-Null
$userPolicyValue = (Get-ItemProperty -Path $userPolicyPath -Name $exeName).$exeName
$machinePolicyValue = (Get-ItemProperty -Path $machinePolicyPath -Name $exeName).$exeName
Write-Diagnostic 'user_policy_value' $userPolicyValue
Write-Diagnostic 'machine_policy_value' $machinePolicyValue

# Set this in the same PowerShell process that creates the real installed app.
# This avoids any ambiguity about inheritance from the outer GitHub Actions step.
$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = $argument
Write-Diagnostic 'environment_argument' $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS

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
