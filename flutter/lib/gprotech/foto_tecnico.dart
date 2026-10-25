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
const _kCard = Color(0xFF002244);
const _kMuted = Color(0xFF8FA3C7);

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

  @override
  Widget build(BuildContext context) {
    final bytes = _bytes;
    return Container(
      margin: const EdgeInsets.only(left: 16, right: 16, top: 2, bottom: 8),
      child: Row(
        children: [
          ClipRRect(
            borderRadius: BorderRadius.circular(10),
            child: SizedBox(
              width: 56,
              height: 56,
              child: bytes != null
                  ? Image.memory(bytes, fit: BoxFit.cover, gaplessPlayback: true)
                  : Container(
                      color: _kCard,
                      child: const Icon(Icons.person_outline,
                          color: _kMuted, size: 30),
                    ),
            ),
          ),
          const SizedBox(width: 10),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                const Text('Sua foto',
                    style: TextStyle(fontSize: 12, color: _kMuted)),
                Wrap(
                  spacing: 4,
                  children: [
                    TextButton(
                      onPressed: _escolher,
                      child: Text(bytes == null ? 'Escolher' : 'Trocar'),
                    ),
                    if (bytes != null)
                      TextButton(
                        onPressed: _remover,
                        child: const Text('Remover'),
                      ),
                  ],
                ),
                if (_erro != null)
                  Text(_erro!,
                      style: const TextStyle(
                          fontSize: 11, color: Color(0xFFE57373))),
              ],
            ),
          ),
        ],
      ),
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
