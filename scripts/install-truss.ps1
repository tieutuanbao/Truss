<#
.SYNOPSIS
Bootstrap the Rust `truss` CLI and install the Truss core into a target.

.DESCRIPTION
Installs the repository-centered core plus the Rust maintenance CLI. Optional
add-ons requested with -WithEngineeringWisdom, -WithDelivery, or -WithPlanning
are acquired here and installed or updated by the Truss CLI, which owns the
plan, the baseline, and the provenance under .truss-core/:
  truss addon status   --name <name>
  truss addon install  --name <name> --manifest <manifest> --source <dir> --source-ref <ref>
  truss addon update   --name <name> --manifest <manifest> --source <dir> --source-ref <ref>
  truss addon continue --name <name>
  truss addon abort    --name <name>
--source-ref is an immutable release tag or exact commit SHA, never a branch
name, and each managed file is recorded with its SHA-256 in
.truss-core/addons.json. -DryRun previews the plan without writing.
Overlapping local and upstream edits are never overwritten: update stops and
stages the conflict under .truss-core/addon-update/<name>/resolved/, and
continue applies the operator-edited copies while abort removes only the
session. AGENTS.md is never an add-on payload file. -Merge and -Force do not
apply to add-on files, and this installer never copies an add-on file
directly. A remote raw source base URL must be pinned to the release tag or the
add-on step stops.
#>

[CmdletBinding()]
param(
    [Alias("d")]
    [string]$Directory = $env:TRUSS_TARGET_DIR,
    [Alias("y")]
    [switch]$Yes,
    [switch]$Merge,
    [switch]$WithEngineeringWisdom,
    [switch]$WithDelivery,
    [switch]$WithPlanning,
    [switch]$RefreshAgentShim,
    [switch]$Override,
    [switch]$Force,
    [switch]$DryRun
)

$ErrorActionPreference = "Stop"

function Write-Step([string]$Message) {
    Write-Host $Message
}

function Fail([string]$Message) {
    throw "Error: $Message"
}

