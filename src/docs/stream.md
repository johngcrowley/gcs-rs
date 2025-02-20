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








