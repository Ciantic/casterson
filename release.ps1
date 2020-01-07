# Ensure 1. vcpkg installed: https://github.com/Microsoft/vcpkg
# 
# > git clone https://github.com/Microsoft/vcpkg.git
# > cd vcpkg
# 
# PS> .\bootstrap-vcpkg.bat
# PS> .\vcpkg integrate install
#
# vcpkg install openssl:x64-windows-static

# Only one of the environment variables: VCPKGRS_DYNAMIC or RUSTFLAGS static
# should be set! RUSTFLAGS makes the builds statically linked!

# Remove-Item env:VCPKGRS_DYNAMIC
# $env:RUSTFLAGS="-Ctarget-feature=+crt-static"

cargo build --release