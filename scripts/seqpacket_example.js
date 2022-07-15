const { SeqpacketServer, SeqpacketSocket } = require('../js/index')
const os = require('os')
const path = require('path')
const fs = require('fs')

const bindPath = path.resolve(os.tmpdir(), './my_seqpacket.sock')

try { fs.unlinkSync(bindPath) } catch (e) {}

const server = new SeqpacketServer()
server.listen(bindPath)
server.on('connection', socket => {
  socket.on('data', buf => {
    console.log('received', buf.toString())
  })
});

const client = new SeqpacketSocket()
client.connect(bindPath, () => {
  const data = [
    'hello, ',
    'w',
    'o',
    'r',
    'l',
    'd'
  ]

  for (const str of data) {
    client.write(Buffer.from(str))
  }
  client.end()
})
