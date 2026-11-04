# Build Unseen OS and start it in QEMU.
#   .\run.ps1            build + open the QEMU window
#   .\run.ps1 -BuildOnly just build
param([switch]$BuildOnly)
# (No "Stop" mode: cargo writes normal progress to stderr; we check exit codes instead.)
$root = $PSScriptRoot
$cargo = "$env:USERPROFILE\.cargo\bin\cargo.exe"
$rustc = "$env:USERPROFILE\.cargo\bin\rustc.exe"
$qemu = "C:\Program Files\qemu\qemu-system-x86_64.exe"

# Build the kernel (an ELF file for the bare-metal x86_64 target).
Push-Location "$root\kernel"
try {
    & $cargo build --release
    if ($LASTEXITCODE -ne 0) { throw "kernel build failed" }
} finally { Pop-Location }

# Turn it into a flat image that QEMU's multiboot loader can start.
$sysroot = & $rustc --print sysroot
$objcopy = Get-ChildItem "$sysroot\lib\rustlib\*\bin\llvm-objcopy.exe" | Select-Object -First 1
$elf = "$root\kernel\target\x86_64-unknown-none\release\unseen-kernel"
New-Item -ItemType Directory -Force "$root\out" | Out-Null
& $objcopy.FullName -O binary $elf "$root\out\unseen.bin"
if ($LASTEXITCODE -ne 0) { throw "objcopy failed" }
Write-Host "Built out\unseen.bin"

if (-not $BuildOnly) {
    & $qemu -kernel "$root\out\unseen.bin" -m 128M -rtc base=localtime -serial file:"$root\out\serial.log" -name "Unseen OS" -display gtk,zoom-to-fit=on
}
