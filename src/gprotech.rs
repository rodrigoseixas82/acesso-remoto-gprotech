//! Configurações fixas do cliente GPROTECH.
//!
//! Usa os mesmos mapas que o gerador de cliente personalizado do RustDesk preenche
//! (HARD_SETTINGS, BUILTIN_SETTINGS, OVERWRITE_*), só que definidos em tempo de
//! compilação. A edição vem da variável de build `GPROTECH_EDICAO`:
//! - `lojas` (padrão): só recebe acesso, sem configurações de rede/servidor;
//! - `tecnico`: recebe e faz acesso.

use hbb_common::config;

fn edicao() -> &'static str {
    option_env!("GPROTECH_EDICAO").unwrap_or("lojas")
}

pub fn is_edicao_lojas() -> bool {
    edicao() != "tecnico"
}

pub fn apply_defaults() {
    {
        let mut builtin = config::BUILTIN_SETTINGS.write().unwrap();
        for k in [
            "hide-powered-by-me",
            "hide-server-settings",
            "hide-proxy-settings",
            "hide-websocket-settings",
            "hide-network-settings",
            "remove-preset-password-warning",
            "disable-change-id",
        ] {
            builtin.insert(k.to_owned(), "Y".to_owned());
        }
    }

    // Tema escuro fixo, nas cores do site.
    config::OVERWRITE_LOCAL_SETTINGS
        .write()
        .unwrap()
        .insert("theme".to_owned(), "dark".to_owned());

    if is_edicao_lojas() {
        config::HARD_SETTINGS
            .write()
            .unwrap()
            .insert("conn-type".to_owned(), "incoming".to_owned());
    }
}
