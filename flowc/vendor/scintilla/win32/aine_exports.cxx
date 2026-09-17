// aine_exports.cxx — Aine 专用导出层: 显式 dllexport 两个入口
// (官方 def 仅导出 DirectFunction; RegisterClasses 需自行导出)
#include <windows.h>

extern "C" {
    int Scintilla_RegisterClasses(void* hInstance);
    long long __stdcall Scintilla_DirectFunction(void* sci, unsigned iMessage, unsigned long long wParam, long long lParam);
}

extern "C" __declspec(dllexport) int al_scintilla_register(void* hInstance) {
    SetLastError(0);
    int r = Scintilla_RegisterClasses(hInstance);
    return r;
}

extern "C" __declspec(dllexport) long long __stdcall al_scintilla_direct(void* sci, unsigned iMessage, unsigned long long wParam, long long lParam) {
    return Scintilla_DirectFunction(sci, iMessage, wParam, lParam);
}
