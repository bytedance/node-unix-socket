const { SeqpacketServer, SeqpacketSocket } = require('../js/index');
const path = require('path');
const fs = require('fs')

const kSockets = 4;
const kDataSize = 20 * 1024;
const kTimes = 20;

console.log('pid', process.pid);

const kServerPath = path.resolve(
  __dirname,
  '../__test__/.tmp/seqpacket_memory.sock'
);
try {
  fs.unlinkSync(kServerPath);
} catch (err) {
  //
}
const server = new SeqpacketServer();
server.listen(kServerPath);

server.on('connection', socket => {
  socket.on('data', buf => {})

  socket.on('end', () => {
    socket.destroy();
  })
})

async function test() {
  for (let i = 0; i < kSockets; i += 1) {
    const socket = new SeqpacketSocket()
    socket.connect(kServerPath, async () => {
      const pList = [];
      for (let j = 0; j < kTimes; j += 1) {
        const buf = Buffer.allocUnsafe(kDataSize);
        const p = new Promise((resolve, reject) => {
          socket.write(buf, 0, buf.length, () => {
            resolve()
          });
        })
        pList.push(p)
      }
      await Promise.all(pList);
      socket.end(() => {
        socket.destroy();
      });
    })
  }
}

async function main() {
  setInterval(() => {
    test()
    if (global.gc) {
      global.gc()
    }
    console.log(process.memoryUsage());
  }, 500);
}

main();
