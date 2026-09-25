#define WIN32_LEAN_AND_MEAN
#define _WIN32_WINNT 0x0A00
#include <windows.h>
#include <bcrypt.h>
#include <wchar.h>
#include <string.h>
#include <shellapi.h>
#include <stdio.h>
#include "runtime_hash.h"

// This executable imports only OS DLLs. Application code is not loaded until
// its bytes match the digest embedded when this launcher was built.
static WCHAR runtime_path[32768];
static WCHAR final_path[32768];
static BYTE block[65536];
typedef DWORD (WINAPI *AppFunction)(void);

static AppFunction exported(HMODULE library, LPCSTR name) {
    FARPROC address = GetProcAddress(library, name);
    AppFunction function;
    // Win64 function pointers have the same representation; avoid casting calls.
    memcpy(&function, &address, sizeof(function));
    return function;
}

static BOOL digest_matches(HANDLE file) {
    BCRYPT_ALG_HANDLE algorithm = NULL;
    BCRYPT_HASH_HANDLE hash = NULL;
    BYTE digest[32];
    DWORD count = 0;
    BOOL ok = FALSE;
    if (BCryptOpenAlgorithmProvider(&algorithm, BCRYPT_SHA256_ALGORITHM, NULL, 0) < 0) goto cleanup;
    if (BCryptCreateHash(algorithm, &hash, NULL, 0, NULL, 0, 0) < 0) goto cleanup;
    for (;;) {
        if (!ReadFile(file, block, sizeof(block), &count, NULL)) goto cleanup;
        if (!count) break;
        if (BCryptHashData(hash, block, count, 0) < 0) goto cleanup;
    }
    if (BCryptFinishHash(hash, digest, sizeof(digest), 0) < 0) goto cleanup;
    ok = TRUE;
    for (unsigned i = 0; i < sizeof(digest); i++) {
        if (digest[i] != DX_RUNTIME_SHA256[i]) ok = FALSE;
    }
cleanup:
    if (hash) BCryptDestroyHash(hash);
    if (algorithm) BCryptCloseAlgorithmProvider(algorithm, 0);
    return ok;
}

static int fail(int code, BOOL quiet, LPCWSTR text) {
    if (!quiet) MessageBoxW(NULL, text, L"dengeX Remote - Paket denetimi", MB_OK | MB_ICONERROR);
    return code;
}

int WINAPI wWinMain(HINSTANCE instance, HINSTANCE previous, LPWSTR command, int show) {
    (void)instance; (void)previous; (void)show; (void)command;
    int argc = 0;
    LPWSTR *argv = CommandLineToArgvW(GetCommandLineW(), &argc);
    BOOL verify_only = argv && argc == 2 && wcscmp(argv[1], L"--verify-package") == 0;
    BOOL load_only = argv && argc == 2 && wcscmp(argv[1], L"--load-check") == 0;
    if (argv) LocalFree(argv);
    BOOL quiet = verify_only || load_only;
    SetErrorMode(SEM_FAILCRITICALERRORS | SEM_NOOPENFILEERRORBOX);
    // No current directory or PATH search, including subsequent dynamic loads.
    if (!SetDefaultDllDirectories(LOAD_LIBRARY_SEARCH_SYSTEM32))
        return fail(10, quiet, L"Guvenli DLL yukleme baslatilamadi.");
    DWORD length = GetModuleFileNameW(NULL, runtime_path, ARRAYSIZE(runtime_path));
    if (!length || length >= ARRAYSIZE(runtime_path))
        return fail(11, quiet, L"Uygulama konumu okunamadi.");
    WCHAR *slash = wcsrchr(runtime_path, L'\\');
    if (!slash) return fail(11, quiet, L"Uygulama konumu gecersiz.");
    slash[1] = 0;
    if (wcscat_s(runtime_path, ARRAYSIZE(runtime_path), L"dengex_desktop.dll"))
        return fail(11, quiet, L"Uygulama yolu cok uzun.");
    // Deny write/delete sharing from verification through process exit.
    HANDLE file = CreateFileW(runtime_path, GENERIC_READ, FILE_SHARE_READ, NULL,
                              OPEN_EXISTING, FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_SEQUENTIAL_SCAN, NULL);
    if (file == INVALID_HANDLE_VALUE)
        return fail(12, quiet, L"dengex_desktop.dll bulunamadi veya kullanimda. ZIP paketini tamamen cikarin; EXE ve DLL ayni klasorde olmali.");
    BY_HANDLE_FILE_INFORMATION info;
    BOOL regular = GetFileInformationByHandle(file, &info) &&
        !(info.dwFileAttributes & (FILE_ATTRIBUTE_REPARSE_POINT | FILE_ATTRIBUTE_DIRECTORY));
    if (!regular || !digest_matches(file)) {
        CloseHandle(file);
        return fail(13, quiet, L"DLL butunluk kontrolu basarisiz. Dosya degismis veya farkli surumden. Resmi ZIP paketini yeniden indirin.");
    }
    if (verify_only) { CloseHandle(file); return 0; }
    if (load_only) { fputs("DLL hash verified; loading library\n", stderr); fflush(stderr); }
    length = GetFinalPathNameByHandleW(file, final_path, ARRAYSIZE(final_path), FILE_NAME_NORMALIZED | VOLUME_NAME_DOS);
    if (!length || length >= ARRAYSIZE(final_path)) {
        CloseHandle(file);
        return fail(11, quiet, L"Dogrulanan dosyanin konumu okunamadi.");
    }
    HMODULE library = LoadLibraryExW(final_path, NULL, LOAD_LIBRARY_SEARCH_SYSTEM32);
    if (!library) {
        if (load_only) { fprintf(stderr, "LoadLibraryEx failed: %lu\n", GetLastError()); fflush(stderr); }
        CloseHandle(file);
        return fail(14, quiet, L"Uygulama DLL'i acilamadi. Windows x64 ve Media Foundation gerekir. Windows N surumlerinde Media Feature Pack'i kontrol edin.");
    }
    if (load_only) { fputs("Library loaded; checking ABI\n", stderr); fflush(stderr); }
    AppFunction version = exported(library, "DxAppAbiVersion");
    AppFunction run = exported(library, "DxAppRun");
    if (!version || !run || version() != 1)
        return fail(15, quiet, L"EXE ile DLL surumleri uyusmuyor. Paketi birlikte guncelleyin.");
    // Do not unload the library: framework threads/callbacks can outlive run().
    // OS teardown releases the DLL and the locked file together at process exit.
    if (load_only) { fputs("ABI verified\n", stderr); fflush(stderr); return 0; }
    return (int)run();
}
