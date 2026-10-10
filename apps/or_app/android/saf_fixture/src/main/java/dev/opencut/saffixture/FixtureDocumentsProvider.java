package dev.opencut.saffixture;

import android.database.Cursor;
import android.database.MatrixCursor;
import android.os.CancellationSignal;
import android.os.ParcelFileDescriptor;
import android.provider.DocumentsContract;
import android.provider.DocumentsProvider;
import android.util.Log;
import java.io.File;
import java.io.FileNotFoundException;
import java.io.FileOutputStream;
import java.io.InputStream;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicLong;

/** Separate-UID, permission-enforced acceptance fixture; never part of OR's APK. */
public final class FixtureDocumentsProvider extends DocumentsProvider {
    static final String AUTHORITY = "dev.opencut.saffixture.documents";
    static final AtomicLong opens = new AtomicLong();
    static volatile boolean missingRecovered = false;
    static volatile boolean blockNextOpen = false;
    static volatile CountDownLatch entered = new CountDownLatch(1);
    static volatile CountDownLatch release = new CountDownLatch(1);
    static final String[] ROOT_COLUMNS = {DocumentsContract.Root.COLUMN_ROOT_ID,
        DocumentsContract.Root.COLUMN_DOCUMENT_ID, DocumentsContract.Root.COLUMN_TITLE,
        DocumentsContract.Root.COLUMN_FLAGS, DocumentsContract.Root.COLUMN_MIME_TYPES};
    static final String[] DOC_COLUMNS = {DocumentsContract.Document.COLUMN_DOCUMENT_ID,
        DocumentsContract.Document.COLUMN_DISPLAY_NAME, DocumentsContract.Document.COLUMN_MIME_TYPE,
        DocumentsContract.Document.COLUMN_FLAGS, DocumentsContract.Document.COLUMN_SIZE};

    @Override public boolean onCreate() { return true; }
    @Override public Cursor queryRoots(String[] projection) {
        MatrixCursor cursor = new MatrixCursor(projection == null ? ROOT_COLUMNS : projection);
        MatrixCursor.RowBuilder row = cursor.newRow();
        row.add(DocumentsContract.Root.COLUMN_ROOT_ID, "acceptance");
        row.add(DocumentsContract.Root.COLUMN_DOCUMENT_ID, "root");
        row.add(DocumentsContract.Root.COLUMN_TITLE, "OR SAF acceptance");
        row.add(DocumentsContract.Root.COLUMN_FLAGS, DocumentsContract.Root.FLAG_LOCAL_ONLY
            | DocumentsContract.Root.FLAG_SUPPORTS_CREATE);
        row.add(DocumentsContract.Root.COLUMN_MIME_TYPES, "application/json\nvideo/x-matroska\nvideo/mp4\naudio/wav\napplication/x-subrip\ntext/vtt\ntext/plain");
        return cursor;
    }
    @Override public Cursor queryDocument(String id, String[] projection) {
        MatrixCursor cursor = new MatrixCursor(projection == null ? DOC_COLUMNS : projection);
        addDocument(cursor, id);
        return cursor;
    }
    @Override public Cursor queryChildDocuments(String parent, String[] projection, String sortOrder) {
        MatrixCursor cursor = new MatrixCursor(projection == null ? DOC_COLUMNS : projection);
        if (parent.equals("root")) {
            addDocument(cursor, "project");
            addDocument(cursor, "media");
            addDocument(cursor, "media-second");
            addDocument(cursor, "media-audio");
            addDocument(cursor, "relink-replacement");
            addDocument(cursor, "captions");
            if (new File(getContext().getFilesDir(), "export.mkv").isFile()) addDocument(cursor, "export");
            if (new File(getContext().getFilesDir(), "caption-export.srt").isFile()) addDocument(cursor, "caption-export");
            if (new File(getContext().getFilesDir(), "caption-export.vtt").isFile()) addDocument(cursor, "caption-export-vtt");
        }
        return cursor;
    }
    private void addDocument(MatrixCursor cursor, String id) {
        MatrixCursor.RowBuilder row = cursor.newRow();
        row.add(DocumentsContract.Document.COLUMN_DOCUMENT_ID, id);
        row.add(DocumentsContract.Document.COLUMN_DISPLAY_NAME, id.equals("root") ? "OR SAF acceptance" : id.equals("project") ? "acceptance.orproj" : id.equals("media") ? "tiny.mkv" : id.equals("media-second") ? "phone.mp4" : id.equals("media-audio") ? "tiny.wav" : id.equals("relink-replacement") ? "relink-replacement.mkv" : id.equals("captions") ? "captions.srt" : id.equals("caption-export") ? "caption-export.srt" : id.equals("caption-export-vtt") ? "caption-export.vtt" : "export.mkv");
        row.add(DocumentsContract.Document.COLUMN_MIME_TYPE, id.equals("root") ? DocumentsContract.Document.MIME_TYPE_DIR : id.equals("project") ? "application/json" : id.equals("media-audio") ? "audio/wav" : id.equals("media-second") ? "video/mp4" : id.equals("captions") || id.equals("caption-export") ? "application/x-subrip" : id.equals("caption-export-vtt") ? "text/vtt" : "video/x-matroska");
        row.add(DocumentsContract.Document.COLUMN_FLAGS, id.equals("root")
            ? DocumentsContract.Document.FLAG_DIR_SUPPORTS_CREATE
            : id.equals("export") || id.equals("caption-export") || id.equals("caption-export-vtt")
                ? DocumentsContract.Document.FLAG_SUPPORTS_WRITE | DocumentsContract.Document.FLAG_SUPPORTS_DELETE
                : id.equals("project") ? DocumentsContract.Document.FLAG_SUPPORTS_WRITE : 0);
        File file = new File(getContext().getFilesDir(), id.equals("project") ? "acceptance.orproj" : id.equals("export") ? "export.mkv" : id.equals("caption-export") ? "caption-export.srt" : id.equals("caption-export-vtt") ? "caption-export.vtt" : id.equals("captions") ? "captions.srt" : id.equals("media-audio") ? "tiny.wav" : id.equals("media-second") ? "tiny_h264_aac.mp4" : "tiny.mkv");
        row.add(DocumentsContract.Document.COLUMN_SIZE, file.length());
    }

