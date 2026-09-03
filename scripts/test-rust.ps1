$ErrorActionPreference = "Stop"

$workspace = Split-Path -Parent $PSScriptRoot
$manifest = Join-Path $workspace "src-tauri\Cargo.toml"
$cargo = Get-Command cargo.exe -ErrorAction SilentlyContinue | Select-Object -First 1 -ExpandProperty Source
if (-not $cargo) {
    $cargo = Join-Path $env:USERPROFILE ".cargo\bin\cargo.exe"
}
if (-not (Test-Path -LiteralPath $cargo -PathType Leaf)) {
    throw "cargo.exe wurde nicht gefunden. Bitte Rust stable installieren."
}

if (Get-Command link.exe -ErrorAction SilentlyContinue) {
    $env:LDTG_TEST_RUNNER_MANIFEST = "1"
    & $cargo test --manifest-path $manifest --lib
    exit $LASTEXITCODE
}

$vswhere = Join-Path ${env:ProgramFiles(x86)} "Microsoft Visual Studio\Installer\vswhere.exe"
if (-not (Test-Path -LiteralPath $vswhere -PathType Leaf)) {
    throw "vswhere.exe wurde nicht gefunden. Bitte Microsoft C++ Build Tools installieren."
}

$installation = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
if (-not $installation) {
    throw "Eine Visual-Studio-Installation mit den C++-Buildtools wurde nicht gefunden."
}
$vcvars = Join-Path $installation "VC\Auxiliary\Build\vcvars64.bat"
if (-not (Test-Path -LiteralPath $vcvars -PathType Leaf)) {
    throw "vcvars64.bat wurde nicht gefunden: $vcvars"
}

$command = 'call "{0}" >nul && set "LDTG_TEST_RUNNER_MANIFEST=1" && "{1}" test --manifest-path "{2}" --lib' -f $vcvars, $cargo, $manifest
& $env:ComSpec /d /s /c $command
exit $LASTEXITCODE
