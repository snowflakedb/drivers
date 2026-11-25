package net.snowflake.client.internal.loader;

import java.io.File;
import java.io.IOException;
import java.sql.ResultSet;
import java.sql.SQLException;
import java.sql.Statement;
import net.snowflake.client.api.loader.Loader;
import net.snowflake.client.internal.log.SFLogger;
import net.snowflake.client.internal.log.SFLoggerFactory;

/** Class responsible for uploading a single data file. */
public class FileUploader implements Runnable {
  private static final SFLogger logger = SFLoggerFactory.getLogger(FileUploader.class);

  private static final int RETRY = 6;

  private static final String UPLOADED_STATUS = "UPLOADED";

  private final Thread _thread;
  private final StreamLoader _loader;
  private final String _stage;
  private final File _file;

  FileUploader(StreamLoader loader, String stage, File file) {
    logger.debug("Creating new FileUploader");
    _loader = loader;
    _thread = new Thread(this);
    _thread.setName("FileUploaderThread");
    _stage = stage;
    _file = file;
  }

  public synchronized void upload() {
    // throttle up will wait if too many files are uploading
    logger.debug("Scheduling file upload");
    _loader.throttleUp();
    _thread.start();
  }

  @Override
  public void run() {

    Throwable previousException = null;
    try {
      for (int attempt = 0; attempt <= RETRY; attempt++) {

        if (attempt == RETRY) {
          if (previousException != null) {
            _loader.abort(
                new Loader.ConnectionError(
                    String.format(
                        "File could not be uploaded to remote stage "
                            + "after retrying %d times: %s",
                        RETRY, _file.getCanonicalPath()),
                    Utils.getCause(previousException)));
          } else {
            _loader.abort(
                new Loader.ConnectionError(
                    String.format(
                        "File could not be uploaded to remote stage "
                            + "after retrying %d times: %s",
                        RETRY, _file.getCanonicalPath())));
          }
          break;
        }

        if (attempt > 0) {
          logger.warn("Will retry PUT after {} seconds", Math.pow(2, attempt));
          Thread.sleep(1000 * ((int) Math.pow(2, attempt)));
        }

        // In test mode the first two attempts fail without sending the PUT.
        if (_loader._testMode && attempt < 2) {
          logger.debug("Simulated upload failure: attempt={}, filename={}", attempt, _file);
          continue;
        }

        // Upload local files to a remote stage

        // No double quote is added _loader.getRemoteStage(), since
        // it is most likely "~". If not, we may need to double quote
        // them.
        String remoteStage = "@" + _loader.getRemoteStage() + "/" + remoteSeparator(_stage);

        try (Statement statement = _loader.getPutConnection().createStatement()) {
          // PUT takes the file path and stage location as literals, not bind parameters.
          // Backslashes in the path are escaped; single quotes are not.
          String putStatement =
              "PUT "
                  + (attempt > 0 ? "/* retry:" + attempt + " */ " : "")
                  + "'file://"
                  + _file.getCanonicalPath().replaceAll("\\\\", "\\\\\\\\")
                  + "' '"
                  + remoteStage
                  + "' parallel=10" // upload chunks in parallel
                  + " overwrite=true"; // skip file existence check
          if (_loader._compressDataBeforePut) {
            putStatement += " auto_compress=false" + " SOURCE_COMPRESSION=gzip";
          } else if (_loader._compressFileByPut) {
            putStatement += " auto_compress=true";
          } else {
            // don't compress file at all
            putStatement += " auto_compress=false";
          }

          logger.debug("Put Statement start: attempt={}", attempt);
          statement.execute(putStatement);
          logger.debug("Put Statement end: attempt={}", attempt);

          try (ResultSet putResult = statement.getResultSet()) {
            putResult.next();

            String file = localSeparator(putResult.getString("source"));
            String status = putResult.getString("status");
            String message = putResult.getString("message");

            if (UPLOADED_STATUS.equals(status)) {
              // UPLOAD is success
              _file.delete();
              break;
            } else {
              logger.warn("Failed to upload a file: status={}, filename={}", status, file);
              logger.debug("Upload failure message: {}", message);
            }
          }
        } catch (SQLException | IOException | RuntimeException e) {
          logger.warn("Failed to PUT on attempt {}: {}", attempt, e.getClass().getName());
          logger.debug("PUT failure detail: {}", e.getMessage());
          previousException = e;
        }
      }
    } catch (InterruptedException | IOException | RuntimeException t) {
      logger.error("PUT exception: {}", t.getClass().getName());
      _loader.abort(new Loader.ConnectionError(t.getMessage(), t.getCause()));
    } finally {
      _loader.throttleDown();
    }
  }

  public void join() {
    logger.debug("Joining threads");
    try {
      _thread.join(0);
    } catch (InterruptedException ex) {
      logger.error("Interrupted while joining the upload thread", ex);
    }
  }

  /**
   * convert any back slashes to forward slashes if necessary when converting a local filename to a
   * one suitable for S3
   *
   * @param fname a file name to PUT
   * @return A fname string for S3
   */
  private String remoteSeparator(String fname) {
    if (File.separatorChar == '\\') {
      return fname.replace("\\", "/");
    } else {
      return fname;
    }
  }

  /**
   * convert any forward slashes to back slashes if necessary when converting a S3 file name to a
   * local file name
   *
   * @param fname a file name to PUT
   * @return A fname string for the local FS (Windows/other Unix like OS)
   */
  private String localSeparator(String fname) {
    if (File.separatorChar == '\\') {
      return fname.replace("/", "\\");
    } else {
      return fname;
    }
  }
}