function Resolve-TargetPath([string]$PathValue) {
    if ([string]::IsNullOrWhiteSpace($PathValue)) {
        $PathValue = (Get-Location).Path
    }

    $expanded = [Environment]::ExpandEnvironmentVariables($PathValue)
    if ($expanded.StartsWith("~")) {
        $expanded = Join-Path $HOME $expanded.Substring(1).TrimStart("\", "/")
    }
    if ([System.IO.Path]::IsPathRooted($expanded)) {
        return [System.IO.Path]::GetFullPath($expanded)
    }
    return [System.IO.Path]::GetFullPath((Join-Path (Get-Location).Path $expanded))
}

function Get-SourceMode {
    if ($PSScriptRoot) {
        $candidate = Split-Path -Parent $PSScriptRoot
        if ((Test-Path (Join-Path $candidate "AGENTS.md")) -and (Test-Path (Join-Path $candidate ".truss-core/docs/TRUSS.md"))) {
            return @{ Mode = "local"; Root = $candidate }
        }
    }
    return @{ Mode = "remote"; Root = "" }
}

function Read-RemoteText([string]$Url) {
    if ($Url.StartsWith("file://")) {
        return Get-Content -LiteralPath ([uri]$Url).LocalPath -Raw
    }
    return (Invoke-WebRequest -UseBasicParsing -Uri $Url).Content
}

function Read-SourceText([string]$Relative) {
    if ($script:Source.Mode -eq "local") {
        $source = Join-Path $script:Source.Root $Relative
        if (!(Test-Path $source)) {
            Fail "Source file missing: $source"
        }
        return Get-Content -LiteralPath $source -Raw
    }

    $url = "$script:SourceBaseUrl/$($Relative -replace '\\','/')"
    return Read-RemoteText $url
}

function Read-PayloadManifest([string]$Manifest) {
    if ($script:Source.Mode -eq "local") {
        $path = Join-Path $script:Source.Root $Manifest
        if (!(Test-Path $path)) {
            Fail "Payload manifest missing: $path"
        }
        return Get-Content -LiteralPath $path
    }

    $url = "$script:SourceBaseUrl/$Manifest"
    try {
        return ((Read-RemoteText $url) -split "\r?\n")
    } catch {
        Fail "Could not download $url"
    }
}

function Get-PayloadFiles([string]$Manifest) {
    foreach ($line in (Read-PayloadManifest $Manifest)) {
        $relative = $line.Trim()
        if ([string]::IsNullOrWhiteSpace($relative) -or $relative.StartsWith("#")) {
            continue
        }
        $relative
    }
}

function Get-AgentShimBlock {
    return (Read-SourceText "scripts/agent-truss-block.md").TrimEnd("`r", "`n")
}

function Assert-TrussMarkers([string]$Content, [string]$Label) {
    $begin = [regex]::Matches($Content, '<!-- TRUSS:BEGIN -->')
    $end = [regex]::Matches($Content, '<!-- TRUSS:END -->')
    if ($begin.Count -eq 0 -and $end.Count -eq 0) {
        return
    }
    if ($begin.Count -ne 1 -or $end.Count -ne 1) {
        Fail "$Label must contain exactly one complete Truss marker pair"
    }
    if ($begin[0].Index -ge $end[0].Index) {
        Fail "$Label Truss markers are out of order"
    }
}

function Refresh-AgentShimFile {
    if (!$RefreshAgentShim) {
        return
    }
    $target = Join-Path $script:TargetDir "AGENTS.md"
    if (!(Test-Path $target)) {
        return
    }

    $content = Get-Content -LiteralPath $target -Raw
    Assert-TrussMarkers $content "AGENTS.md"

    if ($DryRun) {
        Write-Step "refresh  AGENTS.md (replace marked Truss block, backup first)"
        $script:Updated++
        return
    }

    New-Item -ItemType Directory -Force -Path $script:BackupDir | Out-Null
    $backup = Join-Path $script:BackupDir "AGENTS.md"
    if (!(Test-Path $backup)) {
        Copy-Item -LiteralPath $target -Destination $backup
    }

    $block = Get-AgentShimBlock
    if ($content -match "(?s)<!-- TRUSS:BEGIN -->.*?<!-- TRUSS:END -->") {
        $content = [regex]::Replace($content, "(?s)<!-- TRUSS:BEGIN -->.*?<!-- TRUSS:END -->", [System.Text.RegularExpressions.MatchEvaluator]{ param($m) $block })
    } else {
        $content = $content.TrimEnd() + "`n`n" + $block + "`n"
    }
    Set-Content -LiteralPath $target -Value $content -NoNewline
    Write-Step "updated  AGENTS.md (refreshed Truss block; backup: $($backup.Substring($script:TargetDir.Length + 1)))"
    $script:Updated++
}

function Get-TrussReleaseTag {
    if ($env:TRUSS_CORE_RELEASE_TAG) { return $env:TRUSS_CORE_RELEASE_TAG.Trim() }
    if ($script:Source.Mode -eq "local") {
        $path = Join-Path $script:Source.Root "scripts/truss-release-tag"
        if (!(Test-Path $path)) { Fail "Truss core release tag is missing: $path" }
        return ((Get-Content -LiteralPath $path | Where-Object { $_ -match "\S" -and $_ -notmatch "^\s*#" } | Select-Object -First 1) -as [string]).Trim()
    }
    try {
        $text = Read-RemoteText "$script:CoreSourceBaseUrl/scripts/truss-release-tag"
        return (($text -split "`n" | Where-Object { $_ -match "\S" -and $_ -notmatch "^\s*#" } | Select-Object -First 1) -as [string]).Trim()
    } catch {
        Fail "Truss core release tag is missing"
    }
}

function Merge-CoreGitignore([string]$Target) {
    $rules = @("# Truss core maintenance binary", ".truss-core/bin/truss", ".truss-core/bin/truss.exe")
    $existing = if (Test-Path $Target) { Get-Content -LiteralPath $Target } else { @() }
    $missing = @($rules | Where-Object { $existing -notcontains $_ })
    if ($missing.Count -eq 0) {
        Write-Step "skip     .gitignore (Truss core binary rules already present)"
        return
    }
    if ($DryRun) {
        Write-Step "update   .gitignore (append Truss core binary rules)"
        return
    }
    $prefix = if ((Test-Path $Target) -and ((Get-Item $Target).Length -gt 0)) { "`n" } else { "" }
    Add-Content -LiteralPath $Target -Value ($prefix + (($missing -join "`n") + "`n")) -NoNewline
    Write-Step "updated  .gitignore (appended Truss core binary rules)"
}

function Assert-NotReparsePoint([string]$Path, [string]$Label) {
    $item = Get-Item -LiteralPath $Path -Force -ErrorAction SilentlyContinue
    if ($null -ne $item -and (($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0)) {
        Fail "refusing symlink or reparse point for $Label"
    }
}

function Install-TrussCore {
    $platform = if ($env:TRUSS_CORE_CLI_PLATFORM) { $env:TRUSS_CORE_CLI_PLATFORM } else { "windows-x64" }
    if ($platform -ne "windows-x64") { Fail "Unsupported Windows Truss core platform: $platform" }
    $stageRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("truss-core-" + [guid]::NewGuid().ToString("N"))
    $staged = Join-Path $stageRoot "truss.exe"
    $command = if (Test-Path (Join-Path $script:TargetDir ".truss-core/manifest.json")) { "update" } else { "install" }
    $pendingVersion = $null
    $sessionPath = Join-Path $script:TargetDir ".truss-core/update/session.json"
    if ($command -eq "update" -and (Test-Path $sessionPath)) {
        $pendingVersion = (Get-Content -LiteralPath $sessionPath -Raw | ConvertFrom-Json).to_version
        if ([string]::IsNullOrWhiteSpace($pendingVersion)) { Fail "could not read pending Truss update version" }
    }
    New-Item -ItemType Directory -Force -Path $stageRoot | Out-Null
    try {
        if ($env:TRUSS_CORE_BINARY) {
            if (!(Test-Path $env:TRUSS_CORE_BINARY)) { Fail "TRUSS_CORE_BINARY does not exist: $env:TRUSS_CORE_BINARY" }
            Copy-Item -LiteralPath $env:TRUSS_CORE_BINARY -Destination $staged
        } elseif ($script:Source.Mode -eq "local") {
            & cargo build --quiet --manifest-path (Join-Path $script:Source.Root "Cargo.toml") -p truss --locked
            if ($LASTEXITCODE -ne 0) { Fail "could not build the local Rust truss CLI" }
            $cargoTargetRoot = if ([string]::IsNullOrWhiteSpace($env:CARGO_TARGET_DIR)) {
                Join-Path $script:Source.Root "target"
            } elseif ([System.IO.Path]::IsPathRooted($env:CARGO_TARGET_DIR)) {
                $env:CARGO_TARGET_DIR
            } else {
                Join-Path (Get-Location).Path $env:CARGO_TARGET_DIR
            }
            Copy-Item -LiteralPath (Join-Path $cargoTargetRoot "debug/truss.exe") -Destination $staged
        } else {
            $releaseTag = if ($pendingVersion) { "truss-v$pendingVersion" } else { Get-TrussReleaseTag }
            if ($releaseTag -notmatch '^truss-v[0-9]+\.[0-9]+\.[0-9]+(?:[-.][A-Za-z0-9]+)*$') { Fail "invalid Truss core release tag: $releaseTag" }
            $baseUrl = if ($env:TRUSS_CORE_CLI_BASE_URL) { $env:TRUSS_CORE_CLI_BASE_URL.TrimEnd("/") } else {
                if (!$env:TRUSS_RELEASE_REPO) { Fail "set TRUSS_RELEASE_REPO (owner/name) or TRUSS_CORE_CLI_BASE_URL for a remote core download" }
                "https://github.com/$($env:TRUSS_RELEASE_REPO)/releases/download/$releaseTag"
            }
            $binaryUrl = "$baseUrl/truss-windows-x64.exe"
            $checksumUrl = "$binaryUrl.sha256"
            $checksum = "$staged.sha256"
            if ($binaryUrl.StartsWith("file://")) {
                Copy-Item -LiteralPath ([uri]$binaryUrl).LocalPath -Destination $staged
                Copy-Item -LiteralPath ([uri]$checksumUrl).LocalPath -Destination $checksum
            } else {
                Invoke-WebRequest -UseBasicParsing -Uri $binaryUrl -OutFile $staged
                Invoke-WebRequest -UseBasicParsing -Uri $checksumUrl -OutFile $checksum
            }
            $expected = ((Get-Content -LiteralPath $checksum -Raw) -split "\s+")[0].ToLowerInvariant()
            $actual = (Get-FileHash -Algorithm SHA256 -LiteralPath $staged).Hash.ToLowerInvariant()
            if ([string]::IsNullOrWhiteSpace($expected) -or $expected -ne $actual) { Fail "Checksum mismatch for truss-windows-x64.exe: expected $expected, got $actual" }
            $reportedVersion = ((& $staged --version) -split "\s+")[-1]
            if ($LASTEXITCODE -ne 0 -or $reportedVersion -ne $releaseTag.Substring(9)) {
                Fail "Truss core release identity mismatch: tag=$($releaseTag.Substring(9)), binary=$reportedVersion"
            }
        }

        $runner = $staged
        $arguments = @($command, "--directory", $script:TargetDir)
        if ($command -eq "update") { $arguments += "--candidate" }
        if ($pendingVersion) { $arguments += "--continue" }
        if ($DryRun) { $arguments += "--dry-run" }
        $target = $null
        $targetTemp = $null
        if (!$DryRun) {
            $target = Join-Path $script:TargetDir ".truss-core/bin/truss.exe"
            Assert-NotReparsePoint (Join-Path $script:TargetDir ".truss-core") ".truss-core"
            Assert-NotReparsePoint (Join-Path $script:TargetDir ".truss-core/bin") ".truss-core/bin directory"
            Assert-NotReparsePoint $target "repository Truss executable"
            New-Item -ItemType Directory -Force -Path (Split-Path -Parent $target) | Out-Null
            $targetTemp = Join-Path (Split-Path -Parent $target) (".truss." + [guid]::NewGuid().ToString("N") + ".tmp")
            Copy-Item -LiteralPath $staged -Destination $targetTemp
            if (Test-Path $target) {
                $backup = Join-Path $script:BackupDir ".truss-core/bin/truss.exe"
                New-Item -ItemType Directory -Force -Path (Split-Path -Parent $backup) | Out-Null
                Copy-Item -LiteralPath $target -Destination $backup -Force
            }
        }
        & $runner @arguments
        $commandStatus = $LASTEXITCODE
        if ($commandStatus -eq 2 -and !$DryRun) {
            $retained = Join-Path $script:TargetDir ".truss-core/update-candidate/truss.exe"
            Assert-NotReparsePoint (Join-Path $script:TargetDir ".truss-core") ".truss-core"
            Assert-NotReparsePoint (Join-Path $script:TargetDir ".truss-core/update-candidate") "retained candidate directory"
            Assert-NotReparsePoint $retained "retained update candidate"
            New-Item -ItemType Directory -Force -Path (Split-Path -Parent $retained) | Out-Null
            Copy-Item -LiteralPath $staged -Destination $retained -Force
        }
        if ($commandStatus -eq 0 -and !$DryRun) {
            if (Test-Path $target) {
                [System.IO.File]::Replace($targetTemp, $target, $null)
            } else {
                Move-Item -LiteralPath $targetTemp -Destination $target
            }
            Remove-Item -LiteralPath (Join-Path $script:TargetDir ".truss-core/update-candidate") -Recurse -Force -ErrorAction SilentlyContinue
            Merge-CoreGitignore (Join-Path $script:TargetDir ".gitignore")
            Write-Step "installed .truss-core/bin/truss.exe ($platform)"
        } elseif ($targetTemp) {
            Remove-Item -LiteralPath $targetTemp -Force -ErrorAction SilentlyContinue
        }
        if ($commandStatus -eq 2) { Fail "Truss core update needs resolution; edit .truss-core/update/resolved/, then rerun this installer or truss update --continue" }
        if ($commandStatus -ne 0) { Fail "truss $command failed with exit code $commandStatus" }
    } finally {
        Remove-Item -LiteralPath $stageRoot -Recurse -Force -ErrorAction SilentlyContinue
    }
}

# ---------------------------------------------------------------------------
# Add-on installation: delegated to the Truss CLI, never copied.
#
# The membership manifests stay the owner of what an add-on contains: their
# non-comment lines are exactly the payload path set staged for the CLI. The
# CLI is invoked once per requested add-on and owns planning, preservation,
# update, adoption, and conflict staging against the recorded baseline, so
# every add-on install writes `.truss-core/addons.json` and the
# `.truss-core/base-addons/<name>/` copies of the payload bytes. No add-on path
# is written by a direct copy path in this installer.
# ---------------------------------------------------------------------------

function Test-ImmutableSourceRef([string]$Ref) {
    if ($Ref -match '^truss-v[0-9]+\.[0-9]+\.[0-9]+(?:[-.][A-Za-z0-9]+)*$') { return $true }
    if ($Ref -match '^([0-9a-f]{40}|[0-9a-f]{64})$') { return $true }
    return $false
}

function Assert-ImmutableSourceRef([string]$Ref, [string]$Label) {
    if (!(Test-ImmutableSourceRef $Ref)) {
        Fail "$Label did not resolve to an immutable --source-ref (got '$Ref'); an add-on records exactly a truss-vX.Y.Z release tag or a full commit SHA, and the installer stops instead of copying files"
    }
}

function Assert-GitAvailable {
    if (!(Get-Command git -ErrorAction SilentlyContinue)) {
        Fail "git is required to resolve an immutable --source-ref for a local Truss source checkout"
    }
}

# An add-on is installed the first time and updated once it is recorded. The
# record is read from the CLI-owned state file, never from the workspace.
function Test-AddOnRecorded([string]$Name) {
    $record = Join-Path $script:TargetDir ".truss-core/addons.json"
    if (!(Test-Path $record)) { return $false }
    return [bool](Select-String -LiteralPath $record -SimpleMatch -Quiet -Pattern ('"name": "' + $Name + '"'))
}

function Resolve-LocalAddOnSourceRef {
    Assert-GitAvailable
    $root = $script:Source.Root
    $head = @(& git -C $root rev-parse --verify HEAD 2>$null)
    if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace(($head -join ""))) {
        Fail "the local Truss source at $root is not a git checkout, so no immutable --source-ref can be resolved; install add-ons from a git checkout, or from a source base URL pinned to the released ref, and never from a direct copy"
    }
    $head = ($head -join "").Trim()
    $tag = $null
    $tagPath = Join-Path $root "scripts/truss-release-tag"
    if (Test-Path $tagPath) {
        $tag = ((Get-Content -LiteralPath $tagPath | Where-Object { $_ -match "\S" -and $_ -notmatch "^\s*#" } | Select-Object -First 1) -as [string])
        if ($tag) { $tag = $tag.Trim() }
    }
    if ($tag) {
        $pointsAt = @(& git -C $root tag --points-at HEAD 2>$null)
        if (($pointsAt | Where-Object { $_.Trim() -eq $tag }).Count -gt 0) {
            $script:AddOnSourceRef = $tag
            return
        }
    }
    $script:AddOnSourceRef = $head
}

# A raw source base URL is a released source only when the URL itself pins the
# released ref: the last URL segment must be the tag that the tag file at that
# same URL declares. A floating URL (a branch) has no immutable ref to record,
# so the installer stops instead of recording a tag for unreleased bytes.
function Resolve-RemoteAddOnSourceRef {
    $tagFile = "scripts/truss-release-tag"
    $urlTag = ($script:SourceBaseUrl.TrimEnd("/") -split "/")[-1]
    Assert-ImmutableSourceRef $urlTag "the raw source base URL ($script:SourceBaseUrl)"
    $tagText = Read-RemoteText "$script:SourceBaseUrl/$tagFile"
    $tag = (($tagText -split "\r?\n" | Where-Object { $_ -match "\S" -and $_ -notmatch "^\s*#" } | Select-Object -First 1) -as [string])
    if ($tag) { $tag = $tag.Trim() }
    if ($tag -ne $urlTag) {
        Fail "the raw source base URL pins $urlTag but $tagFile declares '$tag'; install add-ons from a base URL pinned to the released ref"
    }
    $script:AddOnSourceRef = $tag
}

function Resolve-AddOnSourceRef {
    if ($script:AddOnSourceRef) { return }
    if ($script:Source.Mode -eq "local") {
        Resolve-LocalAddOnSourceRef
    } else {
        Resolve-RemoteAddOnSourceRef
    }
    Assert-ImmutableSourceRef $script:AddOnSourceRef "the Truss source"
}

# The core version the payload was acquired with: the released ref's version,
# the installed CLI's reported version, or the local source checkout's own
# crate version when no CLI is installed yet (a dry run).
function Resolve-AddOnSourceCoreVersion {
    if ($script:AddOnSourceCoreVersion) { return }
    if ($script:AddOnSourceRef.StartsWith("truss-v")) {
        $script:AddOnSourceCoreVersion = $script:AddOnSourceRef.Substring(7)
        return
    }
    $runner = Join-Path $script:TargetDir ".truss-core/bin/truss.exe"
    if (Test-Path $runner) {
        $reported = ((& $runner --version) -split "\s+")[-1]
        if ($LASTEXITCODE -eq 0 -and ![string]::IsNullOrWhiteSpace($reported)) {
            $script:AddOnSourceCoreVersion = $reported.Trim()
            return
        }
    }
    $cargoToml = Join-Path $script:Source.Root "crates/truss/Cargo.toml"
    if (!(Test-Path $cargoToml)) {
        Fail "could not resolve the source core version for the add-on install: neither an installed Truss CLI nor $cargoToml is available"
    }
    $line = Get-Content -LiteralPath $cargoToml | Where-Object { $_ -match '^\s*version\s*=' } | Select-Object -First 1
    if ($line -match '"([^"]+)"') {
        $script:AddOnSourceCoreVersion = $Matches[1]
        return
    }
    Fail "could not read the Truss source core version from $cargoToml"
}

function Assert-LocalAddOnPayloadCommitted([string]$Name, [string]$Manifest) {
    $root = $script:Source.Root
    $paths = @(Get-PayloadFiles $Manifest)
    if ($paths.Count -eq 0) {
        Fail "the $Name add-on payload manifest $Manifest lists no files"
    }
    Assert-GitAvailable
    $untracked = @(& git -C $root ls-files --others --exclude-standard -- $paths)
    if ($LASTEXITCODE -ne 0) {
        Fail "could not inspect the local Truss source checkout at $root with git"
    }
    if (($untracked | Where-Object { $_ -match "\S" }).Count -gt 0) {
        Fail "the $Name add-on payload is not committed in $root; an immutable --source-ref must describe the bytes that are installed, so this installer stops instead of recording one"
    }
    & git -C $root diff --quiet HEAD -- $paths
    if ($LASTEXITCODE -ne 0) {
        Fail "the $Name add-on payload has uncommitted changes in $root; commit or discard them first, because an immutable --source-ref must describe the bytes that are installed and no dirty provenance format is invented here"
    }
}

# The manifest stays the membership owner: its lines are read here, and every
# path it lists is staged for the CLI.
function Stage-AddOnPayload([string]$Name, [string]$Manifest) {
    $script:AddOnStagedPayload = $null
    $script:AddOnStagedManifest = $null
    if ($script:Source.Mode -eq "local") {
        Assert-LocalAddOnPayloadCommitted $Name $Manifest
        $script:AddOnStagedPayload = $script:Source.Root
        $script:AddOnStagedManifest = Join-Path $script:Source.Root $Manifest
        return
    }
    $stageRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("truss-addon-" + [guid]::NewGuid().ToString("N"))
    $script:AddOnStageRoot = $stageRoot
    $script:AddOnStagedPayload = Join-Path $stageRoot "payload"
    $script:AddOnStagedManifest = Join-Path $stageRoot $Manifest
    New-Item -ItemType Directory -Force -Path $script:AddOnStagedPayload | Out-Null
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $script:AddOnStagedManifest) | Out-Null
    Set-Content -LiteralPath $script:AddOnStagedManifest -Value (Read-PayloadManifest $Manifest)
    foreach ($relative in (Get-PayloadFiles $Manifest)) {
        $target = Join-Path $script:AddOnStagedPayload $relative
        New-Item -ItemType Directory -Force -Path (Split-Path -Parent $target) | Out-Null
        $url = "$script:SourceBaseUrl/$($relative -replace '\\','/')"
        if ($url.StartsWith("file://")) {
            Copy-Item -LiteralPath ([uri]$url).LocalPath -Destination $target -Force
        } else {
            Invoke-WebRequest -UseBasicParsing -Uri $url -OutFile $target
        }
    }
}

