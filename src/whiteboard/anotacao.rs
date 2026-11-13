// GPROTECH: caneta de anotação. Guarda os traços que o técnico desenha e os
// pinta na camada transparente que fica por cima da tela do cliente.
use super::{
    server::argb_to_rgba,
    win_linux::{caminho_texto, largura_texto},
    Anotacao,
};
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};
use tiny_skia::{
    FillRule, LineCap, LineJoin, Paint, PathBuilder, PixmapMut, Rect, Stroke, Transform,
};
use ttf_parser::Face;

// Mesmos valores de `GprotechAnotacao.Acao` em message.proto.
const ACAO_TRACO: i32 = 0;
const ACAO_DESFAZER: i32 = 1;
const ACAO_LIMPAR: i32 = 2;
const ACAO_LIGAR: i32 = 3;
const ACAO_DESLIGAR: i32 = 4;

const FERRAMENTA_SETA: u32 = 1;
const FERRAMENTA_RETANGULO: u32 = 2;
const FERRAMENTA_CIRCULO: u32 = 3;

/// Com "sumir" ligado, o traço fica 5 s e depois apaga aos poucos.
const SUMIR_APOS: Duration = Duration::from_secs(5);
const SUMIR_DURACAO: Duration = Duration::from_millis(600);
/// Limite de traços guardados por conexão.
const MAX_TRACOS: usize = 300;
/// Limite de pontos de um traço livre.
const MAX_PONTOS: usize = 20_000;

struct Traco {
    id: u32,
    ferramenta: u32,
    argb: u32,
    espessura: f32,
    sumir: bool,
    fim: Option<Instant>,
    pontos: Vec<(f32, f32)>,
}

impl Traco {
    fn alfa(&self, agora: Instant) -> f32 {
        if !self.sumir {
            return 1.0;
        }
        let Some(fim) = self.fim else {
            return 1.0;
        };
        let passou = agora.saturating_duration_since(fim);
        if passou <= SUMIR_APOS {
            return 1.0;
        }
        let fracao = (passou - SUMIR_APOS).as_secs_f32() / SUMIR_DURACAO.as_secs_f32();
        (1.0 - fracao).max(0.0)
    }
}

#[derive(Default)]
struct Conjunto {
    nome: String,
    tracos: Vec<Traco>,
}

#[derive(Default)]
pub(super) struct Anotacoes {
    por_conexao: HashMap<String, Conjunto>,
}

impl Anotacoes {
    pub fn aplicar(&mut self, k: String, a: Anotacao) {
        match a.acao {
            ACAO_TRACO => {
                let conjunto = self.por_conexao.entry(k).or_default();
                conjunto.nome = a.nome;
                let pontos: Vec<(f32, f32)> = a
                    .pontos
                    .into_iter()
                    .filter(|(x, y)| x.is_finite() && y.is_finite())
                    .collect();
                if let Some(t) = conjunto.tracos.iter_mut().find(|t| t.id == a.id) {
                    if t.ferramenta == 0 {
                        if t.pontos.len() + pontos.len() <= MAX_PONTOS {
                            t.pontos.extend(pontos);
                        }
                    } else if !pontos.is_empty() {
                        t.pontos = pontos;
                    }
                    if a.fim && t.fim.is_none() {
                        t.fim = Some(Instant::now());
                    }
                } else {
                    conjunto.tracos.push(Traco {
                        id: a.id,
                        ferramenta: a.ferramenta,
                        argb: a.argb,
                        espessura: (a.espessura as f32).clamp(1.0, 40.0),
                        sumir: a.sumir,
                        fim: if a.fim { Some(Instant::now()) } else { None },
                        pontos: pontos.into_iter().take(MAX_PONTOS).collect(),
                    });
                    if conjunto.tracos.len() > MAX_TRACOS {
                        conjunto.tracos.remove(0);
                    }
                }
            }
            ACAO_DESFAZER => {
                if let Some(conjunto) = self.por_conexao.get_mut(&k) {
                    conjunto.tracos.pop();
                }
            }
            ACAO_LIMPAR | ACAO_DESLIGAR => {
                self.por_conexao.remove(&k);
            }
            ACAO_LIGAR => {}
            _ => {}
        }
    }

