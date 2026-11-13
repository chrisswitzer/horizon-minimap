param(
    [string]$Launcher = "$env:LOCALAPPDATA\Programs\Horizon Launcher\horizon-launcher.exe",
    [string]$Instance
)
$ErrorActionPreference = 'Stop'
if (-not (Test-Path -LiteralPath $Launcher -PathType Leaf)) {
    throw "Launcher unavailable at '$Launcher'. Run .\Build.ps1 -Launcher 'C:\path\horizon-launcher.exe'."
}
$bundle = Join-Path $PSScriptRoot 'mixins\paper_minimap'
# The launcher builds with its managed Rust toolchain. No global settings are changed.
$buildArgs = @('native-build', $bundle, '--trust-native-code')
if ($Instance) { $buildArgs += @('--instance', $Instance) }
Write-Host 'Building Paper Minimap. The first native build may take 20-30 minutes.'
& $Launcher @buildArgs
if ($LASTEXITCODE -ne 0) { throw "Horizon native build failed (exit $LASTEXITCODE)." }
Write-Host 'Build succeeded. Open Horizon and press Play for the instance you built.'
