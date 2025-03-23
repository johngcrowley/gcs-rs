dir=${${ZSH_ARGZERO:a}%/*}
bearer="ya29.c.c0ASRK0GYz2uYs5isMUOgNAz7PiDVFY7qwpz5gHEJpwdG2E5slDKiXvk9gudzCmAFB1ipuDGv7YPUzwHlBpT3Lld5D8A2RRtRdonzxi_mXAWkA0oWzex_UPrK0wHRsRKZcJ9FVoAmieQJT-Dbl9k8qjC5qGkt_trhqHZZ1W8Rd5xhTs1pLQl1fKkV2khyTUah3LSomMJcrbalOmOmF_h07I85aB69Sv3ETpcsH-uaNtO6re9-oChTQ2BuXzOTzy_eVcSWQL6ExZ6Vmi_ZjYJt8XBUnCKtbWu12nkrFKqE6Aee_etPkxX1FFEXgIxfcDXDKxaD-cDlFhg051AuH90iEfgPK7iXONbnYYFKtKfRyzUC-FiBzHAwUfr4BT385P3Zpxsgnv2J1rr4W40xtV_IrcvcvFoltmdgyOq9zs-R2ictBirozImh5bhrd2ldJM-f6gU2BXV4Qg_o-fZh9oFwVdxmqbmxRvxbzFUowga4MWRtc776rahe6uyl3qhZZqhgtbhrwkWbpofYrUzBqmJqtjkXOypRaee4Snu-p6mSxa2lXUjBsmoQsziMF8MpnQkqFoVMki_ve0w9vkOxgbQhlB38d4QX70tRv2QJxOtQtZ7Vqo2wXBlMUgQY3vZuY_15UaxhMZlYpzv9mFyF1wuBpdddZumIfc63dBdZ03bVVpQ4j8SU7yfhF2k5oiZSWvVp0InX9Sde171fJ8946yY_vQaafcztgs0OQZYlq5rMUVQZ2Mqwj3vgMBOdBO2QdOnefihQdeob35UI_daQ8Zz5SZR10vt_03OOgqr8BZtMdSRB2XmrVOcZU4g_x_gdX7o29FUF6v01mySroy_ZXwJ7ORc0QZ3y6UcaxwIagms46moncu2ZwZQYlr0y5YByuWbgnIV5dqpMzxJ_g5tzyMhnoW7FxxOs9shZJ0ViyuJ2d_xugWwzlhQU4MO_phxv5XJ2d00x4fguaxomSO3QvmfF3_4c4JZM11oxUkdkJOIsxJ2MZjOmgtzXhFZn"

# e.g. 
#           zsh post.zsh "$(cargo run)"

# ---- Upload ----
#curl -v -X POST --data-binary @$dir/foo.txt \
#    -H "Authorization: Bearer $bearer" \
#    -H "Content-Type: text/csv" \
#    "https://storage.googleapis.com/upload/storage/v1/b/acrelab-production-us1c-transfer/o?uploadType=media&name=foo.txt"
#

# ---- Get ----

#typeset obj="foo.txt"
#curl -v -X GET \
#  -H "Authorization: Bearer $bearer" \
#  -o "./foo.txt" \
#  "https://storage.googleapis.com/storage/v1/b/acrelab-production-us1c-transfer/o/$obj?alt=media"
#
obj="cdl%2Fyankee%2Fboundary.jsonl"
bucket=acrelab-production-us1c-transfer
curl -v -X GET \
  -H "Authorization: Bearer $(gcloud auth print-access-token)" \
  "https://storage.googleapis.com/storage/v1/b/$bucket/o/$obj?alt=json" 


## ---- Resumable Upload Session URI ----
## A nudge to a rube-goldberg machine. Not idempotent. `POST`ing a user id will try to make another one. It's on the server. Flicking a domino into a black box.
#curl -i -X POST \
#    -H "Authorization: Bearer $bearer" \
#    -H "Content-Type: application/json" \
#    "https://storage.googleapis.com/upload/storage/v1/b/acrelab-production-us1c-transfer/o?uploadType=resumable&name=foo.txt"
#
#
## ---- Resumable Upload PUT ----
## Putting something there. Making a file. Idempotent. Overwrites. It is deterministic. Same result everytime. 
#curl -i -X PUT --data-binary @$dir/foo.txt \
#    -H "Content-Length: 12" "https://storage.googleapis.com/upload/storage/v1/b/acrelab-production-us1c-transfer/o?uploadType=resumable&name=foo.txt&upload_id=AFIdbgRNLVaMGdhUkx9pa5tLQd_viRiSlRTcOk9tMh1tVUp783VSlJ22Ju9ZhWnBGRoxEm3EaJvGucgrds2TgRX5QNlhjsH8jMYDTNy_HUDYYQ"
