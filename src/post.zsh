dir=${${ZSH_ARGZERO:a}%/*}
bearer="$1"

# e.g. 
#           zsh post.zsh "$(cargo run)"

# ---- Upload ----
curl -v -X POST --data-binary @$dir/foo.txt \
    -H "Authorization: Bearer $bearer" \
    -H "Content-Type: text/csv" \
    "https://storage.googleapis.com/upload/storage/v1/b/acrelab-production-us1c-transfer/o?uploadType=media&name=foo.txt"



# ---- Resumable Upload Session URI ----
# A nudge to a rube-goldberg machine. Not idempotent. `POST`ing a user id will try to make another one. It's on the server. Flicking a domino into a black box.
curl -i -X POST \
    -H "Authorization: Bearer $bearer" \
    -H "Content-Type: application/json" \
    "https://storage.googleapis.com/upload/storage/v1/b/acrelab-production-us1c-transfer/o?uploadType=resumable&name=foo.txt"


# ---- Resumable Upload PUT ----
# Putting something there. Making a file. Idempotent. Overwrites. It is deterministic. Same result everytime. 
curl -i -X PUT --data-binary @$dir/foo.txt \
    -H "Content-Length: 12" "https://storage.googleapis.com/upload/storage/v1/b/acrelab-production-us1c-transfer/o?uploadType=resumable&name=foo.txt&upload_id=AFIdbgRNLVaMGdhUkx9pa5tLQd_viRiSlRTcOk9tMh1tVUp783VSlJ22Ju9ZhWnBGRoxEm3EaJvGucgrds2TgRX5QNlhjsH8jMYDTNy_HUDYYQ"
