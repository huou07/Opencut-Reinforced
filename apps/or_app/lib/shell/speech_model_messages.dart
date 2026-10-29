import 'dart:io';

/// Turns a speech-model failure into a sentence the owner can act on.
///
/// The dialog used to answer every unexpected error with "The speech model
/// could not be changed.", which hid whether the source was refused, the
/// transfer was cut short or the network was unreachable.
String speechModelFailureMessage(Object error) {
  if (error is FormatException) {
    return 'The model source must stay on HTTPS.';
  }
  if (error is HttpException) {
    return 'The model download failed: ${error.message}';
  }
  if (error is SocketException) {
    return 'The model download could not reach the network.';
  }
  return 'The speech model could not be changed.';
}
