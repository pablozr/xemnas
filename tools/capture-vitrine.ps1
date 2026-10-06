<#
.SYNOPSIS
    Captures the five scenes of the showcase route (docs/design/VISUAL-IDENTITY.md)
    without focus, clicks or keys.

.DESCRIPTION
    Overview, Review, Decision, Map (blocks and graph) and Context, one image
    each, in one palette and one window size, through capture-background.ps1.
    Demo data only; set XEMNAS_DEMO_SCALE first for the large-volume take.

.EXAMPLE
    powershell -File tools\capture-vitrine.ps1 -Theme charcoal -Compact -Prefix take2
#>
[CmdletBinding()]
param(
    [string]$ExePath = "target\release\xemnas.exe",
    [ValidateSet('quiet', 'charcoal', 'organization', 'moss', 'midnight')][string]$Theme = 'quiet',
    [switch]$Compact,
    [string]$Prefix = 'vitrine'
)

$ErrorActionPreference = 'Stop'
$here = Split-Path -Parent $MyInvocation.MyCommand.Path
$scenes = @(
    @{ Route = 'overview';  Name = '1-visao' },
    @{ Route = 'review';    Name = '2-revisao' },
    @{ Route = 'decisions'; Name = '3-decisao' },
    @{ Route = 'map';       Name = '4a-mapa-blocos' },
    @{ Route = 'map:graph'; Name = '4b-mapa-grafo' },
    @{ Route = 'context';   Name = '5-contexto' }
)
foreach ($scene in $scenes) {
    $args = @('-NoProfile', '-File', (Join-Path $here 'capture-background.ps1'),
        '-Route', $scene.Route, '-Name', "$Prefix-$($scene.Name)",
        '-ExePath', $ExePath, '-Theme', $Theme)
    if ($Compact) { $args += '-Compact' }
    powershell @args | Select-Object -Last 1
}
