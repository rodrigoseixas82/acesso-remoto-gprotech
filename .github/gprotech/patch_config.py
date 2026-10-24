#!/usr/bin/env python3
"""Embute servidor, chave e nome da GPROTECH no submódulo hbb_common antes do build.

O submódulo libs/hbb_common continua apontando para o repositório oficial; este
script só altera a cópia de trabalho dentro do runner do GitHub Actions.
"""
import os
import re
import sys

CONFIG = os.path.join("libs", "hbb_common", "src", "config.rs")

SERVER = os.environ.get("GPROTECH_SERVER", "acesso.gprotech.com.br")
PUB_KEY = os.environ.get("GPROTECH_PUB_KEY", "HFMnWCRtF2l+XiO7xe+hffIyGYZRBIzvfMYuA2c8Pxo=")
# Sem espaços: o nome vira nome de serviço do Windows (sc create <nome>) e de pastas.
APP_NAME = os.environ.get("GPROTECH_APP_NAME", "GPROTECH-Suporte")

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
    return 0


if __name__ == "__main__":
    sys.exit(main())
