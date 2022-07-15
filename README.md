# nix-socket

`nix-socket` allows you to use some nonblocking sockets that are not supported by Node.js native modules, including:
- unix seqpacket(`SOCK_SEQPACKET`) sockets
- unix datagram(`SOCK_DGRAM`) sockets
- Using `SO_REUSEPORT` for TCP [net.Server](https://nodejs.org/dist/latest-v16.x/docs/api/net.html#class-netserver)

`nix-socket` is a [napi-rs](https://napi.rs/) based [Node.js addons](https://nodejs.org/docs/latest-v16.x/api/addons.html). This lib uses [libuv](https://libuv.org/) inside Node.js so that it won't introduce other asynchronous runtimes.

## Examples

### Seqpacket Sockets

```js
const { SeqpacketServer, SeqpacketSocket } = require('nix-socket')
const os = require('os')
const path = require('path')

const bindPath = path.resolve(os.tmp(), './my.sock')

const server = new SeqpacketServer()

server.listen(bindPath)

const client = new SeqpacketSocket()

// TODO
client.write()
```

## API Documents

[API Documents](./docs/modules.md)
