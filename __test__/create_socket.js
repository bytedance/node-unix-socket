const { DgramSocket } = require('../js')
const fs = require('fs')
const path = require('path')

const serverPath = path.resolve(__dirname, './.tmp/worker_server.sock')
try {
  fs.unlinkSync(serverPath)
} catch (err) {
  //
}
const socket = new DgramSocket(serverPath);
socket.bind(serverPath)
