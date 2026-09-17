#!/usr/bin/env bash
# 合并构建: Scintilla + Lexilla 全部 lexer 单 DLL
cd "$(dirname "$0")"
LEX=../lexilla
SOURCES=$(ls src/*.cxx win32/PlatWin.cxx win32/ScintillaWin.cxx win32/ScintillaDLL.cxx win32/HanjaDic.cxx $LEX/src/Lexilla.cxx $LEX/lexlib/*.cxx $LEX/lexers/*.cxx | tr '\n' ' ')
zig c++ -shared -std=c++17 -O1 \
  -I include -I src -I win32 -I $LEX/include -I $LEX/lexlib -I $LEX/src \
  -DNDEBUG -D_CRT_SECURE_NO_DEPRECATE=1 -D_UNICODE -DUNICODE \
  -o bin/Scintilla.dll $SOURCES \
  win32/aine.def -lgdi32 -luser32 -limm32 -lole32 -luuid -loleaut32