    pub fn limpar(&mut self, k: &str) {
        self.por_conexao.remove(k);
    }

    /// `origem`: canto de cima à esquerda da camada, nas coordenadas da tela.
    /// `telas`: (x, y, largura, altura) de cada monitor, para pôr o aviso.
    pub fn desenhar(
        &mut self,
        pixmap: &mut PixmapMut,
        origem: (f32, f32),
        escala: f32,
        face: Option<&Face>,
        telas: &[(i32, i32, u32, u32)],
    ) {
        let agora = Instant::now();
        for conjunto in self.por_conexao.values_mut() {
            conjunto.tracos.retain(|t| t.alfa(agora) > 0.0);
        }
        self.por_conexao.retain(|_, c| !c.tracos.is_empty());

        for conjunto in self.por_conexao.values() {
            for traco in &conjunto.tracos {
                desenhar_traco(pixmap, traco, origem, escala, agora);
            }
        }

        if let Some(face) = face {
            if let Some(conjunto) = self.por_conexao.values().next() {
                let ponto = conjunto.tracos.last().and_then(|t| t.pontos.first().copied());
                desenhar_aviso(pixmap, face, &conjunto.nome, origem, escala, telas, ponto);
            }
        }
    }
}

fn desenhar_traco(
    pixmap: &mut PixmapMut,
    traco: &Traco,
    origem: (f32, f32),
    escala: f32,
    agora: Instant,
) {
    if traco.pontos.is_empty() {
        return;
    }
    let alfa = traco.alfa(agora);
    if alfa <= 0.0 {
        return;
    }
    let (r, g, b, a) = argb_to_rgba(traco.argb);
    let mut paint = Paint::default();
    // Note: The real color is bgra here.
    paint.set_color_rgba8(b, g, r, (a as f32 * alfa) as u8);
    paint.anti_alias = true;
    let largura = (traco.espessura * escala).max(1.0);
    let mut stroke = Stroke::default();
    stroke.width = largura;
    stroke.line_cap = LineCap::Round;
    stroke.line_join = LineJoin::Round;

    let p: Vec<(f32, f32)> = traco
        .pontos
        .iter()
        .map(|(x, y)| (x - origem.0, y - origem.1))
        .collect();
    let inicio = p[0];
    let fim = p[p.len() - 1];
    let (esq, dir) = (inicio.0.min(fim.0), inicio.0.max(fim.0));
    let (topo, baixo) = (inicio.1.min(fim.1), inicio.1.max(fim.1));

    let mut pb = PathBuilder::new();
    match traco.ferramenta {
        FERRAMENTA_SETA => {
            let (dx, dy) = (fim.0 - inicio.0, fim.1 - inicio.1);
            let comprimento = (dx * dx + dy * dy).sqrt();
            let ang = dy.atan2(dx);
            let ponta = largura * 3.2 + 6.0 * escala;
            // A linha para na base da ponta, para a ponta ficar afiada.
            let recuo = if comprimento > ponta { ponta * 0.8 } else { 0.0 };
            pb.move_to(inicio.0, inicio.1);
            pb.line_to(fim.0 - recuo * ang.cos(), fim.1 - recuo * ang.sin());
            let mut cabeca = PathBuilder::new();
            cabeca.move_to(fim.0, fim.1);
            cabeca.line_to(
                fim.0 - ponta * (ang - 0.45).cos(),
                fim.1 - ponta * (ang - 0.45).sin(),
            );
            cabeca.line_to(
                fim.0 - ponta * (ang + 0.45).cos(),
                fim.1 - ponta * (ang + 0.45).sin(),
            );
            cabeca.close();
            if let Some(path) = cabeca.finish() {
                pixmap.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);
            }
        }
        FERRAMENTA_RETANGULO => {
            pb.move_to(esq, topo);
            pb.line_to(dir, topo);
            pb.line_to(dir, baixo);
            pb.line_to(esq, baixo);
            pb.close();
        }
        FERRAMENTA_CIRCULO => {
            if let Some(rect) = Rect::from_ltrb(esq, topo, dir.max(esq + 1.0), baixo.max(topo + 1.0)) {
                pb.push_oval(rect);
            }
        }
        _ => {
            pb.move_to(inicio.0, inicio.1);
            if p.len() == 1 {
                // Um clique só: um ponto.
                pb.line_to(inicio.0 + 0.1, inicio.1 + 0.1);
            } else {
                for pt in &p[1..] {
                    pb.line_to(pt.0, pt.1);
                }
            }
        }
    }
    if let Some(path) = pb.finish() {
        pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
    }
}

