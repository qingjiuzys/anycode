#!/usr/bin/env pwsh
# anyCode installer for Windows PowerShell
#
# Canonical repo: qingjiuzys/anycode
# One-liner:
#   irm https://raw.githubusercontent.com/qingjiuzys/anycode/main/scripts/install.ps1 | iex

[CmdletBinding()]
param(
  [string]$Repo = $env:ANYCODE_GITHUB_REPO,
  [string]$Version = $(if ($env:ANYCODE_VERSION) { $env:ANYCODE_VERSION } else { "latest" }),
  [string]$BinDir = $env:ANYCODE_INSTALL_BIN,
  [ValidateSet("binary", "auto", "source")]
  [string]$Method = "binary",
  [string]$SourceDir = "",
  [switch]$DryRun,
  [switch]$Setup,
  [switch]$NoSetup,
  [switch]$Quiet
)

$ErrorActionPreference = "Stop"

if (-not [System.Runtime.InteropServices.RuntimeInformation]::IsOSPlatform([System.Runtime.InteropServices.OSPlatform]::Windows)) {
  throw "[anycode-install] scripts/install.ps1 is for Windows only. Use scripts/install.sh on macOS/Linux."
}

function Write-Info([string]$msg) { Write-Host $msg }
function Write-Warn([string]$msg) { Write-Warning "[anycode-install] $msg" }
function Fail([string]$msg) { throw "[anycode-install] $msg" }

function Invoke-Download([string]$url, [string]$outFile) {
  $oldPref = $ProgressPreference
  try {
    if ($Quiet -or $env:ANYCODE_INSTALL_QUIET -eq "1") {
      $ProgressPreference = "SilentlyContinue"
    }
    else {
      $ProgressPreference = "Continue"
    }
    Invoke-WebRequest -Uri $url -OutFile $outFile -UseBasicParsing
  }
  finally {
    $ProgressPreference = $oldPref
  }
}

function Normalize-Version([string]$v) {
  if ($v -eq "latest") { return "latest" }
  if ($v.StartsWith("v")) { return $v }
  return "v$v"
}

function Resolve-LatestTag([string]$repo) {
  $api = "https://api.github.com/repos/$repo/releases/latest"
  $release = Invoke-RestMethod -Uri $api -UseBasicParsing
  if (-not $release.tag_name) { Fail "Cannot resolve latest release tag from $api" }
  return [string]$release.tag_name
}

function Detect-TargetTriple() {
  $arch = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture
  switch ($arch.ToString()) {
    "X64" { return "x86_64-pc-windows-msvc" }
    "Arm64" { return "aarch64-pc-windows-msvc" }
    default { Fail "Unsupported Windows architecture: $arch" }
  }
}