function Remove-AddOnStageRoot {
    if ($script:AddOnStageRoot) {
        Remove-Item -LiteralPath $script:AddOnStageRoot -Recurse -Force -ErrorAction SilentlyContinue
        $script:AddOnStageRoot = $null
    }
}

# Resolve the immutable ref, and prove that a local checkout really carries the
# payload that ref names, before any mutation. A mutable ref or a checkout whose
# add-on payload is not committed stops the installer before the core install
# writes anything.
function Invoke-AddOnPreflight {
    if (!$WithEngineeringWisdom -and !$WithDelivery -and !$WithPlanning) { return }
    Resolve-AddOnSourceRef
    if ($script:Source.Mode -ne "local") { return }
    if ($WithEngineeringWisdom) {
        Assert-LocalAddOnPayloadCommitted "engineering-wisdom" $script:EngineeringWisdomPayloadManifest
    }
    if ($WithDelivery) {
        Assert-LocalAddOnPayloadCommitted "delivery" $script:DeliveryPayloadManifest
    }
    if ($WithPlanning) {
        Assert-LocalAddOnPayloadCommitted "planning" $script:PlanningPayloadManifest
    }
}

# One CLI invocation per add-on. `install` for a new add-on, `update` for one
# already recorded; the CLI then plans, preserves, updates, adopts, or stages a
# conflict, and writes the record and baseline. -Merge and -Force never reach an
# add-on: there is no installer-side skip or overwrite path left.
function Install-AddOn([string]$Name, [string]$Manifest) {
    $operation = "install"
    $runner = Join-Path $script:TargetDir ".truss-core/bin/truss.exe"
    $dryPreview = $false

    Stage-AddOnPayload $Name $Manifest
    Resolve-AddOnSourceRef
    Resolve-AddOnSourceCoreVersion
    if (Test-AddOnRecorded $Name) { $operation = "update" }

    $coreManifest = Join-Path $script:TargetDir ".truss-core/manifest.json"
    if ($DryRun -and !(Test-Path $coreManifest)) {
        # A dry run installs no core state for the CLI to validate. Nothing is
        # copied either way; the invocation a real run would make is reported.
        $dryPreview = $true
    } elseif (!(Test-Path $runner)) {
        Fail "the Truss CLI is required to install the $Name add-on but is not available at $runner; run a full install first (a dry run installs no CLI and no core state)"
    }

    $arguments = @("addon", $operation, "--name", $Name, "--manifest", $script:AddOnStagedManifest, "--source", $script:AddOnStagedPayload, "--source-ref", $script:AddOnSourceRef, "--source-core-version", $script:AddOnSourceCoreVersion, "--directory", $script:TargetDir)
    if ($DryRun) { $arguments += "--dry-run" }
    $invocation = "$runner " + ($arguments -join " ")

    if ($dryPreview) {
        Write-Step "${Name}: dry run would delegate to the Truss CLI (no core state in the target yet)"
        Write-Step "  $invocation"
        return
    }

    Write-Step "${Name}: $operation via the Truss CLI"
    Write-Step "  $invocation"
    & $runner @arguments
    $status = $LASTEXITCODE
    if ($status -eq 2 -and $DryRun) {
        Write-Step "${Name}: the Truss CLI preview reports conflicts; the dry run changed nothing"
        return
    }
    if ($status -eq 2) {
        Fail "the $Name add-on update stopped on a conflict and staged a resolution; edit the files under .truss-core/addon-update/$Name/resolved/, then run: $runner addon continue --name $Name --directory $script:TargetDir"
    }
    if ($status -ne 0) {
        Fail "the Truss CLI failed to $operation the $Name add-on with exit code $status"
    }
}

