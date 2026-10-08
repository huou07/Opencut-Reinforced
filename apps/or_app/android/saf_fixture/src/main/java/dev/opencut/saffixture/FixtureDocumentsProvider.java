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
        row.add(DocumentsContract.Root.COLUMN_MIME_TYPES, "application/json\nvideo/x-matroska");
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
            if (new File(getContext().getFilesDir(), "export.mkv").isFile()) addDocument(cursor, "export");
        }
        return cursor;
    }
    private void addDocument(MatrixCursor cursor, String id) {
        MatrixCursor.RowBuilder row = cursor.newRow();
        row.add(DocumentsContract.Document.COLUMN_DOCUMENT_ID, id);
        row.add(DocumentsContract.Document.COLUMN_DISPLAY_NAME, id.equals("root") ? "OR SAF acceptance" : id.equals("project") ? "acceptance.orproj" : "export.mkv");
        row.add(DocumentsContract.Document.COLUMN_MIME_TYPE, id.equals("root") ? DocumentsContract.Document.MIME_TYPE_DIR : id.equals("project") ? "application/json" : "video/x-matroska");
        row.add(DocumentsContract.Document.COLUMN_FLAGS, id.equals("root")
            ? DocumentsContract.Document.FLAG_DIR_SUPPORTS_CREATE
            : id.equals("export")
                ? DocumentsContract.Document.FLAG_SUPPORTS_WRITE | DocumentsContract.Document.FLAG_SUPPORTS_DELETE
                : 0);
        File file = new File(getContext().getFilesDir(), id.equals("project") ? "acceptance.orproj" : id.equals("export") ? "export.mkv" : "tiny.mkv");
        row.add(DocumentsContract.Document.COLUMN_SIZE, file.length());
    }

    @Override public String createDocument(String parentId, String mimeType, String displayName) throws FileNotFoundException {
        if (!parentId.equals("root") || !mimeType.equals("video/x-matroska")) throw new FileNotFoundException("Unsupported fixture document");
        Log.i("OrSafFixture", "createDocument parent=" + parentId + " mime=" + mimeType);
        File output = new File(getContext().getFilesDir(), "export.mkv");
        if (output.exists() && !output.delete()) throw new FileNotFoundException("Existing export could not be replaced");
        try {
            if (!output.createNewFile()) throw new FileNotFoundException("Export document could not be created");
        } catch (java.io.IOException error) { throw new FileNotFoundException("Export document could not be created"); }
        return "export";
    }

    @Override public void deleteDocument(String documentId) throws FileNotFoundException {
        Log.i("OrSafFixture", "deleteDocument id=" + documentId);
        if (!documentId.equals("export") || !new File(getContext().getFilesDir(), "export.mkv").delete()) {
            throw new FileNotFoundException("Export document could not be deleted");
        }
    }
    @Override public ParcelFileDescriptor openDocument(String id, String mode, CancellationSignal signal) throws FileNotFoundException {
        Log.i("OrSafFixture", "openDocument id=" + id + " mode=" + mode);
        if (id.equals("export") && (mode.contains("w") || mode.contains("t"))) {
            return ParcelFileDescriptor.open(new File(getContext().getFilesDir(), "export.mkv"),
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
        File file = new File(getContext().getFilesDir(), id.equals("project") ? "acceptance.orproj" : id.equals("export") ? "export.mkv" : "tiny.mkv");
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
