import * as path from 'path'
import * as fs from 'fs'
import { DgramSocket } from '../js/index'

const kTmp = path.resolve(__dirname, './.tmp')
const kServerPath = path.resolve(kTmp, './server.sock')
const kClientPath = path.resolve(kTmp, './client.sock')
const kInvalidPath = path.resolve(kTmp, './A_PATH_THAT_DOESNT_EXIST')

const emptyFn = () => {}

function sliently(fn) {
  try { fn() } catch (_) { }
}

function createDefer<T>() {
  let resolve, reject

  const p = new Promise<T>((res, rej) => {
    resolve = res
    reject = rej
  })

  return {
    p,
    resolve,
    reject,
  }
}

describe('DgramSocket', () => {
  beforeAll(() => {
    sliently(() => fs.mkdirSync(kTmp))
  })
  beforeEach(async () => {
    sliently(() => fs.unlinkSync(kServerPath))
    sliently(() => fs.unlinkSync(kClientPath))
  })

  it('should work', async () => {
    let resolve: any
    const readCb = new Promise((r) => {
      resolve = r
    })
    const msg = 'hello'
    const mockFn = jest.fn((err, buf, filepath) => {
      try {
        expect(buf.toString()).toBe(msg)
        expect(filepath).toBe('')
      } catch (err) {
        console.log(err)
      }
      resolve()
    })
    const client = new DgramSocket(() => {})
    const server = new DgramSocket(mockFn as any)

    server.bind(kServerPath)

    const waitCb = new Promise<void>((resolve, reject) => {
      const buf = Buffer.from(msg)
      client.sendTo(buf, 0, buf.length, kServerPath, () => {
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

    const client = new DgramSocket(() => {})
    const server = new DgramSocket((err, data) => {
      receiveData.push(data)
      received += 1
      if (received === times) {
        resolve()
      }
    })

    server.bind(kServerPath)

    // Try to trigger a ENOBUFS
    for (let i = 0; i < times; i += 1) {
      const waitCb = new Promise<void>((resolve, reject) => {
        const buf = Buffer.from(msg)
        client.sendTo(buf, 0, buf.length, kServerPath, (err) => {
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
    const client = new DgramSocket(() => {})

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
    const socket = new DgramSocket(() => {})

    expect(() => socket.bind(path.resolve(kTmp, './' + 't'.repeat(65535)))).toThrow('path to bind is too long')

    socket.close()
  })

  it('should not emit segment fault when we delete the sock path of a DgramSocket before closing it', async () => {
    const server = new DgramSocket(() => {})
    server.bind(kServerPath)
    const client = new DgramSocket(() => {})
    {
      let buf = Buffer.from('hello');
      client.sendTo(buf, 0, buf.length, kServerPath, () => {})
    }

    fs.unlinkSync(kServerPath)
    const server2 = new DgramSocket(() => {})
    server2.bind(kServerPath)

    {
      let buf = Buffer.from('hello');
      client.sendTo(buf, 0, buf.length, kServerPath, () => {})
    }

    client.close()
    server.close()
    server2.close()
  })

  it('should work when we call close in callbacks', async () => {
    const server = new DgramSocket(() => {
      server.close()
    })
    server.bind(kServerPath)
    const client = new DgramSocket(() => {})

    const buf = Buffer.from('hello')
    const afterSend = () => {
      process.nextTick(() => {
        client.close();
      })
    };
    client.sendTo(buf, 0, buf.length, kServerPath, afterSend);
    client.sendTo(buf, 0, buf.length, kServerPath, afterSend);
  })

  it('should return remote path correctly', async () => {
    let resolve

    let waitMsg = new Promise((res, rej) => {
      resolve = res
    });

    const mockFn = jest.fn(() => {
      resolve()
    })
    const client = new DgramSocket(emptyFn)
    client.bind(kClientPath)
    const server = new DgramSocket(mockFn)
    server.bind(kServerPath)

    const buf = Buffer.from('hello')
    client.sendTo(buf, 0, buf.length, kServerPath, emptyFn)

    await waitMsg

    expect(mockFn.mock.calls.length).toBe(1)
    const call: any = mockFn.mock.calls[0]
    expect(call[2]).toBe(kClientPath);

    client.close()
    server.close()
  })

  // TODO
  xit('should emit "uncaughtException" when throw errors in callbacks', async () => {
    let resolve

    let waitMsg = new Promise((res, rej) => {
      resolve = res
    });

    process.on('uncaughtException', (e) => {
      console.log('ee', e)
      resolve(e)
    })

    const client = new DgramSocket(emptyFn)
    const server = new DgramSocket(() => {
      throw new Error('error')
    })
    server.bind(kServerPath)

    const buf = Buffer.from('hello')
    client.sendTo(buf, 0, buf.length, kServerPath, emptyFn)
    const e = await waitMsg

    client.close()
    server.close()
  });

  it('should allow to send zeroed msg ', async () => {
    const { p, resolve } = createDefer<Buffer>()

    const client = new DgramSocket(emptyFn)
    const server = new DgramSocket((err, buf) => {
      resolve(buf)
    })
    server.bind(kServerPath)

    const buf = Buffer.alloc(1024).fill(0);
    client.sendTo(buf, 0, buf.length, kServerPath, emptyFn)

    const bufReceived = await p
    expect(buf.toString('hex')).toBe(bufReceived.toString('hex'))

    client.close()
    server.close()
  })

  it('should allow to send a long msg (although th msg might get dropped)', async () => {
    const { p, resolve } = createDefer<Buffer>()

    const client = new DgramSocket(emptyFn)
    const server = new DgramSocket(emptyFn)
    server.bind(kServerPath)

    const buf = Buffer.alloc(1024 * 1024).fill(0);
    const sendCb = jest.fn();
    client.sendTo(buf, 0, buf.length, kServerPath, () => {
      resolve()
    })

    await p

    client.close()
    server.close()
  })
})
