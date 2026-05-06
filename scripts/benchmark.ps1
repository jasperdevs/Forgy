$ErrorActionPreference = "Stop"

Push-Location (Split-Path -Parent $PSScriptRoot)
try {
    cargo build --release --workspace | Out-Host

    $commands = @(
        @{ Name = "forge --help"; Command = { target\release\forge.exe --help | Out-Null } },
        @{ Name = "forge --version"; Command = { target\release\forge.exe --version | Out-Null } },
        @{ Name = "forge doctor --json"; Command = { target\release\forge.exe --json doctor | Out-Null } }
    )

    if (Get-Command ffmpeg -ErrorAction SilentlyContinue) {
        $commands += @{ Name = "ffmpeg -version"; Command = { ffmpeg -version | Out-Null } }
    }

    if (Get-Command yt-dlp -ErrorAction SilentlyContinue) {
        $commands += @{ Name = "yt-dlp --help"; Command = { yt-dlp --help | Out-Null } }
    }

    foreach ($entry in $commands) {
        $runs = 1..5 | ForEach-Object {
            (Measure-Command $entry.Command).TotalMilliseconds
        }
        $avg = ($runs | Measure-Object -Average).Average
        $min = ($runs | Measure-Object -Minimum).Minimum
        $max = ($runs | Measure-Object -Maximum).Maximum
        "{0,-22} avg={1,8:N1} ms min={2,8:N1} ms max={3,8:N1} ms" -f $entry.Name, $avg, $min, $max
    }
}
finally {
    Pop-Location
}
