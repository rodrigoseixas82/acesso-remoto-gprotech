// GPROTECH: caneta de anotação do Atendente.
//
// Com a caneta ligada, o mouse do técnico desenha em vez de controlar o
// computador da loja. Cada traço vai para o cliente em pedaços (mensagem
// `GprotechAnotacao`) e aparece lá numa camada transparente por cima de tudo.
// Aqui os traços são guardados nas coordenadas da tela remota, para continuarem
// no lugar certo quando o técnico mexe no zoom ou na rolagem.
//
// Prévia aprovada: design/previa-caneta.html (pasta do projeto no OneDrive).

import 'dart:async';
import 'dart:convert';
import 'dart:math';

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';

import '../models/input_model.dart';
import '../models/model.dart';
import '../models/platform_model.dart';

// Mesmos valores de `GprotechAnotacao` em message.proto.
const int _acaoTraco = 0;
const int _acaoDesfazer = 1;
const int _acaoLimpar = 2;
const int _acaoLigar = 3;
const int _acaoDesligar = 4;

const int ferramentaLivre = 0;
const int ferramentaSeta = 1;
const int ferramentaRetangulo = 2;
const int ferramentaCirculo = 3;

const List<int> coresCaneta = [
  0xFFE5484D, // vermelho
  0xFFFFC53D, // amarelo
  0xFF3567FD, // azul
  0xFF30A46C, // verde
];
const List<int> espessurasCaneta = [3, 5, 9];

const Duration _sumirApos = Duration(seconds: 5);
const Duration _sumirDuracao = Duration(milliseconds: 600);
const Duration _intervaloEnvio = Duration(milliseconds: 40);

class _Traco {
  final int id;
  final int ferramenta;
  final int cor;
  final int espessura;
  final bool sumir;
  final List<Offset> pontos; // coordenadas da tela remota
  DateTime? fim;

  _Traco(this.id, this.ferramenta, this.cor, this.espessura, this.sumir,
      this.pontos);

  double alfa(DateTime agora) {
    if (!sumir || fim == null) return 1.0;
    final passou = agora.difference(fim!);
    if (passou <= _sumirApos) return 1.0;
    final fracao = (passou - _sumirApos).inMilliseconds /
        _sumirDuracao.inMilliseconds;
    return max(0.0, 1.0 - fracao);
  }
}

/// Estado da caneta de uma sessão (uma aba do Atendente).
class GpAnotacao extends ChangeNotifier {
  static final Map<String, GpAnotacao> _porSessao = {};

  static GpAnotacao de(FFI ffi) => _porSessao.putIfAbsent(
      ffi.sessionId.toString(), () => GpAnotacao._(ffi));

  static void descartar(FFI ffi) {
    _porSessao.remove(ffi.sessionId.toString())?._encerrar();
  }

  final FFI ffi;
  GpAnotacao._(this.ffi);

  bool _ligada = false;
  int _ferramenta = ferramentaLivre;
  int _cor = coresCaneta.first;
  int _espessura = espessurasCaneta[1];
  bool _sumir = false;

  bool get ligada => _ligada;
  int get ferramenta => _ferramenta;
  int get cor => _cor;
  int get espessura => _espessura;
  bool get sumir => _sumir;

  final List<_Traco> _tracos = [];
  _Traco? _atual;
  int _proximoId = 1;
  int _enviados = 0; // pontos do traço livre atual já enviados
  Timer? _timerEnvio;
  Timer? _timerSumir;
  bool _encerrado = false;

  set ferramenta(int v) {
    _ferramenta = v;
    notifyListeners();
  }

  set cor(int v) {
    _cor = v;
    notifyListeners();
  }

  set espessura(int v) {
    _espessura = v;
    notifyListeners();
  }

  set sumir(bool v) {
    _sumir = v;
    notifyListeners();
  }

  void alternar() => _ligada ? desligar() : ligar();

  void ligar() {
    if (_ligada) return;
    _ligada = true;
    _enviar({'acao': _acaoLigar});
    notifyListeners();
  }

  void desligar() {
    if (!_ligada) return;
    _ligada = false;
    _cancelarAtual();
    _tracos.clear();
    _enviar({'acao': _acaoDesligar});
    notifyListeners();
  }

