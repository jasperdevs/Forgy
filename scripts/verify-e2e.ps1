param(
    [switch]$Network
)

$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $true

$Root = Split-Path -Parent $PSScriptRoot
$ForgeName = if ($env:OS -eq "Windows_NT") { "forge.exe" } else { "forge" }
$Forge = Join-Path $Root "target\release\$ForgeName"
$Work = Join-Path $Root "target\e2e"
$Fixtures = Join-Path $Work "fixtures"
$Out = Join-Path $Work "out"
$Log = Join-Path $Work "verify-e2e.jsonl"

function New-CleanDir($Path) {
    if (Test-Path $Path) {
        Remove-Item -LiteralPath $Path -Recurse -Force
    }
    New-Item -ItemType Directory -Force $Path | Out-Null
}

function Invoke-Step($Name, [scriptblock]$Command) {
    $started = Get-Date
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    try {
        & $Command
        $sw.Stop()
        [pscustomobject]@{
            name = $Name
            ok = $true
            duration_ms = [math]::Round($sw.Elapsed.TotalMilliseconds, 1)
            started_at = $started.ToUniversalTime().ToString("o")
        } | ConvertTo-Json -Compress | Add-Content -LiteralPath $Log
        "ok   {0,-34} {1,8:N1} ms" -f $Name, $sw.Elapsed.TotalMilliseconds
    }
    catch {
        $sw.Stop()
        [pscustomobject]@{
            name = $Name
            ok = $false
            duration_ms = [math]::Round($sw.Elapsed.TotalMilliseconds, 1)
            error = $_.Exception.Message
            started_at = $started.ToUniversalTime().ToString("o")
        } | ConvertTo-Json -Compress | Add-Content -LiteralPath $Log
        throw
    }
}

function Invoke-Native {
    & $args[0] @($args | Select-Object -Skip 1)
    if ($LASTEXITCODE -ne 0) {
        throw "Command failed with exit code ${LASTEXITCODE}: $($args -join ' ')"
    }
}

