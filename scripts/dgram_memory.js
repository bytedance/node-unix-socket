const path = require('path');
const fs = require('fs');
const { DgramSocket } = require('../js/index');

const kServerPath = path.resolve(
  __dirname,
  '../__test__/.tmp/dgram_memory.sock'
);

async function sendSomeBufs() {
  try {
    fs.unlinkSync(kServerPath);
  } catch (e) {}

  const client = new DgramSocket(() => {});
  const server = new DgramSocket(() => {});

  server.bind(kServerPath);

  const pList = [];

  for (let i = 0; i < 1024; i += 1) {
    const buf = Buffer.allocUnsafe(1024);
    let resolve;
    let reject;
    const p = new Promise((res, rej) => {
      resolve = res;
      reject = rej;
    });
    client.sendTo(buf, 0, buf.length, kServerPath, (err) => {
      if (err) {
        reject(err);
        return;
      }
      resolve();
    });

    pList.push(p);
  }

  await Promise.all(pList);
  client.close();
  server.close();
}

module.exports = {
  kServerPath,
};

if (module === require.main) {
  setInterval(() => {
    sendSomeBufs().catch((err) => {
      console.error('receive error', err);
    });
    if (global.gc) {
      global.gc()
    }
    console.log(process.memoryUsage());
  }, 500);
}