  void desfazer() {
    if (_tracos.isEmpty) return;
    if (identical(_tracos.last, _atual)) _cancelarAtual();
    _tracos.removeLast();
    _enviar({'acao': _acaoDesfazer});
    notifyListeners();
  }

  void limpar() {
    _cancelarAtual();
    _tracos.clear();
    _enviar({'acao': _acaoLimpar});
    notifyListeners();
  }

  // ---- Desenho ----

  void comecar(Offset remoto) {
    if (!_ligada) return;
    _cancelarAtual();
    final t = _Traco(_proximoId++, _ferramenta, _cor, _espessura, _sumir,
        _ferramenta == ferramentaLivre ? [remoto] : [remoto, remoto]);
    _tracos.add(t);
    _atual = t;
    _enviados = 0;
    _enviarAtual(fim: false);
    notifyListeners();
  }

  void mover(Offset remoto) {
    final t = _atual;
    if (t == null) return;
    if (t.ferramenta == ferramentaLivre) {
      if ((t.pontos.last - remoto).distance < 1.0) return;
      t.pontos.add(remoto);
    } else {
      t.pontos[1] = remoto;
    }
    _timerEnvio ??= Timer(_intervaloEnvio, () {
      _timerEnvio = null;
      if (_atual != null) _enviarAtual(fim: false);
    });
    notifyListeners();
  }

  void terminar() {
    final t = _atual;
    if (t == null) return;
    _timerEnvio?.cancel();
    _timerEnvio = null;
    t.fim = DateTime.now();
    _enviarAtual(fim: true);
    _atual = null;
    if (t.sumir) _iniciarTimerSumir();
    notifyListeners();
  }

  void _cancelarAtual() {
    _timerEnvio?.cancel();
    _timerEnvio = null;
    final t = _atual;
    if (t != null) {
      t.fim = DateTime.now();
      _enviarAtual(fim: true);
      _atual = null;
    }
  }

  void _enviarAtual({required bool fim}) {
    final t = _atual;
    if (t == null) return;
    List<Offset> pontos;
    if (t.ferramenta == ferramentaLivre) {
      pontos = t.pontos.sublist(_enviados);
      _enviados = t.pontos.length;
    } else {
      pontos = t.pontos;
    }
    if (pontos.isEmpty && !fim) return;
    _enviar({
      'acao': _acaoTraco,
      'id': t.id,
      'ferramenta': t.ferramenta,
      'argb': t.cor,
      'espessura': t.espessura,
      'sumir': t.sumir,
      'fim': fim,
      'pontos': [
        for (final p in pontos) ...[p.dx.round(), p.dy.round()]
      ],
    });
  }

  void _enviar(Map<String, dynamic> dados) {
    if (_encerrado) return;
    bind.sessionSendGprotechAnotacao(
        sessionId: ffi.sessionId, json: jsonEncode(dados));
  }

  void _iniciarTimerSumir() {
    _timerSumir ??=
        Timer.periodic(const Duration(milliseconds: 50), (timer) {
      final agora = DateTime.now();
      _tracos.removeWhere((t) => t.alfa(agora) <= 0.0);
      if (!_tracos.any((t) => t.sumir && t.fim != null)) {
        timer.cancel();
        _timerSumir = null;
      }
      notifyListeners();
    });
  }

  void _encerrar() {
    _timerEnvio?.cancel();
    _timerSumir?.cancel();
    _encerrado = true;
  }

  // ---- Coordenadas ----

  /// Posição do ponteiro (global na janela, como o InputModel usa) para a
  /// tela remota. Mesma conta de `InputModel._handlePointerDevicePos`.
  Offset? paraRemoto(Offset global, {bool limitar = false}) {
    final rect = ffi.ffiModel.rect;
    if (rect == null) return null;
    final c = CanvasCoords.fromCanvasModel(ffi.canvasModel);
    if (c.scale <= 0) return null;
    double x = global.dx - CanvasModel.leftToEdge;
    double y = global.dy - CanvasModel.topToEdge;
    final larguraImg = rect.width * c.scale;
    final alturaImg = rect.height * c.scale;
    if (c.scrollStyle != ScrollStyle.scrollauto) {
      x += larguraImg * c.scrollX;
      y += alturaImg * c.scrollY;
      if (c.size.width > larguraImg) x -= (c.size.width - larguraImg) / 2;
      if (c.size.height > alturaImg) y -= (c.size.height - alturaImg) / 2;
    } else {
      x -= c.x;
      y -= c.y;
    }
    x = x / c.scale + rect.left;
    y = y / c.scale + rect.top;
    final dentro = x >= rect.left &&
        y >= rect.top &&
        x <= rect.right &&
        y <= rect.bottom;
    if (!dentro) {
      if (!limitar) return null;
      x = x.clamp(rect.left, rect.right).toDouble();
      y = y.clamp(rect.top, rect.bottom).toDouble();
    }
    return Offset(x, y);
  }

