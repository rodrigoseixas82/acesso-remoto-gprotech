//! Configurações fixas do cliente GPROTECH.
//!
//! Usa os mesmos mapas que o gerador de cliente personalizado do RustDesk preenche
//! (HARD_SETTINGS, BUILTIN_SETTINGS, OVERWRITE_*), só que definidos em tempo de
//! compilação. Variáveis de build:
//! - `GPROTECH_EDICAO`: `cliente` (padrão, só recebe acesso) ou `atendente`;
//! - `GPROTECH_DISPLAY_NAME`: nome mostrado na janela (pode ter espaço);
//! - `GPROTECH_WHATSAPP`: número do suporte (só dígitos, com DDI) para o botão;
//! - `GPROTECH_PRESET_PASSWORD_HASH` / `GPROTECH_PRESET_SALT`: senha interna da
//!   GPROTECH, embutida só como hash ("00" + base64(sha256(senha + salt))).
//!   Vêm de secrets do GitHub; o texto da senha nunca entra no código nem no binário.

use hbb_common::config;

fn edicao() -> &'static str {
    option_env!("GPROTECH_EDICAO").unwrap_or("cliente")
}

/// Edição instalada nos clientes (lojas): só recebe acesso.
pub fn is_edicao_cliente() -> bool {
    edicao() != "atendente"
}

fn non_empty(v: Option<&'static str>) -> Option<&'static str> {
    v.filter(|s| !s.trim().is_empty())
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
        if let Some(name) = non_empty(option_env!("GPROTECH_DISPLAY_NAME")) {
            builtin.insert("gprotech-display-name".to_owned(), name.to_owned());
        }
        if is_edicao_cliente() {
            if let Some(phone) = non_empty(option_env!("GPROTECH_WHATSAPP")) {
                builtin.insert("gprotech-whatsapp".to_owned(), phone.to_owned());
            }
        }
    }

    // Tema escuro fixo, nas cores do site.
    config::OVERWRITE_LOCAL_SETTINGS
        .write()
        .unwrap()
        .insert("theme".to_owned(), "dark".to_owned());

    if !is_edicao_cliente() {
        return;
    }

    let mut hard = config::HARD_SETTINGS.write().unwrap();
    hard.insert("conn-type".to_owned(), "incoming".to_owned());
    hard.insert("disable-settings".to_owned(), "Y".to_owned());

    if let (Some(hash), Some(salt)) = (
        non_empty(option_env!("GPROTECH_PRESET_PASSWORD_HASH")),
        non_empty(option_env!("GPROTECH_PRESET_SALT")),
    ) {
        hard.insert("password".to_owned(), hash.trim().to_owned());
        hard.insert("salt".to_owned(), salt.trim().to_owned());
        drop(hard);
        // Só a senha interna: sem senha de uso único e sem clique de aprovação.
        let mut overwrite = config::OVERWRITE_SETTINGS.write().unwrap();
        overwrite.insert("verification-method".to_owned(), "use-permanent-password".to_owned());
        overwrite.insert("approve-mode".to_owned(), "password".to_owned());
    }
}
