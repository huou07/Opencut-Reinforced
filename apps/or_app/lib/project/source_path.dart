import 'dart:io';

/// A path identity that survives symlinks.
///
/// The command path stores the file it actually read, which on macOS is the
/// resolved location: `/tmp` is a symlink to `/private/tmp`, and a media library
/// on a linked volume resolves somewhere else again. Comparing raw absolute
/// paths misses media that was imported a moment earlier, so every comparison
/// between a stored source and a freshly chosen path goes through here.
///
/// A missing file falls back to its absolute path: a stored source may point at
/// bytes the owner deleted, and that is still a distinct identity.
String canonicalSourcePath(String path) {
  try {
    return File(path).resolveSymbolicLinksSync();
  } on FileSystemException {
    return File(path).absolute.path;
  }
}

/// True when both names refer to the same file on this host.
bool sameSourcePath(String a, String b) =>
    canonicalSourcePath(a) == canonicalSourcePath(b);
