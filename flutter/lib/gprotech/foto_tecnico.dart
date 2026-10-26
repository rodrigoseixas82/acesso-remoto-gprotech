// GPROTECH: campo "Sua foto" do Atendente. A foto fica salva no PC do técnico
// (opção local, como data URI JPEG 160x160), vai junto na conexão e aparece
// para o cliente no quadro do atendimento.
import 'dart:convert';
import 'dart:io';
import 'dart:math';
import 'dart:typed_data';

import 'package:file_picker/file_picker.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:image/image.dart' as img2;

import '../models/platform_model.dart';

/// Igual a OPTION_FOTO_TECNICO em src/gprotech.rs
const kGprotechOpcaoFoto = 'gprotech-foto-tecnico';
const _kLado = 160;
const _kFundo = Color(0xFF001324);
const _kBorda = Color(0xFF0A2A4A);
const _kMuted = Color(0xFF8FA3C7);
const _kTexto = Color(0xFFCAD7FF);
const _kVermelho = Color(0xFFE57373);

class GprotechFotoTecnico extends StatefulWidget {
  const GprotechFotoTecnico({Key? key}) : super(key: key);

  @override
  State<GprotechFotoTecnico> createState() => _GprotechFotoTecnicoState();
}

class _GprotechFotoTecnicoState extends State<GprotechFotoTecnico> {
  Uint8List? _bytes;
  String? _erro;

  @override
  void initState() {
    super.initState();
    _bytes = _decodificar(bind.mainGetLocalOption(key: kGprotechOpcaoFoto));
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
    if (jpg == null) {
      setState(() => _erro = 'Não foi possível ler essa imagem');
      return;
    }
    await bind.mainSetLocalOption(
        key: kGprotechOpcaoFoto,
        value: 'data:image/jpeg;base64,${base64Encode(jpg)}');
    setState(() {
      _bytes = jpg;
      _erro = null;
    });
  }

  Future<void> _remover() async {
    await bind.mainSetLocalOption(key: kGprotechOpcaoFoto, value: '');
    setState(() {
      _bytes = null;
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

  // Foto (clicável para trocar) com lápis e lixeira ao lado; sem foto, um
  // quadro com câmera e só o botão de escolher.
  @override
  Widget build(BuildContext context) {
    final bytes = _bytes;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      mainAxisSize: MainAxisSize.min,
      children: [
        Row(
          children: [
            Tooltip(
              message: bytes == null ? 'Escolher foto' : 'Trocar foto',
              child: InkWell(
                onTap: _escolher,
                borderRadius: BorderRadius.circular(10),
                child: Container(
                  width: 48,
                  height: 48,
                  clipBehavior: Clip.antiAlias,
                  decoration: BoxDecoration(
                    color: _kFundo,
                    borderRadius: BorderRadius.circular(10),
                    border:
                        bytes == null ? Border.all(color: _kMuted) : null,
                  ),
                  child: bytes != null
                      ? Image.memory(bytes,
                          fit: BoxFit.cover, gaplessPlayback: true)
                      : const Icon(Icons.add_a_photo_outlined,
                          color: _kMuted, size: 22),
                ),
              ),
            ),
            const SizedBox(width: 10),
            if (bytes == null)
              _botao(Icons.upload_outlined, 'Escolher foto', _kTexto,
                  _escolher)
            else ...[
              _botao(Icons.edit_outlined, 'Trocar foto', _kTexto, _escolher),
              const SizedBox(width: 6),
              _botao(
                  Icons.delete_outline, 'Remover foto', _kVermelho, _remover),
            ],
          ],
        ),
        if (_erro != null)
          Padding(
            padding: const EdgeInsets.only(top: 4),
            child: Text(_erro!,
                style: const TextStyle(fontSize: 11, color: _kVermelho)),
          ),
      ],
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
