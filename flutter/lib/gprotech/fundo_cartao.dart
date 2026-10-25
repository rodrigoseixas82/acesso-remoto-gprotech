// GPROTECH: parte de cima do cartão de recentes/favoritos. Mostra a miniatura
// da tela do computador quando existe; senão, o fundo e o ícone do RustDesk.
import 'dart:async';
import 'dart:typed_data';

import 'package:flutter/material.dart';

import 'miniatura.dart';

// A captura é gravada pela janela da sessão (outro processo Flutter), então o
// cartão relê o arquivo de tempos em tempos para pegar a versão nova.
const _kReler = Duration(minutes: 1);

class GprotechFundoCartao extends StatefulWidget {
  final String peerId;
  final String nome;
  final Color color;
  final Widget child;

  const GprotechFundoCartao({
    Key? key,
    required this.peerId,
    required this.nome,
    required this.color,
    required this.child,
  }) : super(key: key);

  @override
  State<GprotechFundoCartao> createState() => _GprotechFundoCartaoState();
}

class _GprotechFundoCartaoState extends State<GprotechFundoCartao> {
  Uint8List? _bytes;
  Timer? _timer;

  @override
  void initState() {
    super.initState();
    _carregar();
    _timer = Timer.periodic(_kReler, (_) => _carregar());
  }

  @override
  void didUpdateWidget(GprotechFundoCartao oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.peerId != widget.peerId) _carregar();
  }

  @override
  void dispose() {
    _timer?.cancel();
    super.dispose();
  }

  Future<void> _carregar() async {
    final b = await gprotechLerMiniatura(widget.peerId);
    if (!mounted) return;
    if (b?.length != _bytes?.length || (b == null) != (_bytes == null)) {
      setState(() => _bytes = b);
    }
  }

  @override
  Widget build(BuildContext context) {
    final bytes = _bytes;
    if (bytes == null) {
      return Container(color: widget.color, child: widget.child);
    }
    return Container(
      color: widget.color,
      child: Column(
        children: [
          Expanded(
            child: Image.memory(
              bytes,
              fit: BoxFit.cover,
              width: double.infinity,
              gaplessPlayback: true,
            ),
          ),
          Padding(
            padding: const EdgeInsets.symmetric(vertical: 4, horizontal: 4),
            child: Text(
              widget.nome,
              style: const TextStyle(color: Colors.white70, fontSize: 12),
              textAlign: TextAlign.center,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
            ),
          ),
        ],
      ),
    );
  }
}
