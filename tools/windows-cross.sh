#!/bin/sh
# Build Windows code on Linux using the user's existing, read-only Windows SDK.
# This does not change Windows application-control policy or sign binaries.
set -eu
: "${XEMNAS_SDK_VERSION:?Set the installed Windows SDK version}"
sdk_include="/win-sdk/Include/$XEMNAS_SDK_VERSION"
sdk_lib="/win-sdk/Lib/$XEMNAS_SDK_VERSION"
test -f "$sdk_include/um/windows.h"
test -f /msvc/include/vcruntime.h
mkdir -p /tmp/xemnas-cross
cat > /tmp/xemnas-cross/clang-cl <<'COMPILER'
#!/bin/sh
exec /usr/bin/clang-14 --driver-mode=cl --target=x86_64-pc-windows-msvc "$@"
COMPILER
chmod +x /tmp/xemnas-cross/clang-cl
export CC_x86_64_pc_windows_msvc=/tmp/xemnas-cross/clang-cl
export CXX_x86_64_pc_windows_msvc=/tmp/xemnas-cross/clang-cl
export AR_x86_64_pc_windows_msvc=/usr/bin/llvm-lib
export CFLAGS_x86_64_pc_windows_msvc="-imsvc /msvc/include -imsvc $sdk_include/ucrt -imsvc $sdk_include/shared -imsvc $sdk_include/um -imsvc $sdk_include/winrt"
export CXXFLAGS_x86_64_pc_windows_msvc="$CFLAGS_x86_64_pc_windows_msvc"
export CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER=/usr/bin/lld-link
export CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUSTFLAGS="-L native=/msvc/lib/x64 -L native=$sdk_lib/ucrt/x64 -L native=$sdk_lib/um/x64"
export RC_x86_64_pc_windows_msvc=/usr/bin/llvm-rc
export INCLUDE="/msvc/include;$sdk_include/ucrt;$sdk_include/shared;$sdk_include/um"
exec cargo build --locked -p desktop-gpui --bin xemnas --target x86_64-pc-windows-msvc