Push-Location $Root
try {
    cargo build --release --workspace | Out-Host
    New-CleanDir $Work
    New-Item -ItemType Directory -Force $Fixtures, $Out | Out-Null

    Invoke-Step "make video mp4" {
        Invoke-Native ffmpeg -hide_banner -v error -y -f lavfi -i testsrc=size=320x180:rate=24 -f lavfi -i sine=frequency=440:sample_rate=44100 -t 2 -pix_fmt yuv420p (Join-Path $Fixtures "video.mp4") | Out-Host
    }
    Invoke-Step "make video mov" {
        Invoke-Native ffmpeg -hide_banner -v error -y -f lavfi -i testsrc=size=320x180:rate=24 -f lavfi -i sine=frequency=660:sample_rate=44100 -t 2 -pix_fmt yuv420p (Join-Path $Fixtures "video.mov") | Out-Host
    }
    Invoke-Step "make audio wav" {
        Invoke-Native ffmpeg -hide_banner -v error -y -f lavfi -i sine=frequency=880:sample_rate=44100 -t 2 (Join-Path $Fixtures "audio.wav") | Out-Host
    }
    Invoke-Step "make image png" {
        Invoke-Native ffmpeg -hide_banner -v error -y -f lavfi -i color=c=black:s=320x180 "-frames:v" 1 -update 1 (Join-Path $Fixtures "image.png") | Out-Host
    }

    Set-Content -Encoding UTF8 (Join-Path $Fixtures "captions.srt") "1`n00:00:00,000 --> 00:00:01,500`nHello from Forgy.`n"
    Set-Content -Encoding UTF8 (Join-Path $Fixtures "transcript.txt") "Hello from Forgy.`n"

    New-Item -ItemType Directory -Force (Join-Path $Fixtures "videos"), (Join-Path $Fixtures "images"), (Join-Path $Fixtures "lectures") | Out-Null
    Copy-Item (Join-Path $Fixtures "video.mp4") (Join-Path $Fixtures "videos\video.mp4")
    Copy-Item (Join-Path $Fixtures "image.png") (Join-Path $Fixtures "images\image.png")
    Copy-Item (Join-Path $Fixtures "audio.wav") (Join-Path $Fixtures "lectures\audio.wav")

    Invoke-Step "doctor" { & $Forge --output (Join-Path $Out "doctor") --json doctor | Out-Null }
    Invoke-Step "inspect" { & $Forge --json inspect (Join-Path $Fixtures "video.mp4") | Out-Null }
    Invoke-Step "convert mov mp4" { & $Forge --output (Join-Path $Out "convert-mp4") convert (Join-Path $Fixtures "video.mov") --to mp4 | Out-Null }
    Invoke-Step "convert wav mp3" { & $Forge --output (Join-Path $Out "convert-mp3") convert (Join-Path $Fixtures "audio.wav") --to mp3 | Out-Null }
    Invoke-Step "convert png webp" { & $Forge --output (Join-Path $Out "convert-webp") convert (Join-Path $Fixtures "image.png") --to webp | Out-Null }
    Invoke-Step "compress video target" { & $Forge --output (Join-Path $Out "compress-video") compress (Join-Path $Fixtures "video.mp4") --target 200kb | Out-Null }
    Invoke-Step "compress image quality" { & $Forge --output (Join-Path $Out "compress-image") compress (Join-Path $Fixtures "image.png") --quality 80 | Out-Null }
    Invoke-Step "compress folder recursive" { & $Forge --output (Join-Path $Out "compress-folder") compress (Join-Path $Fixtures "videos") --recursive --target 200kb | Out-Null }
    Invoke-Step "clip from to" { & $Forge --output (Join-Path $Out "clip-from-to") clip (Join-Path $Fixtures "video.mp4") --from 00:00:00 --to 00:00:01 | Out-Null }
    Invoke-Step "clip range" { & $Forge --output (Join-Path $Out "clip-range") clip (Join-Path $Fixtures "video.mp4") --range 00:00:00..00:00:01 | Out-Null }
    Invoke-Step "clip remove silence" { & $Forge --output (Join-Path $Out "clip-silence") clip (Join-Path $Fixtures "video.mp4") --remove-silence | Out-Null }
    Invoke-Step "resize width" { & $Forge --output (Join-Path $Out "resize-width") resize (Join-Path $Fixtures "video.mp4") --width 160 | Out-Null }
    Invoke-Step "resize height" { & $Forge --output (Join-Path $Out "resize-height") resize (Join-Path $Fixtures "image.png") --height 90 | Out-Null }
    Invoke-Step "resize folder preset social" { & $Forge --output (Join-Path $Out "resize-social") resize (Join-Path $Fixtures "videos") --preset social | Out-Null }
    Invoke-Step "crop 9x16" { & $Forge --output (Join-Path $Out "crop-9x16") crop (Join-Path $Fixtures "video.mp4") --aspect 9:16 | Out-Null }
    Invoke-Step "crop 1x1" { & $Forge --output (Join-Path $Out "crop-1x1") crop (Join-Path $Fixtures "image.png") --aspect 1:1 | Out-Null }
    Invoke-Step "captions burn srt" { & $Forge --output (Join-Path $Out "captions-burn") captions (Join-Path $Fixtures "video.mp4") --srt (Join-Path $Fixtures "captions.srt") --burn --style shorts | Out-Null }
    Invoke-Step "captions from transcript" { & $Forge --output (Join-Path $Out "captions-from") captions (Join-Path $Fixtures "video.mp4") --from (Join-Path $Fixtures "transcript.txt") | Out-Null }
    Invoke-Step "transcribe srt" { & $Forge --output (Join-Path $Out "transcribe") transcribe (Join-Path $Fixtures "audio.wav") --srt | Out-Null }
    Invoke-Step "thumbnail at" { & $Forge --output (Join-Path $Out "thumb-at") thumbnail (Join-Path $Fixtures "video.mp4") --at 00:00:01 | Out-Null }
    Invoke-Step "thumbnail grid" { & $Forge --output (Join-Path $Out "thumb-grid") thumbnail (Join-Path $Fixtures "video.mp4") --grid | Out-Null }
    Invoke-Step "thumbnail best frame" { & $Forge --output (Join-Path $Out "thumb-best") thumbnail (Join-Path $Fixtures "video.mp4") --best-frame | Out-Null }
    Invoke-Step "audio normalize" { & $Forge --output (Join-Path $Out "audio-normalize") audio (Join-Path $Fixtures "audio.wav") --normalize | Out-Null }
    Invoke-Step "audio remove silence" { & $Forge --output (Join-Path $Out "audio-silence") audio (Join-Path $Fixtures "audio.wav") --remove-silence | Out-Null }
    Invoke-Step "audio clean" { & $Forge --output (Join-Path $Out "audio-clean") audio (Join-Path $Fixtures "audio.wav") --clean | Out-Null }
    Invoke-Step "audio extract" { & $Forge --output (Join-Path $Out "audio-extract") audio (Join-Path $Fixtures "video.mp4") --extract | Out-Null }
    Invoke-Step "batch videos compress" { & $Forge --output (Join-Path $Out "batch-videos") batch (Join-Path $Fixtures "videos") --compress --target 200kb | Out-Null }
    Invoke-Step "batch images convert resize" { & $Forge --output (Join-Path $Out "batch-images") batch (Join-Path $Fixtures "images") --to webp --resize 160 | Out-Null }
    Invoke-Step "batch lectures transcribe notes" { & $Forge --output (Join-Path $Out "batch-lectures") batch (Join-Path $Fixtures "lectures") --transcribe --notes | Out-Null }
    Invoke-Step "preset list" { & $Forge preset list | Out-Null }
    Invoke-Step "preset show" { & $Forge preset show social | Out-Null }
    Invoke-Step "preset create" { & $Forge preset create my-short | Out-Null }
    Invoke-Step "notes summary" { & $Forge --output (Join-Path $Out "notes") notes (Join-Path $Fixtures "video.mp4") --chapters --summary | Out-Null }

    if ($Network) {
        Invoke-Step "youtube metadata simulate" {
            & $Forge --output (Join-Path $Out "youtube") youtube "https://archive.org/details/SampleVideo1280x7205mb" --metadata --simulate | Out-Null
        }
    }

    $failed = Get-Content -LiteralPath $Log | ForEach-Object { $_ | ConvertFrom-Json } | Where-Object { -not $_.ok }
    if ($failed) {
        throw "E2E failed"
    }

    "E2E log: $Log"
}
finally {
    Pop-Location
}
