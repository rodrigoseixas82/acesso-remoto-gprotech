// GPROTECH: janela de loading no visual GPROTECH (proposta B): fundo azul-marinho
// com logo, "Abrindo o Suporte Remoto" + edição (imagem por edição, escolhida na
// compilação por GPROTECH_EDICAO) e uma barra ciano correndo embaixo, desenhada
// aqui (sem gif). Imagens geradas por scripts/gerar-loading.js no projeto.
use native_windows_gui as nwg;
use nwg::NativeUi;
use std::cell::RefCell;

const WIN_W: i32 = 300;
const WIN_H: i32 = 132;
const RADIUS: i32 = 16; // diâmetro do canto arredondado (8 px de raio)
const TRACK_X: i32 = 24;
const TRACK_Y: i32 = 112;
const TRACK_W: i32 = 252;
const TRACK_H: i32 = 4;
const BAR_W: i32 = 101; // 40% da trilha
const TRACK_COLOR: [u8; 3] = [0x18, 0x2F, 0x53];
const BAR_COLOR: [u8; 3] = [0x37, 0xFF, 0xFF];
const TICK_MS: u64 = 30;
const CYCLE_TICKS: usize = 37; // ~1,1 s por passada

fn bg_data() -> &'static [u8] {
    match option_env!("GPROTECH_EDICAO") {
        Some("atendente") => include_bytes!("./res/loading-atendente.png"),
        _ => include_bytes!("./res/loading-cliente.png"),
    }
}

#[derive(Default)]
pub struct BasicApp {
    window: nwg::Window,

    bg_image: nwg::ImageFrame,
    track: nwg::ImageFrame,
    bar: nwg::ImageFrame,

    timer: nwg::AnimationTimer,
    tick: RefCell<usize>,
}

impl BasicApp {
    fn exit(&self) {
        self.timer.stop();
        nwg::stop_thread_dispatch();
    }

    // Barra entra pela esquerda e sai pela direita, com aceleração suave.
    fn update_bar(&self) {
        let i = *self.tick.borrow() % CYCLE_TICKS;
        *self.tick.borrow_mut() = i + 1;
        let t = i as f64 / CYCLE_TICKS as f64;
        let e = if t < 0.5 {
            4.0 * t * t * t
        } else {
            1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
        };
        let x = -BAR_W + (e * (TRACK_W + BAR_W) as f64) as i32;
        self.bar.set_position(x, 0);
    }

    fn start_timer(&self) {
        self.timer.start();
    }
}

mod basic_app_ui {
    use super::*;
    use native_windows_gui::{self as nwg, Bitmap};
    use nwg::Event;
    use std::cell::RefCell;
    use std::ops::Deref;
    use std::rc::Rc;

    pub struct BasicAppUi {
        inner: Rc<BasicApp>,
        default_handler: RefCell<Vec<nwg::EventHandler>>,
    }

    impl nwg::NativeUi<BasicAppUi> for BasicApp {
        fn build_ui(mut data: BasicApp) -> Result<BasicAppUi, nwg::NwgError> {
            // Controls
            nwg::Window::builder()
                .flags(nwg::WindowFlags::POPUP | nwg::WindowFlags::VISIBLE)
                .size((WIN_W, WIN_H))
                .center(true)
                .build(&mut data.window)?;

            // Cantos arredondados: recorta a janela (a borda já vem desenhada na imagem).
            if let Some(hwnd) = data.window.handle.hwnd() {
                unsafe {
                    let rgn = winapi::um::wingdi::CreateRoundRectRgn(
                        0,
                        0,
                        WIN_W + 1,
                        WIN_H + 1,
                        RADIUS,
                        RADIUS,
                    );
                    winapi::um::winuser::SetWindowRgn(hwnd, rgn, 1);
                }
            }

            nwg::ImageFrame::builder()
                .parent(&data.window)
                .position((0, 0))
                .size((WIN_W, WIN_H))
                .bitmap(Some(&Bitmap::from_bin(bg_data())?))
                .build(&mut data.bg_image)?;

            nwg::ImageFrame::builder()
                .parent(&data.bg_image)
                .position((TRACK_X, TRACK_Y))
                .size((TRACK_W, TRACK_H))
                .background_color(Some(TRACK_COLOR))
                .build(&mut data.track)?;

            nwg::ImageFrame::builder()
                .parent(&data.track)
                .position((-BAR_W, 0))
                .size((BAR_W, TRACK_H))
                .background_color(Some(BAR_COLOR))
                .build(&mut data.bar)?;

            nwg::AnimationTimer::builder()
                .parent(&data.window)
                .interval(std::time::Duration::from_millis(TICK_MS))
                .build(&mut data.timer)?;

            // Wrap-up
            let ui = BasicAppUi {
                inner: Rc::new(data),
                default_handler: Default::default(),
            };

            // Events
            let evt_ui = Rc::downgrade(&ui.inner);
            let handle_events = move |evt, _evt_data, _handle| {
                if let Some(evt_ui) = evt_ui.upgrade().as_mut() {
                    match evt {
                        Event::OnWindowClose => {
                            evt_ui.exit();
                        }
                        Event::OnTimerTick => {
                            evt_ui.update_bar();
                        }
                        _ => {}
                    }
                }
            };

            ui.default_handler
                .borrow_mut()
                .push(nwg::full_bind_event_handler(
                    &ui.window.handle,
                    handle_events,
                ));

            return Ok(ui);
        }
    }

    impl Drop for BasicAppUi {
        /// To make sure that everything is freed without issues, the default handler must be unbound.
        fn drop(&mut self) {
            let mut handlers = self.default_handler.borrow_mut();
            for handler in handlers.drain(0..) {
                nwg::unbind_event_handler(&handler);
            }
        }
    }

    impl Deref for BasicAppUi {
        type Target = BasicApp;

        fn deref(&self) -> &BasicApp {
            &self.inner
        }
    }
}

fn ui() -> Result<(), nwg::NwgError> {
    nwg::init()?;
    let app = BasicApp::build_ui(Default::default())?;
    app.start_timer();
    nwg::dispatch_thread_events();
    Ok(())
}

pub fn setup() {
    std::thread::spawn(move || {
        if let Err(e) = ui() {
            eprintln!("{:?}", e);
        }
    });
}
