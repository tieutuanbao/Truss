[CmdletBinding()]
param(
    [Alias("d")]
    [string]$Directory = $env:TRUSS_TARGET_DIR,
    [Alias("y")]
    [switch]$Yes,
    [switch]$Merge,
    [switch]$WithEngineeringWisdom,
    [switch]$WithDelivery,
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

function Write-SourceFile([string]$Relative, [string]$Target) {
    if ($Relative -eq "AGENTS.md") {
        $block = (Read-SourceText "scripts/agent-truss-block.md").TrimEnd("`r", "`n")
        Set-Content -LiteralPath $Target -Value ("# Agent Instructions`n`n" + $block + "`n") -NoNewline
        return
    }

    if ($script:Source.Mode -eq "local") {
        $source = Join-Path $script:Source.Root $Relative
        if (!(Test-Path $source)) {
            Fail "Source file missing: $source"
        }
        Copy-Item -LiteralPath $source -Destination $Target -Force
        return
    }

    $url = "$script:SourceBaseUrl/$($Relative -replace '\\','/')"
    if ($url.StartsWith("file://")) {
        Copy-Item -LiteralPath ([uri]$url).LocalPath -Destination $Target -Force
    } else {
        Invoke-WebRequest -UseBasicParsing -Uri $url -OutFile $Target
    }
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

function Copy-TrussFile([string]$Relative) {
    $target = Join-Path $script:TargetDir $Relative

    if (Test-Path $target) {
        if ($Force) {
            if ($DryRun) {
                Write-Step "overwrite $Relative (backup first)"
            } else {
                $backup = Join-Path $script:BackupDir $Relative
                New-Item -ItemType Directory -Force -Path (Split-Path -Parent $backup) | Out-Null
                Copy-Item -LiteralPath $target -Destination $backup -Force
                Write-SourceFile $Relative $target
                Write-Step "updated  $Relative (backup: $($backup.Substring($script:TargetDir.Length + 1)))"
            }
            $script:Updated++
        } elseif ($script:ConflictAction -eq "merge") {
            Write-Step "skip     $Relative (merge keeps existing file)"
            $script:Skipped++
        } else {
            Write-Step "skip     $Relative (already exists)"
            $script:Skipped++
        }
        return
    }

    if ($DryRun) {
        Write-Step "create   $Relative"
    } else {
        New-Item -ItemType Directory -Force -Path (Split-Path -Parent $target) | Out-Null
        Write-SourceFile $Relative $target
        Write-Step "created  $Relative"
    }
    $script:Created++
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

function Install-EngineeringWisdom {
    if (!$WithEngineeringWisdom) { return }
    foreach ($file in (Get-PayloadFiles $script:EngineeringWisdomPayloadManifest)) {
        Copy-TrussFile $file
    }
}

function Install-Delivery {
    if (!$WithDelivery) { return }
    foreach ($file in (Get-PayloadFiles $script:DeliveryPayloadManifest)) {
        Copy-TrussFile $file
    }
}

$script:Created = 0
$script:Updated = 0
$script:Skipped = 0
$script:Source = Get-SourceMode
$script:SourceBaseUrl = if ($env:TRUSS_SOURCE_BASE_URL) { $env:TRUSS_SOURCE_BASE_URL.TrimEnd("/") } else { "" }
$script:CoreSourceBaseUrl = if ($env:TRUSS_CORE_SOURCE_BASE_URL) { $env:TRUSS_CORE_SOURCE_BASE_URL.TrimEnd("/") } else { "" }
$script:PayloadManifest = "scripts/truss-install-files.txt"
$script:EngineeringWisdomPayloadManifest = "scripts/engineering-wisdom-install-files.txt"
$script:DeliveryPayloadManifest = "scripts/delivery-install-files.txt"
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
Write-Step "Target project: $script:TargetDir"

Install-TrussCore

Install-EngineeringWisdom
Install-Delivery
Refresh-AgentShimFile

Write-Step ""
Write-Step "Done. Created: $script:Created, updated: $script:Updated, skipped: $script:Skipped."
if ($script:Skipped -gt 0 -and !$Force) {
    Write-Step "Existing files were left untouched. Re-run with -Force to overwrite with backups."
}
if ($Force -and $script:Updated -gt 0 -and !$DryRun) {
    Write-Step "Backups were written to: $script:BackupDir"
}