function Install-EngineeringWisdom {
    if (!$WithEngineeringWisdom) { return }
    Install-AddOn "engineering-wisdom" $script:EngineeringWisdomPayloadManifest
}

function Install-Delivery {
    if (!$WithDelivery) { return }
    Install-AddOn "delivery" $script:DeliveryPayloadManifest
}

function Install-Planning {
    if (!$WithPlanning) { return }
    Install-AddOn "planning" $script:PlanningPayloadManifest
}

$script:Created = 0
$script:Updated = 0
$script:Skipped = 0
$script:AddOnSourceRef = $null
$script:AddOnSourceCoreVersion = $null
$script:AddOnStagedPayload = $null
$script:AddOnStagedManifest = $null
$script:AddOnStageRoot = $null
$script:Source = Get-SourceMode
$script:SourceBaseUrl = if ($env:TRUSS_SOURCE_BASE_URL) { $env:TRUSS_SOURCE_BASE_URL.TrimEnd("/") } else { "" }
$script:CoreSourceBaseUrl = if ($env:TRUSS_CORE_SOURCE_BASE_URL) { $env:TRUSS_CORE_SOURCE_BASE_URL.TrimEnd("/") } else { "" }
$script:PayloadManifest = "scripts/truss-install-files.txt"
$script:EngineeringWisdomPayloadManifest = "scripts/engineering-wisdom-install-files.txt"
$script:DeliveryPayloadManifest = "scripts/delivery-install-files.txt"
$script:PlanningPayloadManifest = "scripts/plan-install-files.txt"
$script:TargetDir = Resolve-TargetPath $Directory
$script:BackupDir = Join-Path $script:TargetDir (".truss-backup/" + (Get-Date -Format "yyyyMMddHHmmss"))
$script:ConflictAction = "install"

