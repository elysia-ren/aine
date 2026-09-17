#!/usr/bin/env bash
# Aine Build/Publish：single / portable / bundle（Windows）
set -e
cd "$(dirname "$0")/.."
MODE="${1:-single}"
VERSION=$(grep '^version' Cargo.toml | head -1 | awk '{print $3}' | tr -d '"')
DIST="dist/aine-${VERSION}-${MODE}"
rm -rf "$DIST"
mkdir -p "$DIST"
CARGO_TARGET_DIR=target3 cargo build --release
cp target3/release/aine.exe "$DIST/aine.exe"
if [ "$MODE" = "bundle" ]; then
    cp -r examples "$DIST/examples"
    cp -r stdlib "$DIST/stdlib"
    cp -r conformance "$DIST/conformance"
    cp -r tools "$DIST/tools"
    cp -r src "$DIST/src"
    cp Cargo.toml Cargo.lock "$DIST/"
    cp IMPLEMENTATION_LOG.md "$DIST/"
    cp "E:/Flow/The_Aine_Book.md" "E:/Flow/Aine_Cookbook.md" "$DIST/" 2>/dev/null || true
    printf '@echo off\r\naire.exe --version\r\naire.exe run examples\transpiler.aine\r\naire.exe check examples\account_book.aine\r\necho bundle OK\r\n' > "$DIST/verify.bat"
elif [ "$MODE" = "portable" ]; then
    cp -r examples "$DIST/examples"
    cp -r stdlib "$DIST/stdlib"
    cp -r conformance "$DIST/conformance"
    cp IMPLEMENTATION_LOG.md "$DIST/"
fi
FINGERPRINT=$(certutil -hashfile "$DIST/aine.exe" SHA256 2>/dev/null | sed -n '2p' | tr -d ' ' || echo unavailable)
printf 'Aine %s 签名预留接口\n=====================\n代码签名流程（证书就位后执行）:\n  1. signtool sign /f <证书.pfx> /fd SHA256 /t <时间戳服务器> aine.exe\n  2. 重新生成本文件并填入签名指纹\n当前 SHA256 指纹: %s\n分发校验: certutil -hashfile aine.exe SHA256\n' "$VERSION" "$FINGERPRINT" > "$DIST/SIGNATURE.txt"
printf 'Aine %s (%s build)\n语言: Aine  ·  CLI: aine  ·  后缀: .aine\n验证: aine run examples/transpiler.aine  →  31 用例\n签名: 见 SIGNATURE.txt\n' "$VERSION" "$MODE" > "$DIST/README.txt"
echo "打包完成: $DIST"
