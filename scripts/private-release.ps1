[CmdletBinding()]
param(
    [Parameter()]
    [ValidatePattern('^[0-9a-fA-F]{40}$')]
    [string]$ExpectedRevision,

    [Parameter()]
    [string]$OutputDirectory,

    [Parameter()]
    [switch]$ValidateOnly
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$script:ReleaseUtf8NoBom = New-Object Text.UTF8Encoding($false)

function Invoke-Captured {
    param(
        [Parameter(Mandatory = $true)][string]$Command,
        [Parameter()][string[]]$CommandArguments = @()
    )

    $commandInfo = Get-Command -Name $Command -ErrorAction SilentlyContinue
    if ($null -eq $commandInfo) { throw "Erforderlicher Befehl nicht gefunden: $Command" }
    $previousPreference = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try {
        $output = @(& $commandInfo.Source @CommandArguments 2>&1)
        $exitCode = $LASTEXITCODE
    }
    finally {
        $ErrorActionPreference = $previousPreference
    }
    if ($exitCode -ne 0) {
        throw "$Command wurde mit Exitcode $exitCode beendet: $($output -join [Environment]::NewLine)"
    }
    return ($output | ForEach-Object { "$_" }) -join [Environment]::NewLine
}

function Invoke-Logged {
    param(
        [Parameter(Mandatory = $true)][string]$Label,
        [Parameter(Mandatory = $true)][string]$Command,
        [Parameter()][string[]]$CommandArguments = @(),
        [Parameter(Mandatory = $true)][string]$LogPath
    )

    $commandInfo = Get-Command -Name $Command -ErrorAction SilentlyContinue
    if ($null -eq $commandInfo) { throw "Erforderlicher Befehl nicht gefunden: $Command" }
    $header = "`n[$([DateTime]::UtcNow.ToString('O'))] $Label`n"
    [IO.File]::AppendAllText($LogPath, $header, $script:ReleaseUtf8NoBom)
    $previousPreference = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try {
        $output = @(& $commandInfo.Source @CommandArguments 2>&1)
        $exitCode = $LASTEXITCODE
    }
    finally {
        $ErrorActionPreference = $previousPreference
    }
    if ($output.Count -gt 0) {
        $outputText = (($output | ForEach-Object { "$_" }) -join [Environment]::NewLine) + [Environment]::NewLine
        [IO.File]::AppendAllText($LogPath, $outputText, $script:ReleaseUtf8NoBom)
        Write-Host ($outputText.TrimEnd())
    }
    if ($exitCode -ne 0) {
        throw "$Label wurde mit Exitcode $exitCode beendet."
    }
}

function Get-Sha256 {
    param([Parameter(Mandatory = $true)][string]$Path)
    return (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

function Get-OptionalEnvironmentValue {
    param([Parameter(Mandatory = $true)][string]$Name)
    $value = [Environment]::GetEnvironmentVariable($Name)
    if ([string]::IsNullOrWhiteSpace($value)) { return $null }
    return $value
}

function Assert-CargoBinaryLayout {
    $cargoMetadataJson = Invoke-Captured -Command 'cargo' -CommandArguments @(
        'metadata',
        '--manifest-path',
        'src-tauri/Cargo.toml',
        '--format-version',
        '1',
        '--no-deps',
        '--locked'
    )
    $cargoMetadata = $cargoMetadataJson | ConvertFrom-Json
    $package = @($cargoMetadata.packages | Where-Object { $_.name -eq 'ldtg' })
    if ($package.Count -ne 1) {
        throw "Erwartet wurde genau ein Cargo-Paket namens ldtg, gefunden: $($package.Count)."
    }

    $actualBinaries = @($package[0].targets | Where-Object { $_.kind -contains 'bin' } | ForEach-Object { $_.name } | Sort-Object)
    $expectedBinaries = @('ldtg', 'ldtg-firewall-cleanup') | Sort-Object
    $binaryDifference = @(Compare-Object -ReferenceObject $expectedBinaries -DifferenceObject $actualBinaries)
    if ($binaryDifference.Count -ne 0) {
        throw "Unerwartete Cargo-Binaerziele: $($actualBinaries -join ', ')."
    }

    $contractGenerator = @($package[0].targets | Where-Object {
        $_.name -eq 'generate-contracts' -and $_.kind -contains 'example'
    })
    if ($contractGenerator.Count -ne 1) {
        throw 'generate-contracts muss genau einmal als nicht paketiertes Cargo-Beispiel definiert sein.'
    }
}

if ($env:OS -ne 'Windows_NT') {
    throw 'Der private Release-Dry-Run ist ausschliesslich fuer Windows vorgesehen.'
}

$repoRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
Push-Location -LiteralPath $repoRoot
try {
    $revision = (Invoke-Captured -Command 'git' -CommandArguments @('rev-parse', 'HEAD')).Trim().ToLowerInvariant()
    if ([string]::IsNullOrWhiteSpace($ExpectedRevision)) { $ExpectedRevision = $revision }
    if ($revision -ne $ExpectedRevision.ToLowerInvariant()) {
        throw "HEAD $revision stimmt nicht mit der erwarteten Revision $ExpectedRevision ueberein."
    }

    $worktreeState = Invoke-Captured -Command 'git' -CommandArguments @('status', '--porcelain=v1', '--untracked-files=all')
    if (-not [string]::IsNullOrWhiteSpace($worktreeState)) {
        throw "Der Release-Dry-Run akzeptiert nur einen sauberen Arbeitsbaum.`n$worktreeState"
    }

    Assert-CargoBinaryLayout

    $metadataJson = Invoke-Captured -Command 'node' -CommandArguments @('scripts/release-metadata.mjs', '--check', '--json')
    $metadata = $metadataJson | ConvertFrom-Json
    Write-Host "Releaseeingaben validiert: LDTG $($metadata.version), Commit $revision"
    if ($ValidateOnly) { return }

    if ([string]::IsNullOrWhiteSpace($OutputDirectory)) {
        $OutputDirectory = Join-Path $repoRoot "artifacts/private-release/$($metadata.version)/$($revision.Substring(0, 12))"
    }
    elseif (-not [IO.Path]::IsPathRooted($OutputDirectory)) {
        $OutputDirectory = Join-Path $repoRoot $OutputDirectory
    }
    $outputPath = [IO.Path]::GetFullPath($OutputDirectory)
    $normalizedRepoRoot = $repoRoot.TrimEnd('\', '/')
    $normalizedOutputPath = $outputPath.TrimEnd('\', '/')
    if ($normalizedOutputPath -eq $normalizedRepoRoot) {
        throw 'Das Repository-Stammverzeichnis darf nicht als Ausgabeverzeichnis verwendet werden.'
    }
    $repoPrefix = $normalizedRepoRoot + [IO.Path]::DirectorySeparatorChar
    $artifactPrefix = [IO.Path]::GetFullPath((Join-Path $repoRoot 'artifacts')).TrimEnd('\', '/') + [IO.Path]::DirectorySeparatorChar
    if ($normalizedOutputPath.StartsWith($repoPrefix, [StringComparison]::OrdinalIgnoreCase) -and
        -not ($normalizedOutputPath + [IO.Path]::DirectorySeparatorChar).StartsWith($artifactPrefix, [StringComparison]::OrdinalIgnoreCase)) {
        throw 'Ausgaben innerhalb des Repositorys sind nur unter dem ignorierten Verzeichnis artifacts erlaubt.'
    }
    if ((Test-Path -LiteralPath $outputPath) -and (Get-ChildItem -LiteralPath $outputPath -Force | Select-Object -First 1)) {
        throw "Das Ausgabeverzeichnis ist nicht leer: $outputPath"
    }
    New-Item -ItemType Directory -Path $outputPath -Force | Out-Null

    $logPath = Join-Path $outputPath 'build.log'
    [IO.File]::WriteAllText(
        $logPath,
        "LDTG private release dry run`nStatus: private, unsigned, not published`nSource revision: $revision`nStarted UTC: $([DateTime]::UtcNow.ToString('O'))`n",
        $script:ReleaseUtf8NoBom
    )

    Invoke-Logged -Label 'Install pnpm dependencies from lockfile' -Command 'pnpm' -CommandArguments @('install', '--frozen-lockfile') -LogPath $logPath
    Invoke-Logged -Label 'Fetch Cargo dependencies from lockfile' -Command 'cargo' -CommandArguments @('fetch', '--manifest-path', 'src-tauri/Cargo.toml', '--locked') -LogPath $logPath

    $previousCargoOffline = Get-OptionalEnvironmentValue -Name 'CARGO_NET_OFFLINE'
    [Environment]::SetEnvironmentVariable('CARGO_NET_OFFLINE', 'true')
    try {
        Invoke-Logged -Label 'Run complete quality gate' -Command 'pnpm' -CommandArguments @('check') -LogPath $logPath
        $buildStartedUtc = [DateTime]::UtcNow
        Invoke-Logged -Label 'Build unsigned NSIS installer' -Command 'pnpm' -CommandArguments @('build') -LogPath $logPath
    }
    finally {
        [Environment]::SetEnvironmentVariable('CARGO_NET_OFFLINE', $previousCargoOffline)
    }

    foreach ($binaryName in @('ldtg.exe', 'ldtg-firewall-cleanup.exe')) {
        $binaryPath = Join-Path $repoRoot "src-tauri/target/release/$binaryName"
        if (-not (Test-Path -LiteralPath $binaryPath -PathType Leaf) -or
            (Get-Item -LiteralPath $binaryPath).LastWriteTimeUtc -lt $buildStartedUtc.AddSeconds(-2)) {
            throw "Das erwartete frische Paket-Binaerziel fehlt: $binaryName"
        }
    }
    $generatorBinaryPath = Join-Path $repoRoot 'src-tauri/target/release/generate-contracts.exe'
    if ((Test-Path -LiteralPath $generatorBinaryPath -PathType Leaf) -and
        (Get-Item -LiteralPath $generatorBinaryPath).LastWriteTimeUtc -ge $buildStartedUtc.AddSeconds(-2)) {
        throw 'Der Buildgenerator generate-contracts.exe wurde unerwartet als Release-Binaerziel gebaut.'
    }

    $installerRoot = Join-Path $repoRoot 'src-tauri/target/release/bundle/nsis'
    $installers = @(Get-ChildItem -LiteralPath $installerRoot -Filter '*.exe' -File | Where-Object {
        $_.Name -like "*$($metadata.version)*setup.exe" -and $_.LastWriteTimeUtc -ge $buildStartedUtc.AddSeconds(-2)
    })
    if ($installers.Count -ne 1) {
        throw "Erwartet wurde genau ein frisch erzeugter NSIS-Installer fuer $($metadata.version), gefunden: $($installers.Count)."
    }

    $installerPath = Join-Path $outputPath $installers[0].Name
    Copy-Item -LiteralPath $installers[0].FullName -Destination $installerPath
    $licensePath = Join-Path $outputPath 'LICENSE'
    $noticePath = Join-Path $outputPath 'NOTICE'
    $thirdPartyNoticesPath = Join-Path $outputPath 'THIRD_PARTY_NOTICES.md'
    Copy-Item -LiteralPath (Join-Path $repoRoot 'LICENSE') -Destination $licensePath
    Copy-Item -LiteralPath (Join-Path $repoRoot 'NOTICE') -Destination $noticePath
    Copy-Item -LiteralPath (Join-Path $repoRoot 'THIRD_PARTY_NOTICES.md') -Destination $thirdPartyNoticesPath

    Invoke-Logged -Label 'Generate commit-bound CycloneDX SBOM' -Command 'node' -CommandArguments @(
        'scripts/release-metadata.mjs',
        '--check',
        "--revision=$revision",
        "--output=$outputPath"
    ) -LogPath $logPath

    $finalTreeState = Invoke-Captured -Command 'git' -CommandArguments @('status', '--porcelain=v1', '--untracked-files=all')
    if (-not [string]::IsNullOrWhiteSpace($finalTreeState)) {
        throw "Der Build hat eingecheckte Quelldateien veraendert.`n$finalTreeState"
    }

    [IO.File]::AppendAllText(
        $logPath,
        "`n[$([DateTime]::UtcNow.ToString('O'))] Build and source-integrity gates passed.`n",
        $script:ReleaseUtf8NoBom
    )
    $tagsText = Invoke-Captured -Command 'git' -CommandArguments @('tag', '--points-at', $revision)
    $sourceTags = @($tagsText -split '\r?\n' | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
    $sbomPath = Join-Path $outputPath 'sbom.cdx.json'
    $signatureStatus = (Get-AuthenticodeSignature -LiteralPath $installerPath).Status.ToString()

    $artifactEntries = @(
        [ordered]@{
            path = [IO.Path]::GetFileName($installerPath)
            type = 'windows-nsis-installer'
            bytes = (Get-Item -LiteralPath $installerPath).Length
            sha256 = Get-Sha256 -Path $installerPath
            signatureStatus = $signatureStatus
        },
        [ordered]@{
            path = 'sbom.cdx.json'
            type = 'cyclonedx-sbom'
            bytes = (Get-Item -LiteralPath $sbomPath).Length
            sha256 = Get-Sha256 -Path $sbomPath
        },
        [ordered]@{
            path = 'build.log'
            type = 'build-log'
            bytes = (Get-Item -LiteralPath $logPath).Length
            sha256 = Get-Sha256 -Path $logPath
        },
        [ordered]@{
            path = 'LICENSE'
            type = 'project-license'
            bytes = (Get-Item -LiteralPath $licensePath).Length
            sha256 = Get-Sha256 -Path $licensePath
        },
        [ordered]@{
            path = 'NOTICE'
            type = 'project-notice'
            bytes = (Get-Item -LiteralPath $noticePath).Length
            sha256 = Get-Sha256 -Path $noticePath
        },
        [ordered]@{
            path = 'THIRD_PARTY_NOTICES.md'
            type = 'third-party-notices'
            bytes = (Get-Item -LiteralPath $thirdPartyNoticesPath).Length
            sha256 = Get-Sha256 -Path $thirdPartyNoticesPath
        }
    )

    $manifest = [ordered]@{
        schemaVersion = 1
        status = 'private-dry-run-not-published'
        version = $metadata.version
        license = $metadata.license
        sourceRevision = $revision
        sourceTags = $sourceTags
        generatedAtUtc = [DateTime]::UtcNow.ToString('O')
        portableArtifactIncluded = $false
        toolchain = [ordered]@{
            node = (Invoke-Captured -Command 'node' -CommandArguments @('--version')).Trim()
            pnpm = (Invoke-Captured -Command 'pnpm' -CommandArguments @('--version')).Trim()
            rustc = (Invoke-Captured -Command 'rustc' -CommandArguments @('--version')).Trim()
            cargo = (Invoke-Captured -Command 'cargo' -CommandArguments @('--version')).Trim()
            windowsImage = Get-OptionalEnvironmentValue -Name 'ImageOS'
            windowsImageVersion = Get-OptionalEnvironmentValue -Name 'ImageVersion'
            processorArchitecture = Get-OptionalEnvironmentValue -Name 'PROCESSOR_ARCHITECTURE'
        }
        workflow = [ordered]@{
            runId = Get-OptionalEnvironmentValue -Name 'GITHUB_RUN_ID'
            runAttempt = Get-OptionalEnvironmentValue -Name 'GITHUB_RUN_ATTEMPT'
            repository = Get-OptionalEnvironmentValue -Name 'GITHUB_REPOSITORY'
        }
        lockfiles = $metadata.lockfiles
        dependencyAuditSourceRevision = $metadata.dependencyAuditSourceRevision
        dependencyComponents = $metadata.dependencyComponents
        actionPins = $metadata.actionPins
        artifacts = $artifactEntries
    }

    $manifestPath = Join-Path $outputPath 'build-manifest.json'
    [IO.File]::WriteAllText($manifestPath, (($manifest | ConvertTo-Json -Depth 10) + "`n"), $script:ReleaseUtf8NoBom)

    $checksumTargets = @($installerPath, $sbomPath, $logPath, $licensePath, $noticePath, $thirdPartyNoticesPath, $manifestPath) | Sort-Object { [IO.Path]::GetFileName($_) }
    $checksumLines = $checksumTargets | ForEach-Object {
        "$(Get-Sha256 -Path $_) *$([IO.Path]::GetFileName($_))"
    }
    [IO.File]::WriteAllText((Join-Path $outputPath 'SHA256SUMS.txt'), (($checksumLines -join "`n") + "`n"), $script:ReleaseUtf8NoBom)

    Write-Host "Privater, unsignierter Dry-Run erfolgreich: $outputPath"
}
finally {
    Pop-Location
}
