# GCS Client in Rust 

```
cargo run
```

### Goals
 * [x] List -- single, no pagination ([docs](https://cloud.google.com/storage/docs/json_api/v1/objects/list))
 * [x] Upload ([docs](https://cloud.google.com/storage/docs/resumable-uploads#rest-apis))
 * [ ] Resumable Upload ([docs](https://cloud.google.com/storage/docs/performing-resumable-uploads))
 * [ ] Streaming Upload
  * I'm only doing a [put](https://github.com/neondatabase/neon/blob/main/libs/remote_storage/src/s3_bucket.rs#L718C7-L727C21).
  * But with "multiple chunk upload" Resumable upload, since I see a `Content-Length` header in Neon's request.
  * So I need to handle the resumable error.
 * [ ] Delete With Loop
 * [ ] Bulk Delete 
 * [ ] Paginated List / "List Streaming"


### Error Handling
 * User input isn't a valid file for upload 
 * Object doesn't exist for delete, download
 * Wrap type over `reqwest`
 

### Resumable Uploads

See [considerations](https://cloud.google.com/storage/docs/resumable-uploads#considerations)

1. Session URI expires after one week (404 Not Found) or could be (410 Gone)
    -> initiate new resumable upload
2. Integrity Check
    -> add MD5 Digest of source file to `Content-MD5` header of `PUT` request and check ... ?
3. Retries (Resumable)
    -> Successful upload receives a `200 OK` or `201 Created` along with metadata
    -> If interrupted, we receive a `5xx` response.

**Check Status**
```zsh
curl -i -X PUT \
    -H "Content-Length: 0" \
    -H "Content-Range: bytes */OBJECT_SIZE" \
    "SESSION_URI"  
```
- OBJECT_SIZE is the total number of bytes in your object. If you don't know the full size
of your object, use * for this value.
- `200`/`201` would already be handled.
- `308 Resume Incomplete` would be what we're looking for.
    - Has `Range` header: pick up from there
    - Doesn't: start from beginning.

**Had a `Range` header:**
```zsh
curl -i -X PUT --data-binary @PARTIAL_OBJECT_LOCATION \
   -H "Content-Length: UPLOAD_SIZE_REMAINING" \
   -H "Content-Range: bytes NEXT_BYTE-LAST_BYTE/TOTAL_OBJECT_SIZE" \
   "SESSION_URI"

```
Where:
   - PARTIAL_OBJECT_LOCATION is the local path to the remaining portion of data that you want to upload.
   - UPLOAD_SIZE_REMAINING is the number of bytes you're uploading in the current request. For example, uploading the rest of an object with a total size of 20000000 that was interrupted after bytes 0-42 uploaded would have an UPLOAD_SIZE_REMAINING of 19999957.
   - NEXT_BYTE is the next integer after the value you saved in step 2. For example, if 42 is the upper value in step 2, the value for NEXT_BYTE is 43.
   - LAST_BYTE is the ending byte contained in this PUT request. For example, to finish uploading an object whose total size is 20000000, the value for LAST_BYTE is 19999999.
   - TOTAL_OBJECT_SIZE is the total size of the object you are uploading. For example, 20000000.
   - SESSION_URI is the value returned in the Location header when you initiated the resumable upload.

**Failures**
Under rare circumstances, a request to resume an interrupted upload might fail with a
non-retriable '4xx' error because permissions on the bucket have changed, or because the
integrity check on the final uploaded object detected a mismatch.


