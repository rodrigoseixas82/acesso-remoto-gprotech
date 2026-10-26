// GPROTECH: link "Procurar atualizações" do Cliente instalado, abaixo do botão
// do WhatsApp. Antecipa a busca automática (ao abrir e a cada 6 horas). Achando
// versão nova, o cartão "Atualizar" toma o lugar do link.
import 'dart:async';

import 'package:flutter/material.dart';

import '../models/platform_model.dart';
import '../models/state_model.dart';

const _kGpMuted = Color(0xFF8FA3C7);
const _kGpVerde = Color(0xFF5FD3A6);
const _kGpCyan = Color(0xFF37FFFF);

enum _Estado { parado, procurando, atual, erro }

class GprotechProcurarAtualizacoes extends StatefulWidget {
  const GprotechProcurarAtualizacoes({Key? key}) : super(key: key);

  @override
  State<GprotechProcurarAtualizacoes> createState() =>
      _GprotechProcurarAtualizacoesState();
}

class _GprotechProcurarAtualizacoesState
    extends State<GprotechProcurarAtualizacoes> {
  var _estado = _Estado.parado;
  var _hover = false;
  Timer? _voltar;

  @override
  void dispose() {
    _voltar?.cancel();
    super.dispose();
  }

  Future<void> _procurar() async {
    if (_estado == _Estado.procurando) return;
    _voltar?.cancel();
    setState(() => _estado = _Estado.procurando);
    String r;
    try {
      r = await bind.mainGetCommon(key: 'gprotech-procurar-atualizacao');
    } catch (_) {
      r = 'erro';
    }
    if (!mounted) return;
    if (r.startsWith('nova:')) {
      // O cartão "Atualizar" aparece no lugar deste link.
      setState(() => _estado = _Estado.parado);
      stateGlobal.updateUrl.value = r.substring('nova:'.length);
      return;
    }
    setState(() => _estado = r == 'atual' ? _Estado.atual : _Estado.erro);
    // A resposta fica alguns segundos e o link volta.
    _voltar = Timer(const Duration(seconds: 6), () {
      if (mounted) setState(() => _estado = _Estado.parado);
    });
  }

  @override
  Widget build(BuildContext context) {
    const estilo = TextStyle(fontSize: 12, color: _kGpMuted);
    final Widget filho;
    switch (_estado) {
      case _Estado.procurando:
        filho = Row(
          mainAxisSize: MainAxisSize.min,
          children: const [
            SizedBox(
              width: 10,
              height: 10,
              child: CircularProgressIndicator(strokeWidth: 2, color: _kGpCyan),
            ),
            SizedBox(width: 6),
            Text('Procurando atualizações…', style: estilo),
          ],
        );
        break;
      case _Estado.atual:
        filho = const Text('✓ Você já está com a versão mais recente.',
            style: TextStyle(fontSize: 12, color: _kGpVerde));
        break;
      case _Estado.erro:
        filho = const Text(
            'Não foi possível verificar agora. Tente mais tarde.',
            style: estilo);
        break;
      case _Estado.parado:
        final cor = _hover ? Colors.white : _kGpMuted;
        filho = MouseRegion(
          cursor: SystemMouseCursors.click,
          onEnter: (_) => setState(() => _hover = true),
          onExit: (_) => setState(() => _hover = false),
          child: GestureDetector(
            onTap: _procurar,
            child: Text('Procurar atualizações',
                style: TextStyle(
                    fontSize: 12,
                    color: cor,
                    decoration: TextDecoration.underline,
                    decorationColor: cor)),
          ),
        );
        break;
    }
    return Padding(
      padding: const EdgeInsets.only(top: 2, bottom: 12),
      child: SizedBox(height: 18, child: Center(child: filho)),
    );
  }
}