    @Override public String createDocument(String parentId, String mimeType, String displayName) throws FileNotFoundException {
        if (!parentId.equals("root") || (!mimeType.equals("video/x-matroska") && !mimeType.equals("application/x-subrip") && !mimeType.equals("text/vtt"))) throw new FileNotFoundException("Unsupported fixture document");
        Log.i("OrSafFixture", "createDocument parent=" + parentId + " mime=" + mimeType);
        boolean caption = mimeType.equals("application/x-subrip") || mimeType.equals("text/vtt");
        boolean webVtt = mimeType.equals("text/vtt");
        File output = new File(getContext().getFilesDir(), webVtt ? "caption-export.vtt" : caption ? "caption-export.srt" : "export.mkv");
        if (output.exists() && !output.delete()) throw new FileNotFoundException("Existing export could not be replaced");
        try {
            if (!output.createNewFile()) throw new FileNotFoundException("Export document could not be created");
        } catch (java.io.IOException error) { throw new FileNotFoundException("Export document could not be created"); }
        return webVtt ? "caption-export-vtt" : caption ? "caption-export" : "export";
    }

    @Override public void deleteDocument(String documentId) throws FileNotFoundException {
        Log.i("OrSafFixture", "deleteDocument id=" + documentId);
        boolean webVtt = documentId.equals("caption-export-vtt");
        boolean caption = documentId.equals("caption-export") || webVtt;
        if ((!caption && !documentId.equals("export")) || !new File(getContext().getFilesDir(), webVtt ? "caption-export.vtt" : caption ? "caption-export.srt" : "export.mkv").delete()) {
            throw new FileNotFoundException("Export document could not be deleted");
        }
    }
    @Override public ParcelFileDescriptor openDocument(String id, String mode, CancellationSignal signal) throws FileNotFoundException {
        Log.i("OrSafFixture", "openDocument id=" + id + " mode=" + mode);
        if ((id.equals("export") || id.equals("caption-export") || id.equals("caption-export-vtt")) && (mode.contains("w") || mode.contains("t"))) {
            File file = new File(getContext().getFilesDir(), id.equals("export") ? "export.mkv" : id.equals("caption-export-vtt") ? "caption-export.vtt" : "caption-export.srt");
            return ParcelFileDescriptor.open(file,
                ParcelFileDescriptor.MODE_WRITE_ONLY | ParcelFileDescriptor.MODE_TRUNCATE);
        }
        if (id.equals("project") && (mode.contains("w") || mode.contains("t"))) {
            return ParcelFileDescriptor.open(new File(getContext().getFilesDir(), "acceptance.orproj"),
                ParcelFileDescriptor.MODE_WRITE_ONLY | ParcelFileDescriptor.MODE_TRUNCATE);
        }
        if (!mode.equals("r")) throw new FileNotFoundException("Read-only acceptance source");
        opens.incrementAndGet();
        if (id.equals("missing") && !missingRecovered) throw new FileNotFoundException("Missing acceptance source");
        if (id.equals("blocked") && blockNextOpen) {
            blockNextOpen = false;
            entered.countDown();
            try {
                if (!release.await(10, TimeUnit.SECONDS)) throw new FileNotFoundException("Blocked-open handshake failed");
            } catch (InterruptedException error) {
                Thread.currentThread().interrupt();
                throw new FileNotFoundException("Blocked-open cancelled");
            }
        }
        File file = new File(getContext().getFilesDir(), id.equals("project") ? "acceptance.orproj" : id.equals("export") ? "export.mkv" : id.equals("captions") ? "captions.srt" : id.equals("caption-export") ? "caption-export.srt" : id.equals("caption-export-vtt") ? "caption-export.vtt" : id.equals("media-audio") ? "tiny.wav" : id.equals("media-second") ? "tiny_h264_aac.mp4" : "tiny.mkv");
        if (id.equals("pipe")) {
            try {
                ParcelFileDescriptor[] pipe = ParcelFileDescriptor.createPipe();
                new Thread(() -> {
                    try (FileOutputStream output = new ParcelFileDescriptor.AutoCloseOutputStream(pipe[1]); InputStream input = new java.io.FileInputStream(file)) {
                        byte[] buffer = new byte[4096];
                        int count;
                        while ((count = input.read(buffer)) != -1) output.write(buffer, 0, count);
                    } catch (java.io.IOException ignored) { /* Reader rejects this nonseekable source. */ }
                }, "fixture-pipe").start();
                return pipe[0];
            } catch (java.io.IOException error) { throw new FileNotFoundException("Pipe unavailable"); }
        }
        return ParcelFileDescriptor.open(file, ParcelFileDescriptor.MODE_READ_ONLY);
    }
}