if ($Merge -and $Override) {
    Fail "Use only one of -Merge or -Override"
}

if (!$DryRun -and !(Test-Path $script:TargetDir)) {
    New-Item -ItemType Directory -Force -Path $script:TargetDir | Out-Null
}

$protectedPaths = @("AGENTS.md", "docs")
$conflicts = $protectedPaths | Where-Object { Test-Path (Join-Path $script:TargetDir $_) }
if ($conflicts.Count -gt 0) {
    if ($Merge) {
        $script:ConflictAction = "merge"
        Write-Step "Continuing with merge. Existing files will be skipped."
    } elseif ($Override) {
        $script:ConflictAction = "override"
        foreach ($protected in $protectedPaths) {
            $path = Join-Path $script:TargetDir $protected
            if (!(Test-Path $path)) { continue }
            if ($DryRun) {
                Write-Step "override $protected (backup first)"
            } else {
                New-Item -ItemType Directory -Force -Path $script:BackupDir | Out-Null
                Move-Item -LiteralPath $path -Destination (Join-Path $script:BackupDir $protected)
                Write-Step "removed  $protected (backup: $($script:BackupDir.Substring($script:TargetDir.Length + 1))/$protected)"
            }
        }
    } elseif ($Yes) {
        Fail "target already contains protected Truss paths: $($conflicts -join ', '). Use -Merge or -Override."
    } else {
        Write-Host "Warning: target already contains protected Truss paths: $($conflicts -join ', ')"
        $choice = Read-Host "Choose Merge, Override, or Stop [Stop]"
        switch -Regex ($choice) {
            "^(m|merge)$" { $script:ConflictAction = "merge"; Write-Step "Continuing with merge. Existing files will be skipped." }
            "^(o|override)$" {
                $script:ConflictAction = "override"
                foreach ($protected in $protectedPaths) {
                    $path = Join-Path $script:TargetDir $protected
                    if (Test-Path $path) {
                        New-Item -ItemType Directory -Force -Path $script:BackupDir | Out-Null
                        Move-Item -LiteralPath $path -Destination (Join-Path $script:BackupDir $protected)
                    }
                }
            }
            default { Fail "installation stopped" }
        }
    }
}

