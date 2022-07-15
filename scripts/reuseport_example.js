const { createReuseportFd } = require('../js/index')
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
