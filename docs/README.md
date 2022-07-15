nix-socket / [Exports](modules.md)

# nix-socket

`nix-socket` allows you to use some nonblocking sockets that are not supported by Node.js native modules, including:
- Using `SO_REUSEPORT` enabled TCP [net.Server](https://nodejs.org/dist/latest-v16.x/docs/api/net.html#class-netserver)
- unix seqpacket(`SOCK_SEQPACKET`) sockets
- unix datagram(`SOCK_DGRAM`) sockets

`nix-socket` is a [napi-rs](https://napi.rs/) based [Node.js addons](https://nodejs.org/docs/latest-v16.x/api/addons.html). This lib uses [libuv](https://libuv.org/) inside Node.js so that it won't introduce any other asynchronous runtimes.

## API Documents

[API Documents](./docs/modules.md)

## `SO_REUSEPORT` enabled TCP net.Server

The [cluster](https://nodejs.org/dist/latest-v18.x/docs/api/cluster.html) module share server ports by accepting new connections in the primary process and distributing them to worker processes.
With `SO_REUSEPORT`, sockets will be distributed by kernel instead, and which should be more performant especially for scenario of having a lot of short-lived connections.

Note that `SO_REUSEPORT` might behave much differently across operating systems. See this informative [post](https://stackoverflow.com/questions/14388706/how-do-so-reuseaddr-and-so-reuseport-differ) for more information.

### Example

```js
const { createReuseportFd } = require('nix-socket')
const { Server, Socket } = require('net')

const port = 8080
const host = '0.0.0.0'

// create multple servers listening to a same host, port.
for (let i = 0; i < 2; i += 1) {
  const fd = createReuseportFd(port, host)
  const server = new Server((socket) => {
    socket.on('data', (buf) => {
      console.log(`server ${i} received:`, buf)
      // echo
      socket.write(buf)
    })
  })

  server.listen({
    fd,
  }, () => {
    console.log(`server ${i} is listening on ${port}`)
  })
}

setInterval(() => {
    const client = new Socket()
    client.on('data', (buf) => {
      console.log('client received:', buf)
      client.destroy()
    })
    client.connect(port, host, () => {
      client.write(Buffer.from('hello'))
    })
}, 1000)
```

## Seqpacket Sockets

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

## Dgram Sockets

```js
```

## LICENSE
