param(
  [Parameter(Mandatory = $true)][string]$Mode,
  [Parameter(Mandatory = $true)][string]$Path,
  [int]$TimeoutSeconds = 30
)

Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes

$deadline = (Get-Date).AddSeconds($TimeoutSeconds)
$buttonPattern = if ($Mode -eq 'Save') { 'Save|OK' } else { 'Open|Select|OK' }
$root = [System.Windows.Automation.AutomationElement]::RootElement
$windowCondition = New-Object System.Windows.Automation.PropertyCondition(
  [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
  [System.Windows.Automation.ControlType]::Window
)
$editCondition = New-Object System.Windows.Automation.PropertyCondition(
  [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
  [System.Windows.Automation.ControlType]::Edit
)
$buttonCondition = New-Object System.Windows.Automation.PropertyCondition(
  [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
  [System.Windows.Automation.ControlType]::Button
)

while ((Get-Date) -lt $deadline) {
  # Common dialogs are normally direct root children, but the hosted runner can
  # expose them through a desktop subtree while the shell is initializing.
  $windows = @()
  try { $windows += @($root.FindAll([System.Windows.Automation.TreeScope]::Children, $windowCondition)) } catch {}
  try { $windows += @($root.FindAll([System.Windows.Automation.TreeScope]::Descendants, $windowCondition)) } catch {}
  $seen = @{}
  foreach ($window in $windows) {
    try {
      $id = $window.Current.NativeWindowHandle
      if ($id -and $seen.ContainsKey($id)) { continue }
      if ($id) { $seen[$id] = $true }
      $class = $window.Current.ClassName
      $name = $window.Current.Name
      if ($class -ne '#32770' -and $name -notmatch 'Save|Open|Select') { continue }
      $edit = $window.FindFirst([System.Windows.Automation.TreeScope]::Descendants, $editCondition)
      if (-not $edit) { continue }
      $valuePattern = $edit.GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern)
      $valuePattern.SetValue($Path)
      $buttons = $window.FindAll([System.Windows.Automation.TreeScope]::Descendants, $buttonCondition)
      foreach ($button in $buttons) {
        if ($button.Current.Name -notmatch $buttonPattern) { continue }
        $invoke = $button.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern)
        $invoke.Invoke()
        Write-Output "WINDOW_DIALOG mode=$Mode window=$name button=$($button.Current.Name) path=$Path"
        exit 0
      }
    } catch {
      # The dialog may be between native UI states; retry until the bounded deadline.
    }
  }
  Start-Sleep -Milliseconds 200
}
throw "Timed out waiting for the Windows $Mode file dialog."
