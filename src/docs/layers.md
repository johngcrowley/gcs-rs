
# Wire Protocols:
-------------------------

**Application:**

- HTTP: headers, body, 2 new lines delimited -- for _web_ servers
- SMTP: uses MIME to format itself           -- for _email_ servers

->> Email client or Web Browser formats your data with To:/From:/Subject: or HTTP GET
headers, respectively. 
->> "MIME" sits within the data. Could be in SMTP or HTTP.

**Transport:**

- TCP/IP: the "UPS". not the truck itself, but the delivery service. handles
reliablitiy, error-checking, flow-control, just like UPS would handle your delivery for
you. It's a _service_. 

->> Adds port numbers (80 for HTTP, 25 for SMTP) and checksums to "track package"
->> Imagine it's adding a package barcode to track it in UPS's system

**Network:**

- IP: (internet protocol) -- the addresses the delivery truck has to navigate towards.

->> Wraps data in IP "packet", with source/destination addresses
->> This is the shipping label.

**Physical:**

- Ethernet: -- the delivery truck. How electricity travels on wires.

# Streaming

To participate in a concurrent system (Neon), we use queuing.

Streaming is queuing, and TCP is the queue -- flow control.

Would be bad if you kept shoveling bytes at a socket without regard for the socket's reception.

The file system you send from is a reservoir (persistent storage) where you can just stop reading
when the socket stops receiving. 

# Notes:
-------------------------------

Ethernet (LAYER 2)
TCP/IP is a streaming protocol. I stream bytes and you take them.  (LAYER 3)

Ports / conventions of TCP funnel which protocols show up at its door, else server shuts
down door

HTTP: (LAYER 7)
- headers, 2 new lines, body
- 1980's: that's all it is
MIME:
- add indirection. add headers/body key pairs within header / body
- headers say "multi part mime" whereby body is series of headers and bodies
- some format of indents/newlines 
- 1995, Netscape/Thunderbird. Body = html, refs to images, some url telling you which

With a file you can go around and wait for disk to spin around to retrive it
with HTTP you have to wait for the order of things being streamed based on headers

e.g. you query OpenSearch with HTTP, so you post/get with JSON.

with PostgreSQL you query in PG binary.

HTTP borrowed from email which borrowed from MIME.

email attachment: content-type multipart mime and no indication of message length, just look for next
header

Emails just look at headers to route the email to right server. Read until first set of
2 new lines.