function Ensure-BinDir() {
  if ([string]::IsNullOrWhiteSpace($script:BinDir)) {
    $script:BinDir = Join-Path $env:LOCALAPPDATA "Programs\anycode\bin"
  }
  if ($DryRun) {
    Write-Info "[dry-run] mkdir -Force `"$script:BinDir`""
    return
  }
  New-Item -ItemType Directory -Force -Path $script:BinDir | Out-Null
}

function Ensure-PathContains([string]$dir) {
  $currentUserPath = [Environment]::GetEnvironmentVariable("Path", "User")
  $segments = @()
  if ($currentUserPath) {
    $segments = $currentUserPath.Split(";") | ForEach-Object { $_.Trim() } | Where-Object { $_ -ne "" }
  }
  $exists = $segments | Where-Object { $_.TrimEnd("\") -ieq $dir.TrimEnd("\") }
  if ($exists) {
    if (-not ($env:Path.Split(";") | ForEach-Object { $_.TrimEnd("\") } | Where-Object { $_ -ieq $dir.TrimEnd("\") })) {
      $env:Path = "$dir;$env:Path"
    }
    return
  }

  if ($DryRun) {
    Write-Info "[dry-run] Add to user PATH: $dir"
    return
  }

  $newUserPath = if ([string]::IsNullOrWhiteSpace($currentUserPath)) { $dir } else { "$currentUserPath;$dir" }
  [Environment]::SetEnvironmentVariable("Path", $newUserPath, "User")
  if (-not ($env:Path.Split(";") | ForEach-Object { $_.TrimEnd("\") } | Where-Object { $_ -ieq $dir.TrimEnd("\") })) {
    $env:Path = "$dir;$env:Path"
  }
  Write-Info "Added to user PATH: $dir"
}

function Install-FromBinary([string]$repo, [string]$versionArg, [string]$target) {
  $tag = if ($versionArg -eq "latest") { Resolve-LatestTag $repo } else { Normalize-Version $versionArg }
  $tmp = Join-Path ([System.IO.Path]::GetTempPath()) ("anycode-install-" + [Guid]::NewGuid().ToString("N"))
  $zip = Join-Path $tmp "anycode.zip"
  $asset = "anycode-$target.zip"
  $url = "https://github.com/$repo/releases/download/$tag/$asset"
  Write-Warn "Downloading: $url"

  if ($DryRun) {
    Write-Info "[dry-run] download $url -> $zip"
    return $true
  }

  New-Item -ItemType Directory -Force -Path $tmp | Out-Null
  try {
    Invoke-Download $url $zip
    Expand-Archive -Path $zip -DestinationPath $tmp -Force
    $exePath = Join-Path $tmp "anycode.exe"
    if (-not (Test-Path $exePath)) { Fail "Archive missing top-level anycode.exe" }
    $dest = Join-Path $script:BinDir "anycode.exe"
    Copy-Item -Force $exePath $dest
    Write-Info "Installed: $dest"
    return $true
  }
  catch {
    Write-Warn "Binary install failed: $($_.Exception.Message)"
    return $false
  }
  finally {
    Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
  }
}

function Install-FromGit([string]$repo) {
  if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Fail "cargo not found. Install Rust from https://rustup.rs then retry."
  }
  $url = "https://github.com/$repo.git"
  if ($DryRun) {
    Write-Info "[dry-run] cargo install --locked --git $url anycode --root `"$script:BinDir\..`" --force"
    return
  }
  $root = Split-Path -Parent $script:BinDir
  cargo install --locked --git $url anycode --root $root --force
  Write-Info "Installed: $(Join-Path $script:BinDir "anycode.exe")"
}

function Install-FromSourceDir([string]$dir) {
  $cli = Join-Path $dir "crates\cli"
  if (-not (Test-Path (Join-Path $cli "Cargo.toml"))) {
    Fail "--SourceDir must point to repo root containing crates/cli (got $dir)"
  }
  if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Fail "cargo not found. Install Rust from https://rustup.rs then retry."
  }
  $root = Split-Path -Parent $script:BinDir
  if ($DryRun) {
    Write-Info "[dry-run] cargo install --locked --path `"$cli`" --root `"$root`" --force"
    return
  }
  cargo install --locked --path $cli --root $root --force
  Write-Info "Installed: $(Join-Path $script:BinDir "anycode.exe")"
}

if ([string]::IsNullOrWhiteSpace($Repo) -and [string]::IsNullOrWhiteSpace($SourceDir)) {
  Fail "Set -Repo OWNER/REPO (or ANYCODE_GITHUB_REPO), or pass -SourceDir for local install."
}

Ensure-BinDir
$target = Detect-TargetTriple

if (-not [string]::IsNullOrWhiteSpace($SourceDir)) {
  Install-FromSourceDir (Resolve-Path $SourceDir).Path
}
else {
  switch ($Method) {
    "binary" {
      if (-not (Install-FromBinary $Repo $Version $target)) {
        Fail "Binary install failed. Check release assets for tag."
      }
    }
    "source" { Install-FromGit $Repo }
    "auto" {
      if (-not (Install-FromBinary $Repo $Version $target)) {
        Write-Warn "Release binary not found; falling back to cargo install --git."
        Install-FromGit $Repo
      }
    }
  }
}

Ensure-PathContains $BinDir

$runSetup = $true
if ($NoSetup -or $env:ANYCODE_NO_SETUP -eq "1") {
  $runSetup = $false
}
if ($Setup) {
  $runSetup = $true
}

if ($runSetup) {
  if ($DryRun) {
    Write-Info "[dry-run] anycode setup"
  }
  else {
    & anycode setup
  }
}
else {
  Write-Info "Next: run anycode setup"
}

