param(
  [Parameter(Mandatory = $true)][string]$Mode,
  [Parameter(Mandatory = $true)][string]$Path,
  [int]$TimeoutSeconds = 30
)

Add-Type @'
using System;
using System.Text;
using System.Collections.Generic;
using System.Runtime.InteropServices;
public static class LifeRpgWin32Dialog {
  public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc cb, IntPtr lp);
  [DllImport("user32.dll")] public static extern bool EnumChildWindows(IntPtr hWnd, EnumWindowsProc cb, IntPtr lp);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr hWnd, StringBuilder text, int max);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassName(IntPtr hWnd, StringBuilder text, int max);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern IntPtr SendMessage(IntPtr hWnd, uint msg, IntPtr wParam, string lParam);
  [DllImport("user32.dll")] public static extern IntPtr SendMessage(IntPtr hWnd, uint msg, IntPtr wParam, IntPtr lParam);
  public const uint WM_SETTEXT=0x000C, BM_CLICK=0x00F5;
  public static string Text(IntPtr h) { var b=new StringBuilder(512); GetWindowText(h,b,b.Capacity); return b.ToString(); }
  public static string Class(IntPtr h) { var b=new StringBuilder(128); GetClassName(h,b,b.Capacity); return b.ToString(); }
  public static List<IntPtr> Windows() { var r=new List<IntPtr>(); EnumWindows((h,l)=>{ if(IsWindowVisible(h)) r.Add(h); return true; },IntPtr.Zero); return r; }
  public static List<IntPtr> Children(IntPtr parent) { var r=new List<IntPtr>(); EnumChildWindows(parent,(h,l)=>{r.Add(h); return true;},IntPtr.Zero); return r; }
}
'@

$deadline = (Get-Date).AddSeconds($TimeoutSeconds)
$buttonPattern = if ($Mode -eq 'Save') { 'Save|OK' } else { 'Open|Select|OK' }
while ((Get-Date) -lt $deadline) {
  foreach ($window in [LifeRpgWin32Dialog]::Windows()) {
    try {
      $windowClass = [LifeRpgWin32Dialog]::Class($window)
      $windowText = [LifeRpgWin32Dialog]::Text($window)
      if ($windowClass -ne '#32770' -and $windowText -notmatch 'Save|Open|Select') { continue }
      $children = [LifeRpgWin32Dialog]::Children($window)
      $edit = $children | Where-Object { [LifeRpgWin32Dialog]::Class($_) -eq 'Edit' } | Select-Object -First 1
      $button = $children | Where-Object {
        [LifeRpgWin32Dialog]::Class($_) -eq 'Button' -and
        ([LifeRpgWin32Dialog]::Text($_) -match $buttonPattern)
      } | Select-Object -First 1
      if (-not $edit -or -not $button) { continue }
      [LifeRpgWin32Dialog]::SetForegroundWindow($window) | Out-Null
      [LifeRpgWin32Dialog]::SendMessage($edit, [LifeRpgWin32Dialog]::WM_SETTEXT, [IntPtr]::Zero, $Path) | Out-Null
      Start-Sleep -Milliseconds 100
      [LifeRpgWin32Dialog]::SendMessage($button, [LifeRpgWin32Dialog]::BM_CLICK, [IntPtr]::Zero, [IntPtr]::Zero) | Out-Null
      Write-Output "WINDOW_DIALOG mode=$Mode window=$windowText button=$([LifeRpgWin32Dialog]::Text($button)) path=$Path"
      exit 0
    } catch {
      # Retry while the native dialog is still initializing.
    }
  }
  Start-Sleep -Milliseconds 200
}
throw "Timed out waiting for the Windows $Mode file dialog."