  /// O contrário de [paraRemoto]: tela remota para posição global na janela.
  Offset? paraGlobal(Offset remoto) {
    final rect = ffi.ffiModel.rect;
    if (rect == null) return null;
    final c = CanvasCoords.fromCanvasModel(ffi.canvasModel);
    double x = (remoto.dx - rect.left) * c.scale;
    double y = (remoto.dy - rect.top) * c.scale;
    final larguraImg = rect.width * c.scale;
    final alturaImg = rect.height * c.scale;
    if (c.scrollStyle != ScrollStyle.scrollauto) {
      x -= larguraImg * c.scrollX;
      y -= alturaImg * c.scrollY;
      if (c.size.width > larguraImg) x += (c.size.width - larguraImg) / 2;
      if (c.size.height > alturaImg) y += (c.size.height - alturaImg) / 2;
    } else {
      x += c.x;
      y += c.y;
    }
    return Offset(x + CanvasModel.leftToEdge, y + CanvasModel.topToEdge);
  }
}

/// Camada por cima da tela remota: mostra os traços e, com a caneta ligada,
/// recebe o mouse no lugar do computador da loja.
class GpAnotacaoCamada extends StatefulWidget {
  final FFI ffi;
  const GpAnotacaoCamada({Key? key, required this.ffi}) : super(key: key);

  @override
  State<GpAnotacaoCamada> createState() => _GpAnotacaoCamadaState();
}

class _GpAnotacaoCamadaState extends State<GpAnotacaoCamada> {
  final _chave = GlobalKey();
  int? _ponteiro;

  GpAnotacao get _caneta => GpAnotacao.de(widget.ffi);

  bool _botaoValido(PointerDownEvent e) {
    if (e.kind == PointerDeviceKind.mouse) {
      return e.buttons & kPrimaryMouseButton != 0;
    }
    return true;
  }

  @override
  Widget build(BuildContext context) {
    final caneta = _caneta;
    return Positioned.fill(
      child: AnimatedBuilder(
        animation: caneta,
        builder: (context, _) {
          final pintura = IgnorePointer(
            child: CustomPaint(
              key: _chave,
              size: Size.infinite,
              painter: _PintorAnotacao(
                caneta: caneta,
                canvas: widget.ffi.canvasModel,
                origemGlobal: _origemGlobal,
              ),
            ),
          );
          if (!caneta.ligada) {
            return pintura;
          }
          return Stack(
            children: [
              Positioned.fill(
                child: MouseRegion(
                  cursor: SystemMouseCursors.precise,
                  child: Listener(
                    behavior: HitTestBehavior.opaque,
                    onPointerDown: (e) {
                      if (_ponteiro != null || !_botaoValido(e)) return;
                      final p = caneta.paraRemoto(e.position);
                      if (p == null) return;
                      _ponteiro = e.pointer;
                      caneta.comecar(p);
                    },
                    onPointerMove: (e) {
                      if (e.pointer != _ponteiro) return;
                      final p = caneta.paraRemoto(e.position, limitar: true);
                      if (p != null) caneta.mover(p);
                    },
                    onPointerUp: (e) {
                      if (e.pointer != _ponteiro) return;
                      _ponteiro = null;
                      caneta.terminar();
                    },
                    onPointerCancel: (e) {
                      if (e.pointer != _ponteiro) return;
                      _ponteiro = null;
                      caneta.terminar();
                    },
                    child: pintura,
                  ),
                ),
              ),
              Positioned(
                top: 56,
                left: 0,
                right: 0,
                child: Center(child: GpPaletaCaneta(caneta: caneta)),
              ),
            ],
          );
        },
      ),
    );
  }

