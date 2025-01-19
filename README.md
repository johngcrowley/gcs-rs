# GCS Client in Rust 

```
./gcs-rs --op list --uri https://storage.googleapis.com/storage/v1/b/<BUCKET>/o/
```

My goal is to get `batch` endpoints working (upload, delete).

Cloud storage authentication API [docs](https://cloud.google.com/storage/docs/authentication#apiauth) state that the XML API uses HMAC keys in addition,
but the JSON API _only_ allows OAuth 2.0 tokens. I don't feel like running `$(gloud auth access-token-print)` each time to pass the Bearer token.

People seem to like [quick-xml](https://github.com/tafia/quick-xml?tab=readme-ov-file) and it implements bits of `Serde`.

# JSON `list objects`

- [docs](https://cloud.google.com/storage/docs/json_api/v1/objects/list)

# XML `list objects`

- [docs](https://cloud.google.com/storage/docs/xml-api/get-bucket-list)

