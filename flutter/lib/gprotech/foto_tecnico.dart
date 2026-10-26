// GPROTECH: cartão "Seu perfil" do Atendente (rodapé da barra lateral). Nome e
// foto ficam travados para não mudar sem querer; "Editar" abre os dois juntos e
// "Salvar" grava os dois. Ficam salvos no PC do técnico (opções locais; a foto
// como data URI JPEG 160x160), vão junto na conexão e aparecem para o cliente no
// quadro do atendimento.
import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:math';
import 'dart:typed_data';

import 'package:file_picker/file_picker.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:image/image.dart' as img2;

import '../models/platform_model.dart';

/// Igual a OPTION_NOME_TECNICO em src/gprotech.rs
const kGprotechOpcaoNome = 'gprotech-nome-tecnico';

/// Igual a OPTION_FOTO_TECNICO em src/gprotech.rs
const kGprotechOpcaoFoto = 'gprotech-foto-tecnico';
const _kLado = 160;
const _kCartao = Color(0xFF002244);
const _kBordaCartao = Color(0xFF1D3F66);
const _kAzul = Color(0xFF3567FD);
const _kFundo = Color(0xFF001324);
const _kBorda = Color(0xFF0A2A4A);
const _kMuted = Color(0xFF8FA3C7);
const _kTexto = Color(0xFFCAD7FF);
const _kVermelho = Color(0xFFE57373);
const _kVerde = Color(0xFF5FD3A6);

class GprotechPerfilTecnico extends StatefulWidget {
  const GprotechPerfilTecnico({Key? key}) : super(key: key);

  @override
  State<GprotechPerfilTecnico> createState() => _GprotechPerfilTecnicoState();
}

class _GprotechPerfilTecnicoState extends State<GprotechPerfilTecnico> {
  // Salvos (os que vão na conexão).
  String _nome = '';
  String _fotoUri = '';
  Uint8List? _foto;

  // Em edição (só gravados no "Salvar").
  var _editando = false;
  final _nomeController = TextEditingController();
  final _nomeFoco = FocusNode();
  String _fotoUriEdicao = '';
  Uint8List? _fotoEdicao;
  String? _erro;

  var _salvo = false;
  Timer? _timerSalvo;

  @override
  void initState() {
    super.initState();
    _nome = bind.mainGetLocalOption(key: kGprotechOpcaoNome).trim();
    _fotoUri = bind.mainGetLocalOption(key: kGprotechOpcaoFoto);
    _foto = _decodificar(_fotoUri);
    if (_foto == null) _fotoUri = '';
  }

  @override
  void dispose() {
    _timerSalvo?.cancel();
    _nomeController.dispose();
    _nomeFoco.dispose();
    super.dispose();
  }

  static Uint8List? _decodificar(String dataUri) {
    final virgula = dataUri.indexOf(',');
    if (!dataUri.startsWith('data:image/') || virgula < 0) return null;
    try {
      return base64Decode(dataUri.substring(virgula + 1));
    } catch (_) {
      return null;
    }
  }

