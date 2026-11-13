# Source-only packaging, following native/tools/native.py's documented schema.
# This does NOT compile the mod or certify Horizon compatibility.
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.IO.Compression
Add-Type -AssemblyName System.IO.Compression.FileSystem
$compat = Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot 'compatibility.json') | ConvertFrom-Json
$bundle = Join-Path $PSScriptRoot 'mixins\paper_minimap'
$manifest = Get-Content -Raw -LiteralPath (Join-Path $bundle 'horizon-mixin.toml')
$revision = [regex]::Match($manifest, '(?m)^horizon_revision = "([a-f0-9]{40})"$').Groups[1].Value
$version = [regex]::Match($manifest, '(?m)^version = "([^"]+)"$').Groups[1].Value
if ($revision -ne $compat.horizon_revision -or -not $version) { throw 'Invalid or inconsistent manifest.' }
foreach ($reference in [regex]::Matches($manifest, '(?m)^(?:helper|source) = "([^"]+)"$')) {
    if (-not (Test-Path -LiteralPath (Join-Path $bundle $reference.Groups[1].Value) -PathType Leaf)) {
        throw "Missing bundle file: $($reference.Groups[1].Value)"
    }
}
$dist = Join-Path $PSScriptRoot 'dist'
New-Item -ItemType Directory -Path $dist -Force | Out-Null
$destination = Join-Path $dist "paper_minimap-$version.hnmod"
$metadata = [ordered]@{
    schema = 1; id = 'paper_minimap'; version = $version; game = $compat.game
    horizon = $revision; mixins = $compat.horizon_mixins_revision; hooks = $compat.hooks
} | ConvertTo-Json
$encoding = New-Object System.Text.UTF8Encoding($false)
$files = @{}
$files['horizon-native.json'] = $encoding.GetBytes($metadata + "`n")
foreach ($file in Get-ChildItem -LiteralPath $bundle -File) {
    if ($file.Extension -in @('.rs', '.toml')) {
        $files["mixins/paper_minimap/$($file.Name)"] = [IO.File]::ReadAllBytes($file.FullName)
    }
}
$files['mixins/paper_minimap/LICENSE'] = [IO.File]::ReadAllBytes((Join-Path $PSScriptRoot 'LICENSE'))
$stream = [IO.File]::Open($destination, [IO.FileMode]::Create, [IO.FileAccess]::Write)
try {
    $zip = New-Object IO.Compression.ZipArchive($stream, [IO.Compression.ZipArchiveMode]::Create, $true)
    try {
        foreach ($name in ($files.Keys | Sort-Object)) {
            $entry = $zip.CreateEntry($name, [IO.Compression.CompressionLevel]::Optimal)
            $entry.LastWriteTime = [DateTimeOffset]::new(2026, 1, 1, 0, 0, 0, [TimeSpan]::Zero)
            $entry.ExternalAttributes = -2119958528 # Unix regular file, 0644 (0x81A40000).
            $entryStream = $entry.Open()
            try { $entryStream.Write($files[$name], 0, $files[$name].Length) }
            finally { $entryStream.Dispose() }
        }
    } finally { $zip.Dispose() }
} finally { $stream.Dispose() }
# Read back every entry, compare bytes, and check the metadata in the archive.
$check = [IO.Compression.ZipFile]::OpenRead($destination)
try {
    if ($check.Entries.Count -ne $files.Count) { throw 'Unexpected archive entry count.' }
    foreach ($entry in $check.Entries) {
        $inputStream = $entry.Open()
        $memory = New-Object IO.MemoryStream
        try {
            $inputStream.CopyTo($memory)
            if ([Convert]::ToBase64String($memory.ToArray()) -cne [Convert]::ToBase64String($files[$entry.FullName])) {
                throw "Archive content mismatch: $($entry.FullName)"
            }
        } finally { $inputStream.Dispose(); $memory.Dispose() }
    }
} finally { $check.Dispose() }
Get-FileHash -LiteralPath $destination -Algorithm SHA256
Write-Host 'Source archive verified. Compilation and in-game validation are still required.'
