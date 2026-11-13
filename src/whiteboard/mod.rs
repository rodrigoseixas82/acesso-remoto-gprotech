use serde_derive::{Deserialize, Serialize};

mod client;
mod server;

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(any(target_os = "windows", target_os = "linux"))]
mod win_linux;
// GPROTECH: caneta de anotação (só desenhada no Windows).
#[cfg(target_os = "windows")]
mod anotacao;

#[cfg(target_os = "windows")]
use windows::create_event_loop;
#[cfg(target_os = "macos")]
use macos::create_event_loop;
#[cfg(target_os = "linux")]
pub use linux::is_supported;

pub use client::*;
pub use server::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "t", content = "c")]
pub enum CustomEvent {
    Cursor(Cursor),
    // GPROTECH: anotação do técnico (caneta).
    Anotacao(Anotacao),
    Clear,
    Exit,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "t")]
pub struct Cursor {
    pub x: f32,
    pub y: f32,
    pub argb: u32,
    pub btns: i32,
    pub text: String,
}

/// GPROTECH: um pedaço de anotação vindo do técnico. Ver `GprotechAnotacao`
/// em `message.proto`.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Anotacao {
    pub acao: i32,
    pub id: u32,
    pub ferramenta: u32,
    pub argb: u32,
    pub espessura: u32,
    pub sumir: bool,
    pub fim: bool,
    pub pontos: Vec<(f32, f32)>,
    /// Nome do técnico, para o aviso no topo da tela.
    pub nome: String,
}
