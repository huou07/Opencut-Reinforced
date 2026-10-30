import 'package:flutter_test/flutter_test.dart';

import 'package:or_app/project/project_gateway.dart';
import 'package:or_app/shell/project_error_message.dart';

ProjectGatewayException _error(String code, [String message = '']) =>
    ProjectGatewayException(code, message);

void main() {
  test('known codes name the next step', () {
    expect(
      projectErrorMessage(_error('DESTINATION_EXISTS')),
      contains('Choose another name'),
    );
    expect(
      projectErrorMessage(_error('PROJECT_FILE_CHANGED')),
      contains('save was blocked'),
    );
    expect(
      projectErrorMessage(_error('RECOVERY_REQUIRED')),
      contains('explicit recovery'),
    );
    expect(
      projectErrorMessage(_error('LOCAL_IPC_ERROR', 'bridge exited')),
      contains('bridge exited'),
    );
  });

  test('an unknown code keeps the gateway message', () {
    expect(
      projectErrorMessage(_error('SOMETHING_ELSE', 'The media probe failed.')),
      'The media probe failed.',
    );
  });

  test('an empty message still says something', () {
    expect(projectErrorMessage(_error('SOMETHING_ELSE')), isNotEmpty);
  });
}
