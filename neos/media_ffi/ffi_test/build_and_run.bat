@echo off
REM Compiles ffi_test.exe against the freshly-built media_ffi.dll and runs
REM it — the real, independent-of-Rust proof that the C ABI genuinely
REM links, as opposed to Rust calling its own extern "C" functions. Run
REM `cargo build -p media-ffi` first.
setlocal
call "C:\Program Files\Microsoft Visual Studio\18\Community\VC\Auxiliary\Build\vcvarsall.bat" x64
if errorlevel 1 (
    echo VCVARSALL FAILED
    exit /b 1
)
cd /d "%~dp0"
copy /y "..\..\target\debug\media_ffi.dll" . >nul
cl.exe /nologo /W3 main.c /link /LIBPATH:"..\..\target\debug" media_ffi.dll.lib /OUT:ffi_test.exe
if errorlevel 1 (
    echo COMPILE FAILED
    exit /b 1
)
.\ffi_test.exe
