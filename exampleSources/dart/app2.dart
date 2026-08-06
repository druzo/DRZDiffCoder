// Dart — Future + async/await + simple retry.

Future<int> fetchRetry(Future<int> Function() op, int attempts) async {
  for (var i = 0; i < attempts; i++) {
    try {
      return await op();
    } catch (_) {
      if (i == attempts - 1) rethrow;
    }
  }
  throw StateError('unreachable');
}

Future<int> loadCount() async {
  await Future.delayed(Duration(milliseconds: 100));
  return 42;
}

void main() async {
  final n = await fetchRetry(loadCount, 3);
  print('count = $n');
}