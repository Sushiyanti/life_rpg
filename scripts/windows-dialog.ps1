param(
  [Parameter(Mandatory = $true)][string]$Mode,
  [Parameter(Mandatory = $true)][string]$Path,
  [int]$TimeoutSeconds = 30
)

Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes

$deadline = (Get-Date).AddSeconds($TimeoutSeconds)
$buttonName = if ($Mode -eq 'Save') { 'Save' } else { 'Open' }
$root = [System.Windows.Automation.AutomationElement]::RootElement
$windowCondition = New-Object System.Windows.Automation.PropertyCondition(
  [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
  [System.Windows.Automation.ControlType]::Window
)

while ((Get-Date) -lt $deadline) {
  $windows = $root.FindAll([System.Windows.Automation.TreeScope]::Children, $windowCondition)
  foreach ($window in $windows) {
    try {
      $class = $window.Current.ClassName
      if ($class -ne '#32770') { continue }
      $editCondition = New-Object System.Windows.Automation.PropertyCondition(
        [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
        [System.Windows.Automation.ControlType]::Edit
      )
      $edit = $window.FindFirst([System.Windows.Automation.TreeScope]::Descendants, $editCondition)
      if (-not $edit) { continue }
      $valuePattern = $edit.GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern)
      $valuePattern.SetValue($Path)
      $buttonCondition = New-Object System.Windows.Automation.AndCondition(
        (New-Object System.Windows.Automation.PropertyCondition(
          [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
          [System.Windows.Automation.ControlType]::Button
        )),
        (New-Object System.Windows.Automation.PropertyCondition(
          [System.Windows.Automation.AutomationElement]::NameProperty,
          $buttonName
        ))
      )
      $button = $window.FindFirst([System.Windows.Automation.TreeScope]::Descendants, $buttonCondition)
      if ($button) {
        $invoke = $button.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern)
        $invoke.Invoke()
        exit 0
      }
    } catch {
      # The dialog may be between native UI states; retry until the bounded deadline.
    }
  }
  Start-Sleep -Milliseconds 200
}
throw "Timed out waiting for the Windows $Mode file dialog."
