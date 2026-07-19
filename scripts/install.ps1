[CmdletBinding()]
param(
    [string]$Version = "latest",
    [string]$InstallDir = "$env:LOCALAPPDATA\Programs\btop-win"
)

$ErrorActionPreference = "Stop"
$repo = "weepwood/btop-win"

if ($Version -eq "latest") {
    $release = Invoke-RestMethod "https://api.github.com/repos/$repo/releases/latest"
} else {
    $tag = if ($Version.StartsWith("v")) { $Version } else { "v$Version" }
    $release = Invoke-RestMethod "https://api.github.com/repos/$repo/releases/tags/$tag"
}

$asset = $release.assets | Where-Object { $_.name -match '^btop-win-.*-x86_64-pc-windows-msvc\.zip$' } | Select-Object -First 1
if (-not $asset) {
    throw "No Windows x64 release archive was found for $($release.tag_name)."
}

$temp = Join-Path $env:TEMP $asset.name
Invoke-WebRequest $asset.browser_download_url -OutFile $temp
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
Expand-Archive -Path $temp -DestinationPath $InstallDir -Force
Remove-Item $temp -Force

$currentUserPath = [Environment]::GetEnvironmentVariable("Path", "User")
$pathEntries = @($currentUserPath -split ';' | Where-Object { $_ })
if ($pathEntries -notcontains $InstallDir) {
    $newPath = (($pathEntries + $InstallDir) -join ';')
    [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
    Write-Host "Added $InstallDir to the user PATH. Open a new terminal before running btop-win."
}

Write-Host "Installed btop-win $($release.tag_name) to $InstallDir"
