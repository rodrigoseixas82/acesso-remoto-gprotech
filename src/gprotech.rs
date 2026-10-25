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

/// Opção local com o nome do técnico (edição atendente), digitado no campo "Seu nome".
pub const OPTION_NOME_TECNICO: &str = "gprotech-nome-tecnico";

/// Nome que o técnico digitou; vai na conexão e aparece para o cliente.
pub fn nome_tecnico() -> String {
    if is_edicao_cliente() {
        return String::new();
    }
    config::LocalConfig::get_option(OPTION_NOME_TECNICO)
        .trim()
        .to_owned()
}

/// Nome mostrado ao usuário, com espaços: "Aplicativos instalados", atalhos,
/// menu Iniciar, bandeja, serviço e pasta de instalação. O APP_NAME interno
/// continua sem espaços porque vai sem aspas em comandos `sc`, `taskkill` e `reg`.
pub fn display_name() -> String {
    non_empty(option_env!("GPROTECH_DISPLAY_NAME"))
        .map(|s| s.trim().to_owned())
        .unwrap_or_else(crate::get_app_name)
}

/// Arquivo baixado aberto numa máquina que já tem esta mesma versão (ou uma
/// mais nova) instalada: abre o programa instalado e devolve `true` para a cópia
/// temporária encerrar. A cópia não conversa com o serviço instalado (o canal
/// interno só aceita o mesmo executável) e ficaria em "Não está pronto".
/// Se o arquivo for mais novo, segue aberto e mostra o aviso "Atualizar".
#[cfg(windows)]
pub fn abrir_instalado_no_lugar_da_copia() -> bool {
    use crate::platform::windows as win;
    if !win::is_installed()
        || win::is_cur_exe_the_installed()
        || crate::ui_interface::is_installed_lower_version()
    {
        return false;
    }
    let (_, _, _, exe) = win::get_install_info();
    match std::process::Command::new(&exe).spawn() {
        Ok(_) => true,
        Err(e) => {
            hbb_common::log::error!("GPROTECH: falha ao abrir {exe}: {e}");
            false
        }
    }
}

/// Valor de `display` no pedido de captura que significa "papel de parede"
/// (miniatura do cartão no Atendente) em vez de print da tela.
pub const DISPLAY_MINIATURA: i32 = 1000;

/// Resposta ao pedido de miniatura: a imagem do papel de parede do usuário
/// logado (a área de trabalho sem janelas nem ícones).
pub fn resposta_papel_de_parede(sid: String) -> base::message_proto::Message {
    use base::message_proto::{Message, ScreenshotResponse};
    let mut response = ScreenshotResponse::new();
    response.sid = sid;
    match ler_papel_de_parede() {
        Some(data) => response.data = data.into(),
        None => response.msg = "Papel de parede indisponível".to_owned(),
    }
    let mut msg = Message::new();
    msg.set_screenshot_response(response);
    msg
}

#[cfg(windows)]
fn ler_papel_de_parede() -> Option<Vec<u8>> {
    const LIMITE: u64 = 20 * 1024 * 1024;
    let mut candidatos = Vec::new();
    // Cópia que o Windows mantém do papel de parede atual (JPEG, sem extensão).
    if let Some(home) = crate::platform::windows::get_active_user_home() {
        candidatos.push(home.join("AppData\\Roaming\\Microsoft\\Windows\\Themes\\TranscodedWallpaper"));
    }
    if let Ok(path) = wallpaper::get() {
        if !path.is_empty() {
            candidatos.push(std::path::PathBuf::from(path));
        }
    }
    for path in candidatos {
        match std::fs::metadata(&path) {
            Ok(m) if m.is_file() && m.len() > 0 && m.len() <= LIMITE => {
                if let Ok(data) = std::fs::read(&path) {
                    return Some(data);
                }
            }
            _ => {}
        }
    }
    None
}

#[cfg(not(windows))]
fn ler_papel_de_parede() -> Option<Vec<u8>> {
    None
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