  void _editar() {
    _timerSalvo?.cancel();
    setState(() {
      _editando = true;
      _salvo = false;
      _erro = null;
      _nomeController.text = _nome;
      _nomeController.selection =
          TextSelection.collapsed(offset: _nomeController.text.length);
      _fotoUriEdicao = _fotoUri;
      _fotoEdicao = _foto;
    });
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted) _nomeFoco.requestFocus();
    });
  }

  void _cancelar() {
    setState(() {
      _editando = false;
      _erro = null;
    });
  }

  Future<void> _salvar() async {
    final nome = _nomeController.text.trim();
    await bind.mainSetLocalOption(key: kGprotechOpcaoNome, value: nome);
    await bind.mainSetLocalOption(key: kGprotechOpcaoFoto, value: _fotoUriEdicao);
    if (!mounted) return;
    setState(() {
      _nome = nome;
      _fotoUri = _fotoUriEdicao;
      _foto = _fotoEdicao;
      _editando = false;
      _erro = null;
      _salvo = true;
    });
    _timerSalvo?.cancel();
    _timerSalvo = Timer(const Duration(seconds: 3), () {
      if (mounted) setState(() => _salvo = false);
    });
  }

  Future<void> _escolher() async {
    final r = await FilePicker.platform
        .pickFiles(type: FileType.image, dialogTitle: 'Escolha sua foto');
    final path = r?.files.single.path;
    if (path == null) return;
    Uint8List? jpg;
    try {
      jpg = await compute(_prepararFoto, await File(path).readAsBytes());
    } catch (_) {
      jpg = null;
    }
    if (!mounted) return;
    setState(() {
      if (jpg == null) {
        _erro = 'Não foi possível ler essa imagem';
      } else {
        _fotoEdicao = jpg;
        _fotoUriEdicao = 'data:image/jpeg;base64,${base64Encode(jpg)}';
        _erro = null;
      }
    });
  }

  void _remover() {
    setState(() {
      _fotoEdicao = null;
      _fotoUriEdicao = '';
      _erro = null;
    });
  }

  Widget _botao(IconData icon, String dica, Color cor, VoidCallback onTap) {
    return Tooltip(
      message: dica,
      child: InkWell(
        onTap: onTap,
        borderRadius: BorderRadius.circular(7),
        child: Container(
          width: 30,
          height: 30,
          alignment: Alignment.center,
          decoration: BoxDecoration(
            color: _kFundo,
            borderRadius: BorderRadius.circular(7),
            border: Border.all(color: _kBorda),
          ),
          child: Icon(icon, size: 16, color: cor),
        ),
      ),
    );
  }

  Widget _quadroFoto(Uint8List? bytes, {VoidCallback? onTap}) {
    final quadro = Container(
      width: 48,
      height: 48,
      clipBehavior: Clip.antiAlias,
      decoration: BoxDecoration(
        color: _kFundo,
        borderRadius: BorderRadius.circular(10),
        border: bytes == null ? Border.all(color: _kBorda) : null,
      ),
      child: bytes != null
          ? Image.memory(bytes, fit: BoxFit.cover, gaplessPlayback: true)
          : const Icon(Icons.photo_camera_outlined, color: _kMuted, size: 22),
    );
    if (onTap == null) return quadro;
    return Tooltip(
      message: bytes == null ? 'Escolher foto' : 'Trocar foto',
      child: InkWell(
        onTap: onTap,
        borderRadius: BorderRadius.circular(10),
        child: quadro,
      ),
    );
  }

  Widget _botaoEditar() {
    return InkWell(
      onTap: _editar,
      borderRadius: BorderRadius.circular(6),
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
        decoration: BoxDecoration(
          color: _kFundo,
          borderRadius: BorderRadius.circular(6),
          border: Border.all(color: _kBorda),
        ),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: const [
            Icon(Icons.edit_outlined, size: 12, color: _kTexto),
            SizedBox(width: 4),
            Text('Editar', style: TextStyle(fontSize: 11.5, color: _kTexto)),
          ],
        ),
      ),
    );
  }

  // Travado: foto e nome lado a lado, só o botão "Editar".
  Widget _travado() {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      mainAxisSize: MainAxisSize.min,
      children: [
        Row(
          children: [
            const Text('Seu perfil',
                style: TextStyle(fontSize: 11, color: _kMuted)),
            const Spacer(),
            _botaoEditar(),
          ],
        ),
        const SizedBox(height: 6),
        Row(
          children: [
            _quadroFoto(_foto),
            const SizedBox(width: 10),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                mainAxisSize: MainAxisSize.min,
                children: [
                  _nome.isEmpty
                      ? const Text('Sem nome cadastrado',
                          style: TextStyle(
                              fontSize: 12.5,
                              color: _kMuted,
                              fontStyle: FontStyle.italic))
                      : Text(_nome,
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                          style: const TextStyle(
                              fontSize: 14, fontWeight: FontWeight.w600)),
                  const SizedBox(height: 2),
                  Text(
                      _nome.isEmpty
                          ? 'O cliente vê o usuário do Windows'
                          : 'Aparece para o cliente',
                      style: const TextStyle(fontSize: 11, color: _kMuted)),
                ],
              ),
            ),
          ],
        ),
        if (_salvo)
          const Padding(
            padding: EdgeInsets.only(top: 8),
            child: Text('✓ Perfil salvo',
                style: TextStyle(fontSize: 11, color: _kVerde)),
          ),
      ],
    );
  }

  // Editando: campo do nome, foto com trocar/remover, Cancelar e Salvar.
  Widget _edicao() {
    final bytes = _fotoEdicao;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      mainAxisSize: MainAxisSize.min,
      children: [
        const Text('Seu nome', style: TextStyle(fontSize: 11, color: _kMuted)),
        const SizedBox(height: 4),
        Focus(
          // Esc cancela (Enter salva, pelo onSubmitted).
          onKeyEvent: (_, e) {
            if (e is KeyDownEvent &&
                e.logicalKey == LogicalKeyboardKey.escape) {
              _cancelar();
              return KeyEventResult.handled;
            }
            return KeyEventResult.ignored;
          },
          child: TextField(
            controller: _nomeController,
            focusNode: _nomeFoco,
            maxLength: 40,
            style: const TextStyle(fontSize: 13),
            decoration: const InputDecoration(
              counterText: '',
              isDense: true,
              contentPadding: EdgeInsets.symmetric(horizontal: 8, vertical: 9),
            ),
            onSubmitted: (_) => _salvar(),
          ),
        ),
        const SizedBox(height: 10),
        const Text('Sua foto', style: TextStyle(fontSize: 11, color: _kMuted)),
        const SizedBox(height: 6),
        Row(
          children: [
            _quadroFoto(bytes, onTap: _escolher),
            const SizedBox(width: 10),
            if (bytes == null)
              _botao(Icons.upload_outlined, 'Escolher foto', _kTexto, _escolher)
            else ...[
              _botao(Icons.edit_outlined, 'Trocar foto', _kTexto, _escolher),
              const SizedBox(width: 6),
              _botao(Icons.delete_outline, 'Remover foto', _kVermelho, _remover),
            ],
          ],
        ),
        if (_erro != null)
          Padding(
            padding: const EdgeInsets.only(top: 4),
            child: Text(_erro!,
                style: const TextStyle(fontSize: 11, color: _kVermelho)),
          ),
        const SizedBox(height: 10),
        Row(
          children: [
            Expanded(
              child: SizedBox(
                height: 30,
                child: OutlinedButton(
                  onPressed: _cancelar,
                  style: OutlinedButton.styleFrom(
                    padding: EdgeInsets.zero,
                    side: const BorderSide(color: Color(0xFF3B5578)),
                  ),
                  child: const Text('Cancelar',
                      style: TextStyle(fontSize: 13, color: Color(0xFFCFD8E6))),
                ),
              ),
            ),
            const SizedBox(width: 8),
            Expanded(
              child: SizedBox(
                height: 30,
                child: ElevatedButton(
                  onPressed: _salvar,
                  style: ElevatedButton.styleFrom(
                    padding: EdgeInsets.zero,
                    backgroundColor: _kAzul,
                  ),
                  child: const Text('Salvar',
                      style: TextStyle(
                          fontSize: 13,
                          color: Colors.white,
                          fontWeight: FontWeight.w600)),
                ),
              ),
            ),
          ],
        ),
      ],
    );
  }

  @override
  Widget build(BuildContext context) {
    return Container(
      margin: const EdgeInsets.fromLTRB(12, 6, 12, 12),
      padding: const EdgeInsets.fromLTRB(12, 10, 12, 10),
      decoration: BoxDecoration(
        color: _kCartao,
        borderRadius: BorderRadius.circular(10),
        border: Border.all(color: _editando ? _kAzul : _kBordaCartao),
      ),
      child: _editando ? _edicao() : _travado(),
    );
  }
}

/// Recorta o centro em quadrado, reduz para 160x160 e grava em JPEG (~10 KB).
Uint8List? _prepararFoto(Uint8List data) {
  final image = img2.decodeImage(data);
  if (image == null) return null;
  final lado = min(image.width, image.height);
  final quadrado = img2.copyCrop(image,
      x: (image.width - lado) ~/ 2,
      y: (image.height - lado) ~/ 2,
      width: lado,
      height: lado);
  final pequena = img2.copyResize(quadrado,
      width: _kLado, height: _kLado, interpolation: img2.Interpolation.average);
  return img2.encodeJpg(pequena, quality: 85);
}
