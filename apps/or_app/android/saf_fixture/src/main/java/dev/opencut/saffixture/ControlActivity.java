package dev.opencut.saffixture;

import android.app.Activity;
import android.content.Intent;
import android.media.MediaCodec;
import android.media.MediaExtractor;
import android.media.MediaFormat;
import android.net.Uri;
import android.os.Bundle;
import android.os.Process;
import android.provider.DocumentsContract;
import android.util.Log;
import java.io.File;
import java.io.FileOutputStream;
import java.io.FileInputStream;
import java.io.InputStream;
import java.nio.ByteBuffer;
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
                        File audio = new File(getFilesDir(), "music.mp3");
                        try (InputStream input = getAssets().open("cc0_music_excerpt.mp3"); FileOutputStream output = new FileOutputStream(audio)) {
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
                File exported = new File(getFilesDir(), "export.webm");
                JSONObject webm = "exportStatus".equals(operation)
                    ? inspectWebm(exported) : new JSONObject();
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
                    .put("audioBytes", new File(getFilesDir(), "music.mp3").length())
                    .put("projectBytes", projectFile.length())
                    .put("projectSha256", sha256(projectFile))
                    .put("exportBytes", exported.length())
                    .put("validWebm", webm.optBoolean("validWebm"))
                    .put("webmVideoMime", webm.optString("webmVideoMime"))
                    .put("webmAudioMime", webm.optString("webmAudioMime"))
                    .put("webmVideoSampleBytes", webm.optInt("webmVideoSampleBytes"))
                    .put("webmAudioSampleBytes", webm.optInt("webmAudioSampleBytes"))
                    .put("webmVideoDecodeComplete", webm.optBoolean("webmVideoDecodeComplete"))
                    .put("webmVideoDecodedBuffers", webm.optInt("webmVideoDecodedBuffers"))
                    .put("webmVideoDecodedBytes", webm.optLong("webmVideoDecodedBytes"))
                    .put("webmAudioDecodeComplete", webm.optBoolean("webmAudioDecodeComplete"))
                    .put("webmAudioDecodedBuffers", webm.optInt("webmAudioDecodedBuffers"))
                    .put("webmAudioDecodedBytes", webm.optLong("webmAudioDecodedBytes"))
                    .put("captionExportBytes", captionExport.length())
                    .put("validSrtCaption", captionText.contains("Imported SAF caption")
                        && captionText.contains("00:00:00,000 --> 00:00:01,000"));
                runOnUiThread(() -> { setResult(RESULT_OK, new Intent().putExtra("data", result.toString())); finish(); });
            } catch (Exception error) {
                runOnUiThread(() -> { setResult(RESULT_CANCELED, new Intent().putExtra("error", error.toString())); finish(); });
            }
        }, "fixture-control").start();
    }

    private static JSONObject inspectWebm(File file) {
        String videoMime = "";
        String audioMime = "";
        int videoSampleBytes = 0;
        int audioSampleBytes = 0;
        int videoTrack = -1;
        int audioTrack = -1;
        MediaExtractor extractor = new MediaExtractor();
        try {
            if (!file.isFile() || file.length() <= 4) return new JSONObject();
            extractor.setDataSource(file.getAbsolutePath());
            for (int index = 0; index < extractor.getTrackCount(); index++) {
                MediaFormat format = extractor.getTrackFormat(index);
                String mime = format.getString(MediaFormat.KEY_MIME);
                if (mime == null) continue;
                boolean video = mime.equals("video/x-vnd.on2.vp9");
                boolean audio = mime.equals("audio/opus");
                if (!video && !audio) continue;
                if (video) {
                    videoMime = mime;
                    videoTrack = index;
                }
                if (audio) {
                    audioMime = mime;
                    audioTrack = index;
                }
                extractor.selectTrack(index);
                extractor.seekTo(0, MediaExtractor.SEEK_TO_CLOSEST_SYNC);
                ByteBuffer sample = ByteBuffer.allocate(1024 * 1024);
                int bytes = extractor.readSampleData(sample, 0);
                if (video) videoSampleBytes = Math.max(bytes, 0);
                if (audio) audioSampleBytes = Math.max(bytes, 0);
                extractor.unselectTrack(index);
            }
        } catch (Exception error) {
            Log.w("OrSafFixture", "Independent WebM inspection failed", error);
        } finally {
            extractor.release();
        }
        DecodeStats videoDecode = decodeTrack(file, videoTrack);
        DecodeStats audioDecode = decodeTrack(file, audioTrack);
        try {
            return new JSONObject()
                .put("validWebm", videoSampleBytes > 0 && audioSampleBytes > 0
                    && videoDecode.complete && audioDecode.complete)
                .put("webmVideoMime", videoMime)
                .put("webmAudioMime", audioMime)
                .put("webmVideoSampleBytes", videoSampleBytes)
                .put("webmAudioSampleBytes", audioSampleBytes)
                .put("webmVideoDecodeComplete", videoDecode.complete)
                .put("webmVideoDecodedBuffers", videoDecode.outputBuffers)
                .put("webmVideoDecodedBytes", videoDecode.decodedBytes)
                .put("webmAudioDecodeComplete", audioDecode.complete)
                .put("webmAudioDecodedBuffers", audioDecode.outputBuffers)
                .put("webmAudioDecodedBytes", audioDecode.decodedBytes);
        } catch (Exception impossible) {
            throw new IllegalStateException(impossible);
        }
    }

    private static final long MAX_TRACK_DECODE_NANOS = TimeUnit.SECONDS.toNanos(20);

    private static final class DecodeStats {
        final boolean complete;
        final int outputBuffers;
        final long decodedBytes;

        DecodeStats(boolean complete, int outputBuffers, long decodedBytes) {
            this.complete = complete;
            this.outputBuffers = outputBuffers;
            this.decodedBytes = decodedBytes;
        }
    }

    private static DecodeStats decodeTrack(File file, int trackIndex) {
        if (trackIndex < 0) return new DecodeStats(false, 0, 0);
        MediaExtractor extractor = new MediaExtractor();
        MediaCodec decoder = null;
        boolean decoderStarted = false;
        int submittedSamples = 0;
        int outputBuffers = 0;
        long decodedBytes = 0;
        boolean inputEnded = false;
        boolean outputEnded = false;
        long deadline = System.nanoTime() + MAX_TRACK_DECODE_NANOS;
        MediaCodec.BufferInfo outputInfo = new MediaCodec.BufferInfo();
        try {
            extractor.setDataSource(file.getAbsolutePath());
            MediaFormat format = extractor.getTrackFormat(trackIndex);
            String mime = format.getString(MediaFormat.KEY_MIME);
            if (mime == null) return new DecodeStats(false, 0, 0);
            extractor.selectTrack(trackIndex);
            decoder = MediaCodec.createDecoderByType(mime);
            decoder.configure(format, null, null, 0);
            decoder.start();
            decoderStarted = true;

            while (!outputEnded && System.nanoTime() < deadline) {
                if (!inputEnded) {
                    int inputIndex = decoder.dequeueInputBuffer(5_000);
                    if (inputIndex >= 0) {
                        ByteBuffer input = decoder.getInputBuffer(inputIndex);
                        if (input == null) throw new IllegalStateException("Decoder input buffer is unavailable");
                        input.clear();
                        int sampleBytes = extractor.readSampleData(input, 0);
                        if (sampleBytes < 0) {
                            decoder.queueInputBuffer(inputIndex, 0, 0, 0,
                                MediaCodec.BUFFER_FLAG_END_OF_STREAM);
                            inputEnded = true;
                        } else {
                            long presentationTimeUs = Math.max(extractor.getSampleTime(), 0L);
                            decoder.queueInputBuffer(inputIndex, 0, sampleBytes,
                                presentationTimeUs, 0);
                            submittedSamples++;
                            extractor.advance();
                        }
                    }
                }

                int outputIndex = decoder.dequeueOutputBuffer(outputInfo, 5_000);
                if (outputIndex >= 0) {
                    if (outputInfo.size > 0) {
                        outputBuffers++;
                        decodedBytes += outputInfo.size;
                    }
                    outputEnded = (outputInfo.flags & MediaCodec.BUFFER_FLAG_END_OF_STREAM) != 0;
                    decoder.releaseOutputBuffer(outputIndex, false);
                }
            }
            if (!outputEnded) Log.w("OrSafFixture", "Independent WebM decoder exceeded its bounded verification time");
        } catch (Exception error) {
            Log.w("OrSafFixture", "Independent WebM track decode failed", error);
        } finally {
            if (decoder != null) {
                if (decoderStarted) {
                    try { decoder.stop(); } catch (Exception ignored) { }
                }
                decoder.release();
            }
            extractor.release();
        }
        return new DecodeStats(outputEnded && submittedSamples > 0 && outputBuffers > 0
            && decodedBytes > 0, outputBuffers, decodedBytes);
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
