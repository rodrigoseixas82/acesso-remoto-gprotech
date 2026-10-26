fn main() {
    #[cfg(windows)]
    {
        use std::io::Write;
        let mut res = winres::WindowsResource::new();
        // GPROTECH: idioma Português (Brasil) nas propriedades do arquivo.
        const LANG_PT_BR: u16 = 0x0416;
        res.set_icon("../../res/icon.ico")
            .set_language(LANG_PT_BR)
            .set_manifest_file("../../res/manifest.xml");
        gprotech_propriedades(&mut res);
        match res.compile() {
            Err(e) => {
                write!(std::io::stderr(), "{}", e).unwrap();
                std::process::exit(1);
            }
            Ok(_) => {}
        }
    }
}

// GPROTECH: propriedades do arquivo baixado (Propriedades > Detalhes) com o nome
// da edição, a GPROTECH e o número do build, no lugar dos dados do RustDesk.
// Versão do arquivo 1.5.1.<build>; versão do produto "Build <n>".
#[cfg(windows)]
fn gprotech_propriedades(res: &mut winres::WindowsResource) {
    for v in ["GPROTECH_EDICAO", "GPROTECH_DISPLAY_NAME", "GPROTECH_BUILD"] {
        println!("cargo:rerun-if-env-changed={v}");
    }
    let nome = std::env::var("GPROTECH_DISPLAY_NAME")
        .ok()
        .filter(|s| !s.trim().is_empty());
    let Some(nome) = nome else {
        return;
    };
    let edicao = if std::env::var("GPROTECH_EDICAO").as_deref() == Ok("atendente") {
        "Atendente"
    } else {
        "Cliente"
    };
    let build: u64 = std::env::var("GPROTECH_BUILD")
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0);
    let mut partes = env!("CARGO_PKG_VERSION")
        .split('.')
        .map(|p| p.parse::<u64>().unwrap_or(0));
    let (major, minor, patch) = (
        partes.next().unwrap_or(0),
        partes.next().unwrap_or(0),
        partes.next().unwrap_or(0),
    );
    let numero = (major << 48) | (minor << 32) | (patch << 16) | (build & 0xFFFF);
    res.set("FileDescription", &nome)
        .set("ProductName", &nome)
        .set("CompanyName", "GPROTECH")
        .set("LegalCopyright", "GPROTECH")
        .set(
            "OriginalFilename",
            &format!("Suporte-Remoto-GPROTECH-{edicao}.exe"),
        )
        .set("InternalName", &format!("Suporte-Remoto-GPROTECH-{edicao}"))
        .set("FileVersion", &format!("{major}.{minor}.{patch}.{build}"))
        .set(
            "ProductVersion",
            &if build > 0 {
                format!("Build {build}")
            } else {
                format!("{major}.{minor}.{patch}")
            },
        )
        .set_version_info(winres::VersionInfo::FILEVERSION, numero)
        .set_version_info(winres::VersionInfo::PRODUCTVERSION, numero);
}
