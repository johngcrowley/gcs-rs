function {
    integer headers_out headers_in body_out body_in
    body=$(body)
    coproc {
        coproc :
        tac | tac
    }
    exec {headers_in}<&p
    exec {headers_out}>&p
    coproc {
        coproc :
        tac | tac
    }
    exec {body_in}<&p
    exec {body_out}>&p
    coproc :
    curl -sS -X POST \
        -D >(>&${headers_out}) \
        -o >(>&${body_out}) \
        -H 'Content-Type: multipart/mixed; boundary="===============7330845974216740156=="' \
        -H "Authorization: Bearer $(gcloud auth print-access-token)" \
        -d "$body" \
        https://storage.googleapis.com/batch/storage/v1
    exec {headers_out}>&-
    exec {body_out}>&-
    typeset line boundary
    while read line; do
        if [[ ${line%%:*} = content-type ]]; then
            boundary=${line##*boundary=}
        fi
    done <&${headers_in}
    typeset state=seeking
    typeset -A headers
    typeset statuses=()
    integer index=-1
    while read -r line; do
        case $state:$line in
            seeking:--$boundary )
                state=collecting
                index=0
                ;;
            collecting:Content-ID* )
                index=${${line##*response-}%>*}
                ;;
            collecting:HTTP* )
                (( index )) && statuses[$index]=${${line#* }%% *}
                ;;
        esac
    done <&${body_in}
    print $statuses
}
