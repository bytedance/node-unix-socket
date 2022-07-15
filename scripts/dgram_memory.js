const path = require('path')
const fs = require('fs')
const { DgramSocket } = require('../index')

const kServerPath = path.resolve(__dirname, '../__test__/.tmp/dgram_memory.sock')

async function sendSomeBufs() {
  try { fs.unlinkSync(kServerPath) } catch (e) {console.error(e)}

  const client = new DgramSocket(() => {})
  const server = new DgramSocket(() => {})

  server.bind(kServerPath)

  const pList = []

  for (let i = 0; i < 1024; i += 1) {
    const buf = Buffer.from('hello')
    let resolve
    let reject
    const p = new Promise((res, rej) => {
      resolve = res
      reject = rej
    })
    client.sendTo(buf, 0, buf.length, kServerPath, (err) => {
      if (err) {
        reject(err)
        return
      }
      resolve()
    })

    pList.push(p)
  }

  await Promise.all(pList)
  // console.log('finish')
  // client.close()
  // server.close()
}

module.exports = {
  kServerPath,
}

if (module === require.main) {
  setTimeout(() => {
    sendSomeBufs().catch(err => {
      console.error(err)
    })
    console.log(process.memoryUsage())
  }, 1000)
}
