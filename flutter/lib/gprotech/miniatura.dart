// GPROTECH: miniatura da tela da loja nos cartões de recentes/favoritos.
//
// Usa o pedido de captura de tela que o RustDesk já tem (o mesmo do menu
// "Capturar tela"), só que salvando direto em arquivo, sem abrir diálogo.
// A primeira captura sai alguns segundos depois de conectar e é refeita a
// cada poucos minutos enquanto a sessão estiver aberta.
import 'dart:async';
import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:image/image.dart' as img2;
import 'package:path_provider/path_provider.dart';
import 'package:uuid/uuid.dart';

import '../models/platform_model.dart';

const _kPrimeiraCaptura = Duration(seconds: 8);
const _kIntervalo = Duration(minutes: 5);
const _kLarguraMiniatura = 400;

Future<String>? _pastaFuture;

/// Pasta onde ficam as miniaturas (uma por ID).
Future<String> gprotechPastaMiniaturas() {
  return _pastaFuture ??= () async {
    final base = await getApplicationSupportDirectory();
    final dir = Directory(
        '${base.path}${Platform.pathSeparator}gprotech_miniaturas');
    if (!dir.existsSync()) {
      dir.createSync(recursive: true);
    }
    return dir.path;
  }();
}

String _nomeArquivo(String peerId) =>
    '${peerId.replaceAll(RegExp(r'[^A-Za-z0-9]'), '_')}.png';

/// Bytes da miniatura do computador, ou null se ainda não houver.
Future<Uint8List?> gprotechLerMiniatura(String peerId) async {
  try {
    final dir = await gprotechPastaMiniaturas();
    final f = File('$dir${Platform.pathSeparator}${_nomeArquivo(peerId)}');
    if (!await f.exists()) return null;
    return await f.readAsBytes();
  } catch (_) {
    return null;
  }
}

/// Controla as capturas automáticas de uma sessão remota.
class GprotechMiniatura {
  Timer? _timer;
  bool pendente = false;

  void iniciar(UuidValue sessionId, int Function() display) {
    parar();
    final supported = bind.sessionGetCommonSync(
        sessionId: sessionId, key: 'is_screenshot_supported', param: '');
    if (supported != 'true') return;
    void pedir() {
      pendente = true;
      final d = display();
      bind.sessionTakeScreenshot(sessionId: sessionId, display: d < 0 ? 0 : d);
    }

    _timer = Timer(_kPrimeiraCaptura, () {
      pedir();
      _timer = Timer.periodic(_kIntervalo, (_) => pedir());
    });
  }

  void parar() {
    _timer?.cancel();
    _timer = null;
    pendente = false;
  }

  /// Chamado quando a captura chega. Grava a miniatura sem mostrar diálogo.
  Future<void> salvar(UuidValue sessionId, String peerId, String msg) async {
    pendente = false;
    if (msg.isNotEmpty) {
      await bind.sessionHandleScreenshot(sessionId: sessionId, action: '2');
      return;
    }
    try {
      final dir = await gprotechPastaMiniaturas();
      final destino = '$dir${Platform.pathSeparator}${_nomeArquivo(peerId)}';
      final tmp = '$destino.tmp';
      final res = await bind.sessionHandleScreenshot(
          sessionId: sessionId, action: '0:$tmp');
      if (res.isNotEmpty) return;
      final bytes = await File(tmp).readAsBytes();
      final pequena = await compute(_reduzir, bytes);
      await File(destino).writeAsBytes(pequena, flush: true);
      await File(tmp).delete();
    } catch (e) {
      debugPrint('GPROTECH miniatura: $e');
    }
  }
}

List<int> _reduzir(Uint8List bytes) {
  final image = img2.decodePng(bytes);
  if (image == null || image.width <= _kLarguraMiniatura) return bytes;
  return img2.encodePng(img2.copyResize(image,
      width: _kLarguraMiniatura, interpolation: img2.Interpolation.average));
}
