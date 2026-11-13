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

    // GPROTECH: com a senha por assinatura (build com GPROTECH_CHAVE_MESTRA), o hash
    // da senha interna NÃO entra no programa: quem confere o login é `chave_acesso`.
    let senha_assinada = chave_acesso::ativo();
    let preset = (
        non_empty(option_env!("GPROTECH_PRESET_PASSWORD_HASH")),
        non_empty(option_env!("GPROTECH_PRESET_SALT")),
    );
    if senha_assinada || matches!(preset, (Some(_), Some(_))) {
        if let (false, (Some(hash), Some(salt))) = (senha_assinada, preset) {
            hard.insert("password".to_owned(), hash.trim().to_owned());
            hard.insert("salt".to_owned(), salt.trim().to_owned());
        }
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

/// Senha dos técnicos por assinatura (no lugar da senha interna embutida).
///
/// A senha única dos técnicos vira, por Argon2id, a semente de um par de chaves
/// ed25519. A loja guarda só a **chave pública**, que confere mas não assina: nada
/// do que fica no Cliente serve para entrar em outra loja.
///
/// - A chave pública vem do arquivo `chave-acesso.json`, publicado pela área admin
///   (acesso.gprotech.com.br/admin) e assinado no navegador do admin com a frase
///   mestra. O Cliente só aceita o arquivo se a assinatura bater com a chave mestra
///   pública embutida no build (`GPROTECH_CHAVE_MESTRA`) e se o número da troca
///   (`seq`) for maior que o que já tem. Assim a senha muda sem gerar versão nova.
/// - No login, a loja manda no `salt` do `Hash` o texto `gp1:<salt base64>:<seq>`.
///   O Atendente reconhece o prefixo, deriva a semente da senha digitada e assina
///   `CTX_LOGIN + id da loja + "\n" + salt + "\n" + challenge`. A senha não sai do
///   computador do técnico e a assinatura só vale para aquela loja e aquele desafio.
/// - Loja antiga (salt sem o prefixo): o Atendente usa o login normal do RustDesk.
///
/// Os parâmetros (OPS, MEM e os contextos) precisam ser iguais aos da página admin
/// (servidor/admin/public/admin.js no projeto).
pub mod chave_acesso {
    use super::{is_edicao_cliente, non_empty};
    use hbb_common::{
        anyhow::{anyhow, Context},
        base64::{engine::general_purpose::STANDARD, Engine as _},
        bail, config, log,
        sodiumoxide::crypto::{pwhash::argon2id13, sign},
        ResultType,
    };
    use std::{
        io::Read,
        sync::{Once, RwLock},
        time::Duration,
    };

    pub const PREFIXO_SALT: &str = "gp1:";
    const OPS: usize = 3;
    const MEM: usize = 64 * 1024 * 1024;
    const CTX_CHAVE: &[u8] = b"GPROTECH chave-acesso v1\n";
    const CTX_LOGIN: &[u8] = b"GPROTECH login v1\n";
    const URL_CHAVE: &str = "https://acesso.gprotech.com.br/suporte/chave-acesso.json";
    const URL_ASSINATURA: &str = "https://acesso.gprotech.com.br/suporte/chave-acesso.json.sig";
    const ARQUIVO_LOCAL: &str = "gprotech-chave-acesso.json";
    const INTERVALO: Duration = Duration::from_secs(5 * 60);
    const LIMITE_BYTES: u64 = 16 * 1024;
    /// Senha lembrada no Atendente: "gp1" + salt (16) + semente (32).
    const MARCA_LEMBRADA: &[u8] = b"gp1";

    #[derive(Clone)]
    struct Chave {
        seq: u64,
        salt_login: String,
        publica: sign::PublicKey,
    }

    lazy_static::lazy_static! {
        static ref ATUAL: RwLock<Option<Chave>> = RwLock::new(None);
    }

    fn chave_mestra() -> Option<sign::PublicKey> {
        let texto = non_empty(option_env!("GPROTECH_CHAVE_MESTRA"))?;
        let bytes = STANDARD.decode(texto.trim()).ok()?;
        sign::PublicKey::from_slice(&bytes)
    }

    /// Cliente com a senha por assinatura (build com a chave mestra).
    pub fn ativo() -> bool {
        is_edicao_cliente() && chave_mestra().is_some()
    }

    #[derive(serde_derive::Deserialize)]
    struct ArquivoChave {
        tipo: String,
        seq: u64,
        salt: String,
        chave: String,
    }

    fn validar(json: &[u8], assinatura_b64: &str) -> ResultType<Chave> {
        let mestra = chave_mestra().ok_or_else(|| anyhow!("build sem chave mestra"))?;
        let assinatura = STANDARD
            .decode(assinatura_b64.trim())
            .context("assinatura não está em base64")?;
        let assinatura = sign::Signature::from_bytes(&assinatura)
            .map_err(|_| anyhow!("assinatura com tamanho errado"))?;
        hbb_common::sodiumoxide::init().map_err(|_| anyhow!("falha ao iniciar sodiumoxide"))?;
        let mut assinado = Vec::with_capacity(CTX_CHAVE.len() + json.len());
        assinado.extend_from_slice(CTX_CHAVE);
        assinado.extend_from_slice(json);
        if !sign::verify_detached(&assinatura, &assinado, &mestra) {
            bail!("assinatura da chave de acesso não confere");
        }
        let v: ArquivoChave = serde_json::from_slice(json).context("chave-acesso.json inválido")?;
        let salt = STANDARD.decode(v.salt.trim()).context("salt inválido")?;
        let publica = STANDARD.decode(v.chave.trim()).context("chave inválida")?;
        if v.tipo != "gprotech-chave-acesso" || v.seq == 0 || salt.len() != 16 {
            bail!("chave-acesso.json com dados inesperados");
        }
        let publica =
            sign::PublicKey::from_slice(&publica).ok_or_else(|| anyhow!("chave com tamanho errado"))?;
        Ok(Chave {
            seq: v.seq,
            salt_login: format!("{PREFIXO_SALT}{}:{}", STANDARD.encode(&salt), v.seq),
            publica,
        })
    }

    /// Troca a chave em uso se a recebida for mais nova. Devolve true se trocou.
    fn aplicar(json: &[u8], assinatura: &str) -> ResultType<bool> {
        let nova = validar(json, assinatura)?;
        let mut atual = ATUAL.write().unwrap();
        if atual.as_ref().map(|c| c.seq >= nova.seq).unwrap_or(false) {
            return Ok(false);
        }
        log::info!("GPROTECH: chave de acesso nº {} em uso", nova.seq);
        *atual = Some(nova);
        Ok(true)
    }

    fn caminho_local() -> std::path::PathBuf {
        config::Config::path(ARQUIVO_LOCAL)
    }

    fn carregar_local() {
        let Ok(texto) = std::fs::read_to_string(caminho_local()) else {
            return;
        };
        let v: serde_json::Value = match serde_json::from_str(&texto) {
            Ok(v) => v,
            Err(_) => return,
        };
        let json = v.get("json").and_then(|x| x.as_str()).unwrap_or_default();
        let sig = v.get("sig").and_then(|x| x.as_str()).unwrap_or_default();
        if let Err(e) = aplicar(json.as_bytes(), sig) {
            log::warn!("GPROTECH: chave de acesso guardada recusada: {e}");
        }
    }

    fn baixar(url: &str) -> ResultType<Vec<u8>> {
        let client = crate::hbbs_http::create_http_client_with_url_strict(url)?;
        let response = client.get(url).timeout(Duration::from_secs(20)).send()?;
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

    fn baixar_e_aplicar() -> ResultType<()> {
        let json = baixar(URL_CHAVE)?;
        let sig = String::from_utf8(baixar(URL_ASSINATURA)?).context("assinatura inválida")?;
        if aplicar(&json, &sig)? {
            let guardar = serde_json::json!({
                "json": String::from_utf8_lossy(&json),
                "sig": sig.trim(),
            });
            if let Err(e) = std::fs::write(caminho_local(), guardar.to_string()) {
                log::warn!("GPROTECH: não foi possível guardar a chave de acesso: {e}");
            }
        }
        Ok(())
    }

    /// Carrega a chave guardada e confere a da VPS ao iniciar e a cada 5 minutos.
    pub fn iniciar() {
        static ONCE: Once = Once::new();
        if !ativo() {
            return;
        }
        ONCE.call_once(|| {
            carregar_local();
            std::thread::spawn(|| loop {
                if let Err(e) = baixar_e_aplicar() {
                    log::warn!("GPROTECH: não foi possível conferir a chave de acesso: {e}");
                }
                std::thread::sleep(INTERVALO);
            });
        });
    }

    /// Texto que a loja manda no `salt` do `Hash` (None: sem chave carregada).
    pub fn salt_para_login() -> Option<String> {
        if !ativo() {
            return None;
        }
        iniciar();
        ATUAL.read().unwrap().as_ref().map(|c| c.salt_login.clone())
    }

    fn mensagem_login(id_loja: &str, salt: &str, challenge: &str) -> Vec<u8> {
        let mut m = Vec::with_capacity(CTX_LOGIN.len() + id_loja.len() + salt.len() + challenge.len() + 2);
        m.extend_from_slice(CTX_LOGIN);
        m.extend_from_slice(id_loja.as_bytes());
        m.push(b'\n');
        m.extend_from_slice(salt.as_bytes());
        m.push(b'\n');
        m.extend_from_slice(challenge.as_bytes());
        m
    }

    /// Lado da loja: confere a assinatura que o Atendente mandou no campo `password`.
    pub fn conferir_login(id_loja: &str, salt: &str, challenge: &str, assinatura: &[u8]) -> bool {
        let Some(chave) = ATUAL.read().unwrap().clone() else {
            log::warn!("GPROTECH: login recusado, nenhuma chave de acesso carregada");
            return false;
        };
        if chave.salt_login != salt {
            // A chave trocou entre o pedido e o login: o Atendente tenta de novo.
            return false;
        }
        let Ok(assinatura) = sign::Signature::from_bytes(assinatura) else {
            return false;
        };
        sign::verify_detached(&assinatura, &mensagem_login(id_loja, salt, challenge), &chave.publica)
    }

    // ---------- lado do Atendente ----------

    /// Salt de Argon2 se a loja usa a senha por assinatura.
    pub fn salt_do_login(salt: &str) -> Option<[u8; 16]> {
        let resto = salt.strip_prefix(PREFIXO_SALT)?;
        let b64 = resto.split(':').next()?;
        STANDARD.decode(b64).ok()?.try_into().ok()
    }

    /// Semente do par de chaves a partir da senha digitada (~0,1–0,5 s).
    pub fn derivar_semente(senha: &str, salt: &[u8; 16]) -> Option<[u8; 32]> {
        hbb_common::sodiumoxide::init().ok()?;
        let mut semente = [0u8; 32];
        argon2id13::derive_key(
            &mut semente,
            senha.as_bytes(),
            &argon2id13::Salt(*salt),
            argon2id13::OpsLimit(OPS),
            argon2id13::MemLimit(MEM),
        )
        .ok()?;
        Some(semente)
    }

    pub fn assinar(semente: &[u8; 32], id_loja: &str, salt: &str, challenge: &str) -> Vec<u8> {
        let (_, secreta) = sign::keypair_from_seed(&sign::Seed(*semente));
        sign::sign_detached(&mensagem_login(id_loja, salt, challenge), &secreta)
            .to_bytes()
            .to_vec()
    }

    /// O que o Atendente guarda como "senha lembrada" (vale só para esse salt).
    pub fn montar_lembrada(salt: &[u8; 16], semente: &[u8; 32]) -> Vec<u8> {
        [MARCA_LEMBRADA, &salt[..], &semente[..]].concat()
    }

    pub fn ler_lembrada(guardada: &[u8], salt: &[u8; 16]) -> Option<[u8; 32]> {
        let resto = guardada.strip_prefix(MARCA_LEMBRADA)?;
        if resto.len() != 48 || &resto[..16] != &salt[..] {
            return None;
        }
        resto[16..].try_into().ok()
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        // Mesmo resultado da página admin (libsodium.js): senha "teste", salt zerado.
        #[test]
        fn semente_igual_a_da_pagina_admin() {
            let semente = derivar_semente("teste", &[0u8; 16]).unwrap();
            let (publica, _) = sign::keypair_from_seed(&sign::Seed(semente));
            assert_eq!(
                STANDARD.encode(publica.0),
                "t8QriWRIxThWHWSmuOL3QnWsvdcCQde3AeS2RzsessI="
            );
        }

        #[test]
        fn assinatura_vale_so_para_a_loja_e_o_desafio() {
            let salt = [7u8; 16];
            let semente = derivar_semente("senha dos tecnicos", &salt).unwrap();
            let (publica, _) = sign::keypair_from_seed(&sign::Seed(semente));
            let salt_login = format!("{PREFIXO_SALT}{}:1", STANDARD.encode(salt));
            assert_eq!(salt_do_login(&salt_login), Some(salt));
            let a = assinar(&semente, "123456789", &salt_login, "abcdef");
            let ok = |id: &str, ch: &str| {
                sign::verify_detached(
                    &sign::Signature::from_bytes(&a).unwrap(),
                    &mensagem_login(id, &salt_login, ch),
                    &publica,
                )
            };
            assert!(ok("123456789", "abcdef"));
            assert!(!ok("987654321", "abcdef"));
            assert!(!ok("123456789", "zzzzzz"));
            let lembrada = montar_lembrada(&salt, &semente);
            assert_eq!(ler_lembrada(&lembrada, &salt), Some(semente));
            assert_eq!(ler_lembrada(&lembrada, &[8u8; 16]), None);
        }
    }
}

/// Registro dos acessos na área admin (em andamento e recentes). Só no Cliente.
/// Cada evento vai assinado com a chave da própria loja (a registrada no hbbs),
/// e a VPS confere com a chave que o hbbs tem para aquele ID.
pub mod eventos {
    use super::is_edicao_cliente;
    use hbb_common::{
        base64::{engine::general_purpose::STANDARD, Engine as _},
        config::Config,
        log,
        sodiumoxide::crypto::sign,
    };
    use std::{
        collections::HashMap,
        sync::{
            atomic::{AtomicBool, Ordering},
            Arc, Mutex,
        },
        time::Duration,
    };

    const URL: &str = "https://acesso.gprotech.com.br/suporte/api/evento";
    const CTX_EVENTO: &[u8] = b"GPROTECH evento v1\n";
    const INTERVALO_ATIVO: Duration = Duration::from_secs(60);

    struct Sessao {
        id: String,
        tecnico_nome: String,
        tecnico_id: String,
        ip: String,
        vivo: Arc<AtomicBool>,
    }

    lazy_static::lazy_static! {
        static ref SESSOES: Mutex<HashMap<i32, Sessao>> = Default::default();
    }

    fn enviar(tipo: &str, s: &Sessao) {
        let corpo = serde_json::json!({
            "tipo": tipo,
            "sessao": s.id,
            "loja_id": Config::get_id(),
            "loja_nome": crate::common::hostname(),
            "tecnico_nome": s.tecnico_nome,
            "tecnico_id": s.tecnico_id,
            "ip": s.ip,
            "ts": hbb_common::get_time() / 1000,
        })
        .to_string();
        let (secreta, _) = Config::get_key_pair();
        let Some(secreta) = sign::SecretKey::from_slice(&secreta) else {
            return;
        };
        let assinado = [CTX_EVENTO, corpo.as_bytes()].concat();
        let assinatura = STANDARD.encode(sign::sign_detached(&assinado, &secreta).to_bytes());
        let resultado = crate::hbbs_http::create_http_client_with_url_strict(URL).and_then(|c| {
            Ok(c.post(URL)
                .header("Content-Type", "application/json")
                .header("X-GPROTECH-Assinatura", assinatura)
                .body(corpo)
                .timeout(Duration::from_secs(15))
                .send()?)
        });
        match resultado {
            Ok(r) if r.status().is_success() => {}
            Ok(r) => log::warn!("GPROTECH: evento '{tipo}' recusado: HTTP {}", r.status()),
            Err(e) => log::warn!("GPROTECH: evento '{tipo}' não enviado: {e}"),
        }
    }

    /// Acesso autorizado: avisa a VPS e depois manda "ativo" a cada minuto.
    pub fn inicio(conn_id: i32, tecnico_nome: &str, tecnico_id: &str, ip: &str) {
        if !is_edicao_cliente() {
            return;
        }
        let mut sessoes = SESSOES.lock().unwrap();
        if sessoes.contains_key(&conn_id) {
            return;
        }
        let vivo = Arc::new(AtomicBool::new(true));
        let sessao = Sessao {
            id: format!("{}-{}-{}", Config::get_id(), hbb_common::get_time(), conn_id),
            tecnico_nome: tecnico_nome.chars().take(120).collect(),
            tecnico_id: tecnico_id.chars().take(64).collect(),
            ip: ip.trim_start_matches("::ffff:").chars().take(64).collect(),
            vivo: vivo.clone(),
        };
        let copia = Sessao {
            id: sessao.id.clone(),
            tecnico_nome: sessao.tecnico_nome.clone(),
            tecnico_id: sessao.tecnico_id.clone(),
            ip: sessao.ip.clone(),
            vivo: vivo.clone(),
        };
        sessoes.insert(conn_id, sessao);
        drop(sessoes);
        std::thread::spawn(move || {
            enviar("inicio", &copia);
            loop {
                std::thread::sleep(INTERVALO_ATIVO);
                if !copia.vivo.load(Ordering::SeqCst) {
                    break;
                }
                enviar("ativo", &copia);
            }
        });
    }

    /// Conexão encerrada (chamado no fechamento e no Drop; só envia uma vez).
    pub fn fim(conn_id: i32) {
        let Some(s) = SESSOES.lock().unwrap().remove(&conn_id) else {
            return;
        };
        s.vivo.store(false, Ordering::SeqCst);
        std::thread::spawn(move || enviar("fim", &s));
    }
}

/// Caneta de anotação: monta a mensagem para o cliente a partir do JSON que o
/// Atendente manda: `{acao, id, ferramenta, argb, espessura, sumir, fim, pontos}`,
/// com `pontos` = [x0, y0, x1, y1, ...] nas coordenadas da tela remota.
pub fn mensagem_anotacao(json: &str) -> Option<base::message_proto::Message> {
    use base::message_proto::{gprotech_anotacao::Acao, GprotechAnotacao, Message};
    let v: serde_json::Value = serde_json::from_str(json).ok()?;
    let numero = |chave: &str| v.get(chave).and_then(|x| x.as_f64()).unwrap_or(0.0);
    let logico = |chave: &str| v.get(chave).and_then(|x| x.as_bool()).unwrap_or(false);
    let mut anotacao = GprotechAnotacao::new();
    anotacao.acao = match numero("acao") as i64 {
        1 => Acao::Desfazer,
        2 => Acao::Limpar,
        3 => Acao::Ligar,
        4 => Acao::Desligar,
        _ => Acao::Traco,
    }
    .into();
    anotacao.id = numero("id") as u32;
    anotacao.ferramenta = numero("ferramenta") as u32;
    anotacao.argb = numero("argb") as u32;
    anotacao.espessura = numero("espessura") as u32;
    anotacao.sumir = logico("sumir");
    anotacao.fim = logico("fim");
    anotacao.pontos = v
        .get("pontos")
        .and_then(|p| p.as_array())
        .map(|p| {
            p.iter()
                .filter_map(|x| x.as_f64())
                .map(|x| x.round() as i32)
                .collect()
        })
        .unwrap_or_default();
    let mut msg = Message::new();
    msg.set_gprotech_anotacao(anotacao);
    Some(msg)
}
