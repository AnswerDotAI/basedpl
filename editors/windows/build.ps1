param([Parameter(Mandatory)][string]$Compiler)
$ErrorActionPreference = 'Stop'
$Compiler = (Resolve-Path $Compiler).Path
$root = (Resolve-Path "$PSScriptRoot\..\..").Path
$output = "$root\dist\basedpl-keyboard-us-windows-x64"

foreach ($target in @(@('amd64', '-m'), @('wow64', '-o'))) {
    $directory = New-Item -ItemType Directory -Force "$output\$($target[0])"
    Push-Location $directory
    try {
        if (Test-Path 'bplus.dll') { Remove-Item 'bplus.dll' }
        & $Compiler -u $target[1] "$PSScriptRoot\BasedPL-us.klc"
        if ($LASTEXITCODE -ne 0 -or !(Test-Path 'bplus.dll')) { throw "Keyboard compilation failed for $($target[0])" }
    } finally { Pop-Location }
}
Copy-Item "$PSScriptRoot\BasedPL-us.klc", "$PSScriptRoot\install.ps1", "$PSScriptRoot\README.md" $output
Copy-Item "$root\nbs\fonts\SAX2B.ttf", "$root\nbs\fonts\LICENSE-SAX2", "$root\LICENSE" $output
Compress-Archive -Path "$output\*" -DestinationPath "$output.zip" -Force