if ($script:Source.Mode -eq "local") {
    Write-Step "Truss source: $($script:Source.Root)"
} else {
    if (!$script:SourceBaseUrl) { Fail "remote install requires TRUSS_SOURCE_BASE_URL (raw source base URL)" }
    if (!$script:CoreSourceBaseUrl) { Fail "remote install requires TRUSS_CORE_SOURCE_BASE_URL (raw source base URL)" }
    Write-Step "Truss source: $script:SourceBaseUrl"
}
Write-Step "Truss profile: core"
if ($WithEngineeringWisdom) {
    Write-Step "Engineering wisdom: included (explicit opt-in)"
} else {
    Write-Step "Engineering wisdom: excluded"
}
if ($WithDelivery) {
    Write-Step "Delivery add-on: included (explicit opt-in)"
} else {
    Write-Step "Delivery add-on: excluded"
}
if ($WithPlanning) {
    Write-Step "Planning add-on: included (explicit opt-in)"
} else {
    Write-Step "Planning add-on: excluded"
}
Write-Step "Target project: $script:TargetDir"

Invoke-AddOnPreflight

Install-TrussCore

try {
    Install-EngineeringWisdom
    Install-Delivery
    Install-Planning
} finally {
    Remove-AddOnStageRoot
}
Refresh-AgentShimFile

Write-Step ""
Write-Step "Done. Created: $script:Created, updated: $script:Updated, skipped: $script:Skipped."
if ($script:Skipped -gt 0 -and !$Force) {
    Write-Step "Existing files were left untouched. Re-run with -Force to overwrite with backups."
}
if ($Force -and $script:Updated -gt 0 -and !$DryRun) {
    Write-Step "Backups were written to: $script:BackupDir"
}
