# Developer Preview: install, try, and remove

Developer Preview packages are unsigned or ad-hoc-signed debug builds for
testing. They are not production installers. Do not disable operating-system
security protections to run them. If your system blocks a package, stop and
report the platform and message through the [bug report form](https://github.com/huou07/Opencut-Reinforced/issues/new?template=bug_report.yml).

Download the artifact for your platform from the linked **Developer Preview**
Actions run. Artifacts are temporary; the run page shows their expiry and the
source commit.

## macOS

1. Download `developer-preview-macos` and expand the ZIP.
2. Move `OpenCut Reinforced.app` to Applications or another folder you control.
3. Open the app. The preview is ad-hoc-signed and not notarized; if macOS blocks
   it, do not turn off Gatekeeper or change security settings to continue.

## Windows

1. Download `developer-preview-windows` and extract the complete ZIP folder.
2. Open the `OpenCut Reinforced` folder and run `or_app.exe`.
3. This is an unsigned debug build. If Windows warns that it cannot verify the
   publisher, do not disable SmartScreen to continue.

## Linux

1. Download `developer-preview-linux` and extract the complete TAR.GZ folder.
2. Keep the extracted files together and run `./opencut-reinforced/or_app` from
   the folder where you extracted the archive.
3. The package is an unsigned debug build and may require the desktop GTK
   runtime libraries provided by your distribution. Do not install libraries
   from an untrusted source to make it run.

## Android

1. Download `developer-preview-android` and open the APK on the device.
2. Android may require permission for the app used to open the APK to install
   apps. The preview uses the standard debug signing key; Android may reject an
   update if you installed a build signed with a different key.
3. Do not change device security settings if your organization or Android
   blocks the installation.

## Try the editor and send feedback

Use a copy of media you are comfortable testing with. Create a project, import
ordinary video and audio, add media to the timeline, revise a clip, preview,
add a title or manual caption, save, close, reopen, and export. Play the export
in a separate media player. Desktop builds also include local English
transcription; confirm the optional model download and review/edit its caption
proposal before applying it. Android transcription is not supported.

Report the operating system/device, source media format and duration, steps,
what you expected, and what happened. Do not attach private media, credentials,
or unredacted logs.

## Update and uninstall

Keep project files in a separate folder from the application package. To update,
download and extract the newer package, then replace the old app or extracted
folder. Android can update in place only when the installed package has a
compatible signing key. To uninstall, remove the app or extracted package (or
uninstall it through Android settings). Project files and exports are stored
separately and are not removed by uninstalling the app.
