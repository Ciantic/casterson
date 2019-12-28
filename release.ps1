# Ensure 1. vcpkg 
$env:VCPKGRS_DYNAMIC = "0"
$env:OPENSSL_STATIC="1"
$env:OPENSSL_DIR="C:\\DepSource\\vcpkg\\installed\\x64-windows"
$env:OPENSSL_LIB_DIR="C:\\DepSource\\vcpkg\\installed\\x64-windows\\lib"
$env:OPENSSL_INCLUDE_DIR="C:\\DepSource\\vcpkg\\installed\\x64-windows\\include"
$env:RUSTFLAGS = "-Ctarget-feature=+crt-static"
cargo build --release