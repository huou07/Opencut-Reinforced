# Developer Preview — Install, Try and Remove

Last reviewed: 2026-10-26

Scope: how a tester installs, uses and removes a Developer Preview build, and
what it does not promise. Pipeline, signing and asset rules are in
[release.md](release.md).

Developer Preview packages are unsigned or ad-hoc-signed **debug builds for
testing**, not production releases. The Linux `.deb` is tested on Debian 13 amd64
and Ubuntu 24.04 amd64; other distributions should use the portable TAR.GZ.
Never disable operating-system security protections to run a package. If your
system blocks it, stop and report the platform and message through the
[bug report form](https://github.com/huou07/Opencut-Reinforced/issues/new?template=bug_report.yml).

Download the artifact for your platform from the linked **Developer Preview**
Actions run; the run page shows its expiry and source commit.

## macOS

1. Download `developer-preview-macos` and expand the ZIP.
2. Move `OpenCut Reinforced.app` to Applications or another folder you control.
3. Open the app. It is ad-hoc-signed and not notarised; if macOS blocks it, do
   not turn off Gatekeeper or change security settings to continue.

## Windows

1. Download `developer-preview-windows` and extract the complete ZIP folder.
2. Open the `OpenCut Reinforced` folder and run `or_app.exe`.
3. The build is unsigned; if Windows cannot verify the publisher, do not disable
   SmartScreen to continue.

## Linux

The artifact includes an amd64 `.deb` tested on Debian 13 and Ubuntu 24.04, plus
a portable TAR.GZ bundle.

- Debian 13 / Ubuntu 24.04: open the `.deb` in your Software app, or
  `sudo apt install ./opencut-reinforced-linux-x64-debug.deb`. The package adds a
  launcher, installs OR's MIT licence as package copyright, and keeps editor files
  under `/opt/opencut-reinforced`.
- Other distributions: extract the TAR.GZ, keep the files together, and run
  `./opencut-reinforced/or_app` from the extracted folder. Desktop GTK runtime
  libraries come from your distribution.

## Android

1. Download `developer-preview-android` and open the APK on the device.
2. Android may require permission for the app used to open the APK to install
   apps. The preview uses the standard debug signing key, so Android may reject an
   update over a build signed with a different key.
3. Do not change device security settings if your organisation or Android blocks
   the installation.

## Try the editor

Use a copy of media you are comfortable testing with. Create a project, import
ordinary video and audio, add media to the timeline, revise a clip, preview, add
a title or manual caption, save, close, reopen, and export. Play the export in a
separate media player.

Report the operating system or device, source media format and duration, steps,
what you expected, and what happened. Do not attach private media, credentials or
unredacted logs.

## Update and uninstall

Keep project files in a separate folder from the application package.

- Debian/Ubuntu: install the newer `.deb` the same way to update; remove it with
  your Software app or `sudo apt remove opencut-reinforced`.
- Other desktop archives: replace the old app or extracted folder.
- Android: updates in place only with a compatible signing key; uninstall through
  Android settings.

Project files and exports are stored separately and are not removed by
uninstalling the app.
