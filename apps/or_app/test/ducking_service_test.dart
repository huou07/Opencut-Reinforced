import 'package:flutter_test/flutter_test.dart';
import 'package:or_app/production/ducking_service.dart';

void main() {
  test('merges overlapping and touching voice intervals', () {
    final merged = mergeDuckingIntervals(const [
      DuckingInterval(5000, 7000),
      DuckingInterval(0, 2000),
      DuckingInterval(1500, 3000),
      DuckingInterval(2000, 4000),
      DuckingInterval(9000, 9500),
    ]);
    expect(merged.map((interval) => (interval.start, interval.end)), [
      (0, 4000),
      (5000, 7000),
      (9000, 9500),
    ]);
  });

  test('keeps an empty list empty and sorts a single interval', () {
    expect(mergeDuckingIntervals(const []), isEmpty);
    final merged = mergeDuckingIntervals(const [DuckingInterval(10, 20)]);
    expect(merged.single.start, 10);
    expect(merged.single.end, 20);
  });
}
