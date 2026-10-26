#!/usr/bin/env python3
"""Embute servidor, chave e nome da GPROTECH no submódulo hbb_common antes do build.

O submódulo libs/hbb_common continua apontando para o repositório oficial; este
script só altera a cópia de trabalho dentro do runner do GitHub Actions.
"""
import os
import re
import sys

CONFIG = os.path.join("libs", "hbb_common", "src", "config.rs")
RUNNER_RC = os.path.join("flutter", "windows", "runner", "Runner.rc")
PUBSPEC = os.path.join("flutter", "pubspec.yaml")

SERVER = os.environ.get("GPROTECH_SERVER", "acesso.gprotech.com.br")
PUB_KEY = os.environ.get("GPROTECH_PUB_KEY", "HFMnWCRtF2l+XiO7xe+hffIyGYZRBIzvfMYuA2c8Pxo=")
# Sem espaços: o nome vira nome de serviço do Windows (sc create <nome>) e de pastas.
APP_NAME = os.environ.get("GPROTECH_APP_NAME", "SuporteRemoto-GPROTECH-Cliente")
# Nome visível, com espaços.
DISPLAY_NAME = os.environ.get("GPROTECH_DISPLAY_NAME", "Suporte Remoto GPROTECH - Cliente")

PATCHES = [
    (
        r'pub const RENDEZVOUS_SERVERS: &\[&str\] = &\[[^\]]*\];',
        f'pub const RENDEZVOUS_SERVERS: &[&str] = &["{SERVER}"];',
    ),
    (
        r'pub const RS_PUB_KEY: &str = "[^"]*";',
        f'pub const RS_PUB_KEY: &str = "{PUB_KEY}";',
    ),
    (
        r'pub static ref APP_NAME: RwLock<String> = RwLock::new\("[^"]*"\.to_owned\(\)\);',
        f'pub static ref APP_NAME: RwLock<String> = RwLock::new("{APP_NAME}".to_owned());',
    ),
]


def main() -> int:
    if " " in APP_NAME:
        print(f"APP_NAME não pode ter espaços: {APP_NAME!r}", file=sys.stderr)
        return 1

    with open(CONFIG, encoding="utf-8") as f:
        src = f.read()

    for pattern, replacement in PATCHES:
        src, count = re.subn(pattern, lambda _m: replacement, src)
        if count != 1:
            print(f"Padrão encontrado {count} vez(es), esperado 1: {pattern}", file=sys.stderr)
            return 1
        print(f"ok: {replacement}")

    with open(CONFIG, "w", encoding="utf-8", newline="\n") as f:
        f.write(src)
    return patch_runner_rc()


def patch_runner_rc() -> int:
    """Propriedades do .exe: aparecem no aviso do Windows (UAC) ao instalar e
    desinstalar, no Gerenciador de Tarefas e em Propriedades > Detalhes."""
    values = {
        "CompanyName": "GPROTECH",
        "FileDescription": DISPLAY_NAME,
        "InternalName": APP_NAME,
        "LegalCopyright": "GPROTECH",
        "OriginalFilename": f"{APP_NAME}.exe",
        "ProductName": DISPLAY_NAME,
    }
    with open(RUNNER_RC, encoding="utf-8", newline="") as f:
        src = f.read()
    for key, value in values.items():
        line = f'VALUE "{key}", "{value}"'
        src, count = re.subn(rf'VALUE "{key}", "[^"]*"', lambda _m: line, src)
        if count != 1:
            print(f"Runner.rc: {key} encontrado {count} vez(es), esperado 1", file=sys.stderr)
            return 1
        print(f"ok: {line}")
    # Número do build na versão (1.5.1.<build> / "Build <n>") e idioma
    # Português (Brasil), como no arquivo baixado (libs/portable/build.rs).
    build = os.environ.get("GPROTECH_BUILD", "").strip()
    build = build if build.isdigit() else "0"
    with open(PUBSPEC, encoding="utf-8") as f:
        m = re.search(r"^version:\s*([0-9]+)\.([0-9]+)\.([0-9]+)", f.read(), re.M)
    major, minor, patch = m.groups() if m else ("1", "5", "1")
    product = f"Build {build}" if build != "0" else f"{major}.{minor}.{patch}"
    extras = [
        (r" FILEVERSION VERSION_AS_NUMBER", f" FILEVERSION {major},{minor},{patch},{build}"),
        (r" PRODUCTVERSION VERSION_AS_NUMBER", f" PRODUCTVERSION {major},{minor},{patch},{build}"),
        (r'VALUE "FileVersion", VERSION_AS_STRING', f'VALUE "FileVersion", "{major}.{minor}.{patch}.{build}"'),
        (r'VALUE "ProductVersion", VERSION_AS_STRING', f'VALUE "ProductVersion", "{product}"'),
        (r"LANGUAGE LANG_ENGLISH, SUBLANG_ENGLISH_US", "LANGUAGE LANG_PORTUGUESE, SUBLANG_PORTUGUESE_BRAZILIAN"),
        (r'BLOCK "040904e4"', 'BLOCK "041604e4"'),
        (r'VALUE "Translation", 0x409, 1252', 'VALUE "Translation", 0x416, 1252'),
    ]
    for pattern, replacement in extras:
        src, count = re.subn(pattern, lambda _m: replacement, src)
        if count != 1:
            print(f"Runner.rc: {pattern} encontrado {count} vez(es), esperado 1", file=sys.stderr)
            return 1
        print(f"ok: {replacement}")
    with open(RUNNER_RC, "w", encoding="utf-8", newline="") as f:
        f.write(src)
    return 0


if __name__ == "__main__":
    sys.exit(main())
