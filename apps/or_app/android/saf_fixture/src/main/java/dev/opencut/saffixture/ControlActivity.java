package dev.opencut.saffixture;

import android.app.Activity;
import android.content.Intent;
import android.net.Uri;
import android.os.Bundle;
import android.os.Process;
import android.provider.DocumentsContract;
import android.util.Log;
import java.io.File;
import java.io.FileOutputStream;
import java.io.FileInputStream;
import java.io.InputStream;
import java.security.MessageDigest;
import java.nio.charset.StandardCharsets;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.TimeUnit;
import org.json.JSONObject;

/** Explicit fixture setup/revocation, retaining Android's actual URI permission checks. */
public final class ControlActivity extends Activity {
    static final String OR_PACKAGE = "io.github.huou07.or_app";
    // The acceptance journey seeds a project with more than 64 library entries, so
    // OR's own canonical save is ~66 KiB of pretty-printed JSON. This bound must
    // admit that real document and stay well inside the ~1 MiB Binder transaction
    // that carries the Intent between the two UIDs.
    static final int MAX_PROJECT_BYTES = 256 * 1024;
    @Override public void onCreate(Bundle state) {
        super.onCreate(state);
        new Thread(() -> {
            try {
                String operation = getIntent().getStringExtra("operation");
                Uri uri = Uri.parse(getIntent().getStringExtra("uri") == null
                    ? DocumentsContract.buildDocumentUri(FixtureDocumentsProvider.AUTHORITY, "good").toString()
                    : getIntent().getStringExtra("uri"));
                if (!FixtureDocumentsProvider.AUTHORITY.equals(uri.getAuthority())) throw new IllegalArgumentException("Fixture URI required");
                switch (operation == null ? "status" : operation) {
                    case "seedProject":
                        File media = new File(getFilesDir(), "tiny.mkv");
                        try (InputStream input = getAssets().open("tiny.mkv"); FileOutputStream output = new FileOutputStream(media)) {
                            byte[] buffer = new byte[4096];
                            int count;
                            while ((count = input.read(buffer)) != -1) output.write(buffer, 0, count);
                        }
                        File audio = new File(getFilesDir(), "tiny.wav");
                        try (InputStream input = getAssets().open("tiny.wav"); FileOutputStream output = new FileOutputStream(audio)) {
                            byte[] buffer = new byte[4096];
                            int count;
                            while ((count = input.read(buffer)) != -1) output.write(buffer, 0, count);
                        }
                        File phone = new File(getFilesDir(), "tiny_h264_aac.mp4");
                        try (InputStream input = getAssets().open("tiny_h264_aac.mp4"); FileOutputStream output = new FileOutputStream(phone)) {
                            byte[] buffer = new byte[4096];
                            int count;
                            while ((count = input.read(buffer)) != -1) output.write(buffer, 0, count);
                        }
                        String project = getIntent().getStringExtra("projectJson");
                        byte[] encoded = project == null ? new byte[0] : project.getBytes(StandardCharsets.UTF_8);
                        if (encoded.length > MAX_PROJECT_BYTES) throw new IllegalArgumentException(
                            "Bounded fixture project required: " + encoded.length + " bytes exceeds " + MAX_PROJECT_BYTES);
                        try (FileOutputStream output = new FileOutputStream(new File(getFilesDir(), "acceptance.orproj"))) { output.write(encoded); }
                        try (FileOutputStream output = new FileOutputStream(new File(getFilesDir(), "captions.srt"))) {
                            output.write("1\n00:00:00,000 --> 00:00:01,000\nImported SAF caption\n".getBytes(StandardCharsets.UTF_8));
                        }
                        for (String id : new String[]{"good", "late65", "missing", "pipe", "blocked"}) {
                            grantUriPermission(OR_PACKAGE, DocumentsContract.buildDocumentUri(FixtureDocumentsProvider.AUTHORITY, id), Intent.FLAG_GRANT_READ_URI_PERMISSION);
                        }
                        break;
                    case "grant": grantUriPermission(OR_PACKAGE, uri, Intent.FLAG_GRANT_READ_URI_PERMISSION); break;
                    case "persistMediaGrant":
                        grantUriPermission(
                            OR_PACKAGE,
                            uri,
                            Intent.FLAG_GRANT_READ_URI_PERMISSION | Intent.FLAG_GRANT_PERSISTABLE_URI_PERMISSION);
                        break;
                    case "revoke": revokeUriPermission(OR_PACKAGE, uri, Intent.FLAG_GRANT_READ_URI_PERMISSION); break;
                    case "recoverMissing": FixtureDocumentsProvider.missingRecovered = true; break;
                    case "resetMissing": FixtureDocumentsProvider.missingRecovered = false; break;
                    case "armBlocked":
                        FixtureDocumentsProvider.entered = new CountDownLatch(1);
                        FixtureDocumentsProvider.release = new CountDownLatch(1);
                        FixtureDocumentsProvider.blockNextOpen = true;
                        break;
                    case "awaitBlocked":
                        if (!FixtureDocumentsProvider.entered.await(10, TimeUnit.SECONDS)) throw new IllegalStateException("No blocked provider open");
                        break;
                    case "releaseBlocked": FixtureDocumentsProvider.release.countDown(); break;
                    case "status": break;
                    case "exportStatus": break;
                    default: throw new IllegalArgumentException("Unknown fixture operation");
                }
                File projectFile = new File(getFilesDir(), "acceptance.orproj");
                Log.i(
                    "OrSafFixture",
                    "control operation=" + (operation == null ? "status" : operation)
                        + " projectSha256=" + sha256(projectFile)
                );
                File exported = new File(getFilesDir(), "export.mkv");
                boolean validMatroska = false;
                if (exported.isFile() && exported.length() >= 4) {
                    try (InputStream input = new java.io.FileInputStream(exported)) {
                        validMatroska = input.read() == 0x1a && input.read() == 0x45
                            && input.read() == 0xdf && input.read() == 0xa3;
                    }
                }
                File captionExport = new File(getFilesDir(), "caption-export.srt");
                String captionText = "";
                if (captionExport.isFile() && captionExport.length() <= 1024 * 1024) {
                    byte[] captionBytes = new byte[(int) captionExport.length()];
                    try (FileInputStream input = new FileInputStream(captionExport)) {
                        int offset = 0;
                        while (offset < captionBytes.length) {
                            int count = input.read(captionBytes, offset, captionBytes.length - offset);
                            if (count < 0) break;
                            offset += count;
                        }
                        captionText = new String(captionBytes, 0, offset, StandardCharsets.UTF_8);
                    }
                }
                JSONObject result = new JSONObject().put("providerUid", Process.myUid())
                    .put("providerOpens", FixtureDocumentsProvider.opens.get()).put("mediaBytes", new File(getFilesDir(), "tiny.mkv").length())
                    .put("audioBytes", new File(getFilesDir(), "tiny.wav").length())
                    .put("projectBytes", projectFile.length())
                    .put("projectSha256", sha256(projectFile))
                    .put("exportBytes", exported.length())
                    .put("validMatroska", validMatroska)
                    .put("captionExportBytes", captionExport.length())
                    .put("validSrtCaption", captionText.contains("Imported SAF caption")
                        && captionText.contains("00:00:00,000 --> 00:00:01,000"));
                runOnUiThread(() -> { setResult(RESULT_OK, new Intent().putExtra("data", result.toString())); finish(); });
            } catch (Exception error) {
                runOnUiThread(() -> { setResult(RESULT_CANCELED, new Intent().putExtra("error", error.toString())); finish(); });
            }
        }, "fixture-control").start();
    }

    private static String sha256(File file) throws Exception {
        MessageDigest digest = MessageDigest.getInstance("SHA-256");
        try (InputStream input = new java.io.FileInputStream(file)) {
            byte[] buffer = new byte[8192];
            int count;
            while ((count = input.read(buffer)) != -1) digest.update(buffer, 0, count);
        }
        StringBuilder result = new StringBuilder(64);
        for (byte value : digest.digest()) result.append(String.format("%02x", value));
        return result.toString();
    }
}
