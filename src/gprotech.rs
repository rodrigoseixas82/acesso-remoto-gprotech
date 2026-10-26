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
//! - `GPROTECH_BUILD`: número do build (run_number do workflow); 0 = build local.

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

/// Cliente aberto sem instalar (arquivo baixado): ao desinstalar, o RustDesk grava
/// `stop-service = Y`, e a cópia baixada depois ficaria "Serviço não está em
/// execução" (inacessível) até alguém clicar em "Iniciar Serviço". Aqui libera.
#[cfg(windows)]
pub fn liberar_servico_sem_instalar() {
    if is_edicao_cliente()
        && !crate::platform::windows::is_installed()
        && config::Config::get_option("stop-service") == "Y"
    {
        config::Config::set_option("stop-service".to_owned(), "".to_owned());
        hbb_common::log::info!("GPROTECH: serviço liberado no Cliente sem instalar");
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

/// Opção local com a foto do técnico (campo "Sua foto" do Atendente), em data URI.
pub const OPTION_FOTO_TECNICO: &str = "gprotech-foto-tecnico";

/// Foto do técnico para ir na conexão (só no Atendente). Limite folgado abaixo
/// dos 128 KB que o lado da loja aceita na mensagem de login.
pub fn foto_tecnico() -> String {
    if is_edicao_cliente() {
        return String::new();
    }
    let foto = config::LocalConfig::get_option(OPTION_FOTO_TECNICO);
    if foto.starts_with("data:image/") && foto.len() <= 64 * 1024 {
        foto
    } else {
        String::new()
    }
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
        if numero_build() > 0 {
            builtin.insert("gprotech-build".to_owned(), numero_build().to_string());
        }
        if atualizacao_ativa() {
            builtin.insert("gprotech-atualizacao".to_owned(), "Y".to_owned());
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

    // Atualização só pelo cartão "Atualizar" (clique do usuário), nunca sozinha.
    config::OVERWRITE_SETTINGS
        .write()
        .unwrap()
        .insert("allow-auto-update".to_owned(), "N".to_owned());

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

/// Número do build (run_number do workflow "GPROTECH - Build Windows"); 0 em build local.
pub fn numero_build() -> u32 {
    option_env!("GPROTECH_BUILD")
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0)
}

/// Aviso de versão nova pelo Release do GitHub: só no Cliente (o Atendente é
/// atualizado à mão, a partir do build) e só em build numerado.
pub fn atualizacao_ativa() -> bool {
    cfg!(windows) && is_edicao_cliente() && numero_build() > 0
}

/// Atualização pelo GitHub Releases. O workflow "GPROTECH - Publicar Release"
/// publica o `.exe` do Cliente, o `versao.json` e o `versao.json.sig` (assinatura
/// ed25519 com a chave do secret GPROTECH_UPDATE_SIGNING_KEY). O programa só
/// aceita o `versao.json` assinado por essa chave, e o download é conferido pelo
/// tamanho e SHA-256 que vêm nele.
#[cfg(windows)]
mod atualizacao {
    use super::numero_build;
    use hbb_common::{
        anyhow::{anyhow, Context},
        base64::{engine::general_purpose::STANDARD, Engine as _},
        bail, log,
        sodiumoxide::crypto::sign::{verify_detached, PublicKey, Signature},
        ResultType,
    };
    use std::{
        io::Read,
        sync::atomic::{AtomicBool, Ordering},
        time::Duration,
    };

    pub const REPO_DONO: &str = "rodrigoseixas82";
    pub const REPO_NOME: &str = "acesso-remoto-gprotech";
    const URL_VERSAO: &str = "https://github.com/rodrigoseixas82/acesso-remoto-gprotech/releases/latest/download/versao.json";
    const URL_ASSINATURA: &str = "https://github.com/rodrigoseixas82/acesso-remoto-gprotech/releases/latest/download/versao.json.sig";
    const PREFIXO_DOWNLOAD: &str = "https://github.com/rodrigoseixas82/acesso-remoto-gprotech/releases/download/";
    const PREFIXO_PAGINA: &str = "https://github.com/rodrigoseixas82/acesso-remoto-gprotech/releases/tag/";
    const APP: &str = "suporte-remoto-gprotech-cliente";
    const CONTEXTO_ASSINATURA: &[u8] = b"GPROTECH atualizacao v1\n";
    // Chave pública da GPROTECH (a privada fica só no secret do GitHub).
    const CHAVE_PUBLICA: [u8; 32] = [
        92, 124, 134, 248, 74, 13, 26, 135, 234, 71, 32, 232, 50, 46, 126, 107, 67, 209, 140,
        239, 126, 68, 11, 97, 163, 56, 156, 160, 32, 249, 134, 177,
    ];
    const LIMITE_BYTES: u64 = 64 * 1024;
    const INTERVALO: Duration = Duration::from_secs(6 * 60 * 60);

    #[derive(serde_derive::Deserialize)]
    struct VersaoPublicada {
        app: String,
        build: u32,
        pagina: String,
        url: String,
        tamanho: u64,
        sha256: String,
    }

    fn baixar(url: &str) -> ResultType<Vec<u8>> {
        let client = crate::hbbs_http::create_http_client_with_url_strict(url)?;
        let response = client.get(url).timeout(Duration::from_secs(30)).send()?;
        if !response.status().is_success() {
            bail!("HTTP {} em {}", response.status(), url);
        }
        let mut bytes = Vec::new();
        response.take(LIMITE_BYTES + 1).read_to_end(&mut bytes)?;
        if bytes.len() as u64 > LIMITE_BYTES {
            bail!("arquivo grande demais: {}", url);
        }
        Ok(bytes)
    }

    fn conferir_assinatura(dados: &[u8], assinatura_b64: &[u8]) -> ResultType<()> {
        let texto = std::str::from_utf8(assinatura_b64).context("assinatura inválida")?;
        let bytes = STANDARD
            .decode(texto.trim())
            .context("assinatura não está em base64")?;
        let bytes: [u8; 64] = bytes
            .try_into()
            .map_err(|_| anyhow!("assinatura com tamanho errado"))?;
        let assinatura =
            Signature::from_bytes(&bytes).map_err(|_| anyhow!("assinatura inválida"))?;
        hbb_common::sodiumoxide::init().map_err(|_| anyhow!("falha ao iniciar sodiumoxide"))?;
        let mut assinado = Vec::with_capacity(CONTEXTO_ASSINATURA.len() + dados.len());
        assinado.extend_from_slice(CONTEXTO_ASSINATURA);
        assinado.extend_from_slice(dados);
        if !verify_detached(&assinatura, &assinado, &PublicKey(CHAVE_PUBLICA)) {
            bail!("assinatura do versao.json não confere");
        }
        Ok(())
    }

    fn versao_publicada() -> ResultType<VersaoPublicada> {
        let dados = baixar(URL_VERSAO)?;
        let assinatura = baixar(URL_ASSINATURA)?;
        conferir_assinatura(&dados, &assinatura)?;
        let v: VersaoPublicada = serde_json::from_slice(&dados).context("versao.json inválido")?;
        let sha256 = v.sha256.to_ascii_lowercase();
        if v.app != APP
            || !v.url.starts_with(PREFIXO_DOWNLOAD)
            || !v.pagina.starts_with(PREFIXO_PAGINA)
            || v.tamanho == 0
            || sha256.len() != 64
            || !sha256.bytes().all(|b| b.is_ascii_hexdigit())
        {
            bail!("versao.json com dados inesperados");
        }
        Ok(VersaoPublicada { sha256, ..v })
    }

    /// Página do Release com build mais novo que o instalado (ou vazio).
    fn conferir() -> ResultType<String> {
        let v = versao_publicada()?;
        if v.build > numero_build() {
            Ok(v.pagina)
        } else {
            Ok(String::new())
        }
    }

    fn avisar(pagina: &str) {
        *crate::common::SOFTWARE_UPDATE_URL.lock().unwrap() = pagina.to_owned();
        #[cfg(feature = "flutter")]
        if !pagina.is_empty() {
            let data = serde_json::json!({
                "name": "check_software_update_finish",
                "url": pagina,
            });
            let _ = crate::flutter::push_global_event(
                crate::flutter::APP_TYPE_MAIN,
                data.to_string(),
            );
        }
    }

    /// Confere ao abrir e depois a cada 6 horas (o Cliente fica aberto na bandeja).
    pub fn checar_atualizacao() {
        static INICIADO: AtomicBool = AtomicBool::new(false);
        if INICIADO.swap(true, Ordering::SeqCst) {
            let pagina = crate::common::SOFTWARE_UPDATE_URL.lock().unwrap().clone();
            avisar(&pagina);
            return;
        }
        std::thread::spawn(|| loop {
            match conferir() {
                Ok(pagina) => avisar(&pagina),
                Err(e) => log::warn!("GPROTECH: não foi possível conferir atualização: {e}"),
            }
            std::thread::sleep(INTERVALO);
        });
    }

    /// Link "Procurar atualizações" do Cliente: confere agora, sem esperar as 6 horas.
    /// Devolve "nova:<página do Release>", "atual" ou "erro".
    pub fn procurar_agora() -> String {
        match conferir() {
            Ok(pagina) => {
                avisar(&pagina);
                if pagina.is_empty() {
                    "atual".to_owned()
                } else {
                    format!("nova:{pagina}")
                }
            }
            Err(e) => {
                log::warn!("GPROTECH: não foi possível conferir atualização: {e}");
                "erro".to_owned()
            }
        }
    }

    /// Arquivo a baixar quando o usuário clica em "Atualizar" (confere de novo).
    pub(crate) fn artefato_atualizacao() -> ResultType<crate::update_metadata::VerifiedUpdateArtifact> {
        let v = versao_publicada()?;
        if v.build <= numero_build() {
            bail!("Não há versão nova.");
        }
        Ok(crate::update_metadata::VerifiedUpdateArtifact {
            version: format!("build {}", v.build),
            url: v.url,
            size: v.tamanho,
            sha256: v.sha256,
        })
    }
}

#[cfg(windows)]
pub(crate) use atualizacao::artefato_atualizacao;
#[cfg(windows)]
pub use atualizacao::{checar_atualizacao, procurar_agora, REPO_DONO, REPO_NOME};
