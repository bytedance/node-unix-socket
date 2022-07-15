import * as path from 'path'
import * as fs from 'fs'
import { DgramSocketWrap } from '../js/index'

const kTmp = path.resolve(__dirname, './.tmp')
const kSocketpath = path.resolve(kTmp, './test.sock')
const kInvalidPath = path.resolve(kTmp, './A_PATH_THAT_DOESNT_EXIST')

function sliently(fn) {
  try { fn() } catch (_) { }
}

describe('DgramSocketWrap', () => {
  beforeAll(() => {
    sliently(() => fs.mkdirSync(kTmp))
  })
  beforeEach(async () => {
    sliently(() => fs.unlinkSync(kSocketpath))
  })

  it('should work', async () => {
    let resolve: any
    const readCb = new Promise((r) => {
      resolve = r
    })
    const client = new DgramSocketWrap(() => {})
    const server = new DgramSocketWrap((err, buf) => {
      resolve()
    })

    server.bind(kSocketpath)

    const waitCb = new Promise<void>((resolve, reject) => {
      const buf = Buffer.from('hello')
      client.sendTo(buf, 0, buf.length, kSocketpath, () => {
        resolve()
      })
    })

    await waitCb
    await readCb

    client.close()
    server.close()
  })

  it('should work when sending a lot of data', async () => {
    const receiveData: any[] = []
    const writePromiseList: any[] = []
    const msg = 'hello'

    const times = 1024;
    let received = 0

    let resolve: any
    let reject: any

    const waitDataPromise = new Promise((res, rej) => {
      resolve = res
      reject = rej
    })

    const client = new DgramSocketWrap(() => {})
    const server = new DgramSocketWrap((err, data) => {
      receiveData.push(data)
      received += 1
      if (received === times) {
        resolve()
      }
    })

    server.bind(kSocketpath)

    // Try to trigger a ENOBUFS
    for (let i = 0; i < times; i += 1) {
      const waitCb = new Promise<void>((resolve, reject) => {
        const buf = Buffer.from(msg)
        client.sendTo(buf, 0, buf.length, kSocketpath, (err) => {
          if (err) {
            reject(err)
            return
          }
          resolve()
        })
      })
      writePromiseList.push(waitCb)
    }

    await Promise.all(writePromiseList)
    await waitDataPromise

    receiveData.forEach(data => {
      expect(Buffer.isBuffer(data)).toBe(true)
      expect(data.length).toBe(msg.length)
    })

    client.close()
    server.close()
  })

  it('should throw errors when sendTo() fail', async () => {
    const client = new DgramSocketWrap(() => {})

    const buf = Buffer.from('hello')

    await expect(new Promise<void>((resolve, reject) => {
      client.sendTo(buf, 0, buf.length, kInvalidPath, (err) => {
        if (err) {
          return reject(err)
        }
        resolve()
      })
    })).rejects.toThrow()

    client.close()
  });

  it('should throw when trying to bind a path that is too long', async () => {
    const socket = new DgramSocketWrap(() => {})

    expect(() => socket.bind(path.resolve(kTmp, './' + 't'.repeat(65535)))).toThrow('path to bind is too long')

    socket.close()
  })

  it('should not emit segment fault when we delete the sock path of a DgramSocketWrap before closing it', async () => {
    const server = new DgramSocketWrap(() => {})
    server.bind(kSocketpath)
    const client = new DgramSocketWrap(() => {})
    {
      let buf = Buffer.from('hello');
      client.sendTo(buf, 0, buf.length, kSocketpath, () => {})
    }

    fs.unlinkSync(kSocketpath)
    const server2 = new DgramSocketWrap(() => {})
    server2.bind(kSocketpath)

    {
      let buf = Buffer.from('hello');
      client.sendTo(buf, 0, buf.length, kSocketpath, () => {})
    }

    client.close()
    server.close()
    server2.close()
  })
})