  Offset _origemGlobal() {
    final box = _chave.currentContext?.findRenderObject() as RenderBox?;
    if (box == null || !box.hasSize) return Offset.zero;
    return box.localToGlobal(Offset.zero);
  }
}

class _PintorAnotacao extends CustomPainter {
  final GpAnotacao caneta;
  final Offset Function() origemGlobal;

  _PintorAnotacao({
    required this.caneta,
    required CanvasModel canvas,
    required this.origemGlobal,
  }) : super(repaint: Listenable.merge([caneta, canvas]));

  @override
  void paint(Canvas canvas, Size size) {
    if (caneta._tracos.isEmpty || caneta.ffi.ffiModel.rect == null) return;
    final origem = origemGlobal();
    final escala = caneta.ffi.canvasModel.scale;
    final agora = DateTime.now();
    canvas.save();
    canvas.clipRect(Offset.zero & size);
    for (final t in caneta._tracos) {
      final alfa = t.alfa(agora);
      if (alfa <= 0 || t.pontos.isEmpty) continue;
      final pts = <Offset>[];
      for (final r in t.pontos) {
        final g = caneta.paraGlobal(r);
        if (g != null) pts.add(g - origem);
      }
      if (pts.isEmpty) continue;
      _desenharTraco(canvas, t, pts, escala, alfa);
    }
    canvas.restore();
  }

  void _desenharTraco(
      Canvas canvas, _Traco t, List<Offset> p, double escala, double alfa) {
    final cor = Color(t.cor);
    final largura = max(1.0, t.espessura * escala);
    final tinta = Paint()
      ..color = cor.withOpacity(alfa * cor.opacity)
      ..style = PaintingStyle.stroke
      ..strokeWidth = largura
      ..strokeCap = StrokeCap.round
      ..strokeJoin = StrokeJoin.round
      ..isAntiAlias = true;
    final inicio = p.first;
    final fim = p.last;
    switch (t.ferramenta) {
      case ferramentaSeta:
        final d = fim - inicio;
        final ang = atan2(d.dy, d.dx);
        final ponta = largura * 3.2 + 6.0 * escala;
        final recuo = d.distance > ponta ? ponta * 0.8 : 0.0;
        canvas.drawLine(
            inicio, fim - Offset(cos(ang), sin(ang)) * recuo, tinta);
        final cabeca = Path()
          ..moveTo(fim.dx, fim.dy)
          ..lineTo(fim.dx - ponta * cos(ang - 0.45),
              fim.dy - ponta * sin(ang - 0.45))
          ..lineTo(fim.dx - ponta * cos(ang + 0.45),
              fim.dy - ponta * sin(ang + 0.45))
          ..close();
        canvas.drawPath(
            cabeca,
            Paint()
              ..color = tinta.color
              ..style = PaintingStyle.fill
              ..isAntiAlias = true);
        break;
      case ferramentaRetangulo:
        canvas.drawRect(Rect.fromPoints(inicio, fim), tinta);
        break;
      case ferramentaCirculo:
        canvas.drawOval(Rect.fromPoints(inicio, fim), tinta);
        break;
      default:
        if (p.length == 1) {
          canvas.drawCircle(
              inicio,
              largura / 2,
              Paint()
                ..color = tinta.color
                ..style = PaintingStyle.fill
                ..isAntiAlias = true);
        } else {
          final caminho = Path()..moveTo(inicio.dx, inicio.dy);
          for (final pt in p.skip(1)) {
            caminho.lineTo(pt.dx, pt.dy);
          }
          canvas.drawPath(caminho, tinta);
        }
    }
  }

  @override
  bool shouldRepaint(covariant _PintorAnotacao old) => true;
}

/// Paleta da caneta: ferramentas, cores, espessura, desfazer, limpar e "5 s".
class GpPaletaCaneta extends StatelessWidget {
  final GpAnotacao caneta;
  const GpPaletaCaneta({Key? key, required this.caneta}) : super(key: key);

  static const _azul = Color(0xFF1F4EE0);
  static const _azulClaro = Color(0xFFDFE8FF);
  static const _cinza = Color(0xFF33415A);

