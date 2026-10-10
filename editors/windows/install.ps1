#Requires -RunAsAdministrator
$ErrorActionPreference = 'Stop'
$architecture = [Environment]::GetEnvironmentVariable('PROCESSOR_ARCHITECTURE', 'Machine').ToLower()
if (![Environment]::Is64BitProcess -or $architecture -notin @('amd64', 'arm64')) { throw 'Run 64-bit PowerShell on x64 or ARM64 Windows.' }
$registry = 'HKLM:\SYSTEM\CurrentControlSet\Control\Keyboard Layouts'
$entries = @(Get-ChildItem $registry | Get-ItemProperty)
$existing = $entries | Where-Object { $_.'Layout File' -eq 'bplus.dll' } | Select-Object -First 1

if ($existing) {
    $klid, $layoutId = $existing.PSChildName, $existing.'Layout Id'
} else {
    $klid = 0..4095 | ForEach-Object { 'a{0:x3}0409' -f $_ } |
        Where-Object { $_ -notin $entries.PSChildName } | Select-Object -First 1
    $layoutId = 1..4095 | ForEach-Object { '{0:x4}' -f $_ } |
        Where-Object { $_ -notin $entries.'Layout Id' } | Select-Object -First 1
    if (!$klid -or !$layoutId) { throw 'No free keyboard layout ID.' }
}
Copy-Item "$PSScriptRoot\$architecture\bplus.dll" "$env:WINDIR\System32\bplus.dll" -Force
$key = New-Item "$registry\$klid" -Force
New-ItemProperty $key.PSPath -Name 'Layout File' -Value 'bplus.dll' -PropertyType String -Force | Out-Null
New-ItemProperty $key.PSPath -Name 'Layout Text' -Value 'BasedPL-us' -PropertyType String -Force | Out-Null
New-ItemProperty $key.PSPath -Name 'Layout Id' -Value $layoutId -PropertyType String -Force | Out-Null
New-ItemProperty $key.PSPath -Name 'Layout Display Name' -Value '@%SystemRoot%\system32\bplus.dll,-1000' -PropertyType ExpandString -Force | Out-Null
Write-Host 'Installed BasedPL-us. Add it under English (United States) in Settings > Time & language > Language & region.'
Copy-Item "$PSScriptRoot\SAX2B.ttf" "$env:WINDIR\Fonts\SAX2B.ttf" -Force
New-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Fonts' -Name 'SAX2B (TrueType)' -Value 'SAX2B.ttf' -PropertyType String -Force | Out-Null
Write-Host 'SAX2B font installed. Switch layouts with Win+Space; sign out and in if BasedPL-us is not listed.'