/// Aviso no topo do monitor onde está o desenho:
/// "<técnico> está fazendo anotações na sua tela".
fn desenhar_aviso(
    pixmap: &mut PixmapMut,
    face: &Face,
    nome: &str,
    origem: (f32, f32),
    escala: f32,
    telas: &[(i32, i32, u32, u32)],
    ponto: Option<(f32, f32)>,
) {
    let nome = nome.trim();
    let texto = if nome.is_empty() {
        "O técnico está fazendo anotações na sua tela".to_owned()
    } else {
        format!("{} está fazendo anotações na sua tela", nome)
    };
    let tamanho = 14.0 * escala;
    let largura_txt = largura_texto(face, &texto, tamanho);

    let tela = ponto
        .and_then(|(x, y)| {
            telas.iter().find(|(tx, ty, tw, th)| {
                x >= *tx as f32
                    && y >= *ty as f32
                    && x < (*tx as f32 + *tw as f32)
                    && y < (*ty as f32 + *th as f32)
            })
        })
        .or(telas.first());
    let Some((tx, ty, tw, _)) = tela else {
        return;
    };
    let centro_x = *tx as f32 + *tw as f32 / 2.0 - origem.0;
    let topo_tela = *ty as f32 - origem.1;

    let folga = 14.0 * escala;
    let altura = tamanho * 2.2;
    let largura = largura_txt + folga * 2.0;
    let x0 = centro_x - largura / 2.0;
    let y0 = topo_tela + 12.0 * escala;
    let raio = altura / 2.0;

    let mut pb = PathBuilder::new();
    pb.move_to(x0 + raio, y0);
    pb.line_to(x0 + largura - raio, y0);
    pb.quad_to(x0 + largura, y0, x0 + largura, y0 + raio);
    pb.quad_to(x0 + largura, y0 + altura, x0 + largura - raio, y0 + altura);
    pb.line_to(x0 + raio, y0 + altura);
    pb.quad_to(x0, y0 + altura, x0, y0 + raio);
    pb.quad_to(x0, y0, x0 + raio, y0);
    pb.close();
    if let Some(path) = pb.finish() {
        // Note: The real color is bgra here. Fundo #001A41, borda #37FFFF.
        let mut fundo = Paint::default();
        fundo.set_color_rgba8(0x41, 0x1A, 0x00, 235);
        fundo.anti_alias = true;
        pixmap.fill_path(&path, &fundo, FillRule::Winding, Transform::identity(), None);
        let mut borda = Paint::default();
        borda.set_color_rgba8(0xFF, 0xFF, 0x37, 255);
        borda.anti_alias = true;
        let mut stroke = Stroke::default();
        stroke.width = 1.5 * escala;
        pixmap.stroke_path(&path, &borda, &stroke, Transform::identity(), None);
    }

    let base = y0 + altura / 2.0 + tamanho * 0.35;
    if let Some(path) = caminho_texto(face, &texto, x0 + folga, base, tamanho) {
        let mut branco = Paint::default();
        branco.set_color_rgba8(255, 255, 255, 255);
        branco.anti_alias = true;
        pixmap.fill_path(&path, &branco, FillRule::Winding, Transform::identity(), None);
    }
}