  Widget _botao({
    required String dica,
    required bool ativo,
    required VoidCallback onTap,
    required Widget filho,
  }) {
    return Tooltip(
      message: dica,
      waitDuration: const Duration(milliseconds: 400),
      child: InkWell(
        onTap: onTap,
        borderRadius: BorderRadius.circular(6),
        child: Container(
          width: 30,
          height: 28,
          alignment: Alignment.center,
          decoration: BoxDecoration(
            color: ativo ? _azulClaro : Colors.transparent,
            borderRadius: BorderRadius.circular(6),
          ),
          child: filho,
        ),
      ),
    );
  }

  Widget _icone(String dica, IconData icone, int ferramenta) => _botao(
        dica: dica,
        ativo: caneta.ferramenta == ferramenta,
        onTap: () => caneta.ferramenta = ferramenta,
        filho: Icon(icone,
            size: 18,
            color: caneta.ferramenta == ferramenta ? _azul : _cinza),
      );

  Widget _separador() => Container(
        width: 1,
        height: 18,
        margin: const EdgeInsets.symmetric(horizontal: 4),
        color: const Color(0xFFD5DCE7),
      );

  @override
  Widget build(BuildContext context) {
    const nomesCores = ['Vermelho', 'Amarelo', 'Azul', 'Verde'];
    const nomesEspessuras = ['Fina', 'Média', 'Grossa'];
    return Material(
      color: Colors.white,
      elevation: 6,
      borderRadius: BorderRadius.circular(10),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 4),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            _icone('Caneta livre', Icons.gesture, ferramentaLivre),
            _icone('Seta', Icons.north_east, ferramentaSeta),
            _icone('Retângulo', Icons.crop_square, ferramentaRetangulo),
            _icone('Círculo', Icons.circle_outlined, ferramentaCirculo),
            _separador(),
            for (var i = 0; i < coresCaneta.length; i++)
              _botao(
                dica: nomesCores[i],
                ativo: false,
                onTap: () => caneta.cor = coresCaneta[i],
                filho: Container(
                  width: 18,
                  height: 18,
                  decoration: BoxDecoration(
                    color: Color(coresCaneta[i]),
                    shape: BoxShape.circle,
                    border: Border.all(color: Colors.white, width: 2),
                    boxShadow: [
                      BoxShadow(
                        color: caneta.cor == coresCaneta[i]
                            ? _azul
                            : const Color(0xFFC5CFDD),
                        spreadRadius: caneta.cor == coresCaneta[i] ? 2 : 1,
                      ),
                    ],
                  ),
                ),
              ),
            _separador(),
            for (var i = 0; i < espessurasCaneta.length; i++)
              _botao(
                dica: nomesEspessuras[i],
                ativo: caneta.espessura == espessurasCaneta[i],
                onTap: () => caneta.espessura = espessurasCaneta[i],
                filho: Container(
                  width: 4.0 + i * 3.5,
                  height: 4.0 + i * 3.5,
                  decoration: BoxDecoration(
                    color: caneta.espessura == espessurasCaneta[i]
                        ? _azul
                        : _cinza,
                    shape: BoxShape.circle,
                  ),
                ),
              ),
            _separador(),
            _botao(
              dica: 'Desfazer',
              ativo: false,
              onTap: caneta.desfazer,
              filho: const Icon(Icons.undo, size: 18, color: _cinza),
            ),
            _botao(
              dica: 'Limpar tudo',
              ativo: false,
              onTap: caneta.limpar,
              filho: const Icon(Icons.delete_outline,
                  size: 18, color: Color(0xFFC9363B)),
            ),
            _separador(),
            Tooltip(
              message: 'Os traços desaparecem 5 s depois de desenhados',
              waitDuration: const Duration(milliseconds: 400),
              child: InkWell(
                onTap: () => caneta.sumir = !caneta.sumir,
                borderRadius: BorderRadius.circular(6),
                child: Padding(
                  padding: const EdgeInsets.symmetric(horizontal: 4),
                  child: Row(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Icon(
                        caneta.sumir
                            ? Icons.check_box
                            : Icons.check_box_outline_blank,
                        size: 18,
                        color: caneta.sumir ? _azul : _cinza,
                      ),
                      const SizedBox(width: 4),
                      const Text('Sumir em 5 s',
                          style: TextStyle(fontSize: 12, color: _cinza)),
                    ],
                  ),
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}
