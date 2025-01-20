# GCS Client in Rust 

```
cargo run -- --op whatever --uri whatever
```
- For now, `--op` and `--uri` values are hardcoded for early development.

### Goals
* [ ] List -- single, no pagination
 1. **`list`**
  - [docs](https://cloud.google.com/storage/docs/json_api/v1/objects/list)
  
- [ ] Upload
 1. **`upload`**
  - [docs](https://cloud.google.com/storage/docs/resumable-uploads#rest-apis)
  ```
  The Cloud Storage JSON API uses a POST Object request that includes the query parameter uploadType=resumable to initiate the resumable upload. This request returns as session URI that you then use in one or more PUT Object requests to upload the object data. For a step-by-step guide to building your own logic for resumable uploading, see Performing resumable uploads.
  ```



