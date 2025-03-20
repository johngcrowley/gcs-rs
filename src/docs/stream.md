Passing the Body (where the bytes stream / data is) through the function, pinning it in memory, and returning that
to be iterated on as a `Stream` in Rust, which is an async iterator, whose `.next()` method, from the `futures_util::StreamExt` trait,
converts each item into a `Future` to be awaited.

I needed to "parse" the headers (metadata, etag, etc) but didnt want to have to collect all the water out of the pipe into a serialized struct
just to tell the water's temperature. So i do a first request with the `alt=json` URI modifier, parse the `.text()` of the whole response into that struct.

Then I can call the URI with `alt=media` (download the byte stream) and pass _that_ to a pinned field in my return object, `Download` type.

---

 Eureka:
 1. Reqwest is .await-ing on the socket to open. that's it.
 2. We check the header status_code to continue or not. we can check the color of water
    without having to collect all of it!
 3. We then call 'bytes_stream' to get a `Stream`. This is an aynchronous iterator.
 4. Each call to it looks like `.next().await` which is what creates the `Future`
 5. But! We don't do that here. We do it in the outer functions of Neon that call this
    function.
    https://github.com/neondatabase/neon/blob/55cb07f680603ff768a3cbe1ff8367a4fe8566e2/libs/remote_storage/src/local_fs.rs#L1194C1-L1203C16
 6. We have to apply a mask over our stream with Serde
 7. And to return a Stream from a function we need to Pin it in memory.
 --- Those two requirements are what I need to do-.
 Notes:
 - the `tokio::select!` thing in the S3 download function is just a race. It's checking
   if the timeout Future finishes first before the request.

---

Stream is an abstraction. Consistent across languages.

Linux Kernel only has `read()` `close()` and `open()` etc SysCalls.

Unix has the concept of Streams, but its an abstraction.

tokio::fs::File is a file stream

BufReader (1024*1024) reads 1MiB of that file

---

8TiB log lines.

piping that to unix 'sort -k 1,1'. imagine the last line, column 1 is 0 (first).
'sort' has to bring that whole file into memory. 

'cat', 'wc' or 'grep' go line-by-line.

imagine 'cat' is called by a coproc.
'cat' could put on backpressure if nothings taking it.

'tac | tac'

'tail' isn't streaming op.
'head' is.

---

Linux OPEN returns a file descriptor (FD), an integer

NodeJS:
filehandle.read(buffer, offset, length, position)
- if you pass a "position" it will do a SEEK which is not a streaming operation (think 'tail')

Linux READ returns an integer, which is how much was actually read.
- takes int FD (file descriptor), buffer and count. buffer should be "count" big.

AsyncRead in Rust is essentially OPEN but a wrapper around a file descriptor
BufReader is a decorator on that READ. 
So if your buffer size is tiny, you're making a ton of system calls.








