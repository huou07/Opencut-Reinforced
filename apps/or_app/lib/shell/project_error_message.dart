import '../project/project_gateway.dart';

/// What the top bar says when a project command fails.
///
/// Codes that the user can act on get a sentence with the next step; anything
/// else falls back to the gateway's own message so a specific failure never
/// hides behind a generic one.
String projectErrorMessage(ProjectGatewayException error) => switch (error.code) {
  'DESTINATION_EXISTS' =>
    'A file already exists at that location. Choose another name or use Open Project.',
  'PROJECT_FILE_CHANGED' =>
    'The project file changed outside OR. The save was blocked.',
  'RECOVERY_REQUIRED' =>
    'This project needs explicit recovery handling before it can be opened.',
  'LOCAL_IPC_ERROR' =>
    'The local CLI session could not be started. The project remains on disk. ${error.message}',
  _ => error.message.isEmpty
      ? 'The project operation failed.'
      : error.message,
};
