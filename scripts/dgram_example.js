const { DgramSocket } = require('../js/index');
const os = require('os');
const path = require('path');
const fs = require('fs');

const path1 = path.resolve(os.tmpdir(), './my_dgram_1.sock');
const path2 = path.resolve(os.tmpdir(), './my_dgram_2.sock');

try {
  fs.unlinkSync(path1);
  fs.unlinkSync(path2);
} catch (err) {}

const socket1 = new DgramSocket();
const socket2 = new DgramSocket();

socket1.bind(path1);
socket2.bind(path2);

socket2.on('data', (data, remoteAddr) => {
  console.log(`socket2 received: ${data.toString()}`);
  // echo
  socket2.sendTo(data, 0, data.length, remoteAddr);
});

socket1.on('data', (data) => {
  console.log(`socket1 received: ${data.toString()}`);
});

setInterval(() => {
  const buf = Buffer.from('hello');
  socket1.sendTo(buf, 0, buf.length, path2);
}, 1000);
