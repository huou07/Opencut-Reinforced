/// The four export profiles, named once for Settings and the export dialog.
///
/// The engine parses the same wire names; a label here never changes behaviour.
class PerformanceProfileOption {
  const PerformanceProfileOption(this.wire, this.title, this.description);

  /// The name the settings file and the export request carry.
  final String wire;
  final String title;
  final String description;
}

const performanceProfileOptions = [
  PerformanceProfileOption(
    'recommended',
    'Recommended',
    'Use the fastest encoder that opened on this machine.',
  ),
  PerformanceProfileOption(
    'maximum_performance',
    'Maximum performance',
    'Spend the encode hardware. New exports start on the Speed preset.',
  ),
  PerformanceProfileOption(
    'balanced',
    'Balanced',
    'Prefer the operating system encoder over vendor drivers.',
  ),
  PerformanceProfileOption(
    'compatibility',
    'Compatibility',
    'Software encoders only, for a driver that crashes or produces broken files.',
  ),
];
