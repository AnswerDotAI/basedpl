param([string]$VisualStudio)
$ErrorActionPreference = 'Stop'
if (!$VisualStudio) {
    $vswhere = [Environment]::GetFolderPath('ProgramFilesX86') + '\Microsoft Visual Studio\Installer\vswhere.exe'
    $VisualStudio = & $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.ARM64 -property installationPath
}
if (!$VisualStudio) { throw 'Install MSVC ARM64 build tools and a Windows SDK.' }
$root = (Resolve-Path "$PSScriptRoot\..\..").Path
$output = "$root\dist\basedpl-keyboard-us-windows"
if (Test-Path $output) { Remove-Item $output -Recurse -Force }

foreach ($target in @('amd64', 'arm64')) {
    & "$VisualStudio\Common7\Tools\Launch-VsDevShell.ps1" -Arch $target -HostArch amd64
    $directory = New-Item -ItemType Directory -Force "$output\$target"
    Push-Location $directory
    try {
        & cl.exe /nologo /c /O2 /GS- /DWIN32_LEAN_AND_MEAN "$PSScriptRoot\keyboard.c" /Fobplus.obj
        if ($LASTEXITCODE -ne 0) { throw "Keyboard compilation failed for $target" }
        & rc.exe /nologo /fo bplus.res "$PSScriptRoot\keyboard.rc"
        if ($LASTEXITCODE -ne 0) { throw 'Keyboard resource compilation failed.' }
        $hybrid = @()
        if ($target -eq 'arm64') {
            & cl.exe /nologo /c /O2 /GS- /DWIN32_LEAN_AND_MEAN /arm64EC "$PSScriptRoot\keyboard.c" /Fobplus-ec.obj
            if ($LASTEXITCODE -ne 0) { throw 'Keyboard ARM64EC compilation failed.' }
            $hybrid = @('/machine:arm64x', '/include:_load_config_used', "/defArm64Native:$PSScriptRoot\keyboard.def", 'bplus-ec.obj')
        }
        & link.exe /nologo /dll /noentry "/def:$PSScriptRoot\keyboard.def" @hybrid /out:bplus.dll bplus.obj bplus.res /merge:.edata=.data /merge:.rdata=.data /merge:.text=.data /merge:.bss=.data /section:.data,re /ignore:4254
        if ($LASTEXITCODE -ne 0) { throw "Keyboard linking failed for $target" }
        Remove-Item bplus.obj, bplus.res, bplus.lib, bplus.exp
        if ($target -eq 'arm64') { Remove-Item bplus-ec.obj, bplus.a64n.exp }
    } finally { Pop-Location }
}
Copy-Item "$PSScriptRoot\layout.h", "$PSScriptRoot\keyboard.c", "$PSScriptRoot\keyboard.rc", "$PSScriptRoot\keyboard.def",
    "$PSScriptRoot\install.ps1", "$PSScriptRoot\README.md" $output
Copy-Item "$root\nbs\fonts\SAX2B.ttf", "$root\nbs\fonts\LICENSE-SAX2", "$root\LICENSE" $output
Compress-Archive -Path "$output\*" -DestinationPath "$output.zip" -Force
