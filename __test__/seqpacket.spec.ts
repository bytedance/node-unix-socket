import * as path from 'path';
import * as fs from 'fs';
import { SeqpacketSocket, SeqpacketServer } from '../js/seqpacket';
import { kTmp, silently, createDefer, } from './util';

const kServerpath = path.resolve(kTmp, './seqpacket_server.sock');

async function createTestPair(
  next: (args: {
    client: SeqpacketSocket;
    server: SeqpacketServer;
    socket: SeqpacketSocket;
  }) => Promise<any>
) {
  const server = new SeqpacketServer();
  const client = new SeqpacketSocket();
  server.listen(kServerpath);

  const p = new Promise<SeqpacketSocket>((resolve, reject) => {
    server.on('connection', (socket) => {
      resolve(socket);
    });
  });

  client.connect(kServerpath);

  const socket = await p

  await next({
    client,
    server,
    socket,
  });

  socket.destroy();
  client.destroy();
  server.close();
}

describe('SeqpacketSocket', () => {
  beforeAll(() => {
    silently(() => fs.mkdirSync(kTmp));
  });
  beforeEach(async () => {
    silently(() => fs.unlinkSync(kServerpath));
  });

  it('should emit "close"', async () => {
    {
      const socket = new SeqpacketSocket();

      const { p, resolve } = createDefer()

      socket.on('close', () => {
        resolve()
      })

      socket.destroy();

      await p
    }

    {
      const server = new SeqpacketServer();

      const { p, resolve } = createDefer()

      server.on('close', () => {
        resolve()
      })

      server.close();

      await p
    }
  });

  xit('should allow to pass in a fd', async () => {
    // TODO
  });

  it('should support accepting multiple connections', async () => {
    const server = new SeqpacketServer();
    const times = 5;

    server.listen(kServerpath);

    for (let i = 0; i < times; i += 1) {
      const client = new SeqpacketSocket();
      const { p, resolve } = createDefer<SeqpacketSocket>();
      server.once('connection', (socket) => {
        resolve(socket);
      });
      await new Promise<void>((resolve) => {
        client.connect(kServerpath, () => {
          resolve();
        });
      });
      const serverSocket = await p;
      const data = Buffer.from('hello');
      await new Promise<void>((resolve) => {
        client.write(data, 0, data.length, () => {
          resolve();
        });
      });

      client.destroy();
      serverSocket.destroy();
    }

    server.close();
  });

  it('should send some data from the server side', async () => {
    const server = new SeqpacketServer();
    const client = new SeqpacketSocket();

    server.listen(kServerpath);

    const { p, resolve } = createDefer<SeqpacketSocket>();
    server.once('connection', (socket) => {
      resolve(socket);
    });
    await new Promise<void>((resolve, reject) => {
      client.connect(kServerpath, () => {
        resolve();
      });
    });

    const socket = await p;
    const data = Buffer.from('hello');

    const { p: waitData, resolve: resolveWaitData } = createDefer<Buffer>();

    client.on('data', (buf) => {
      resolveWaitData(buf);
    });

    await new Promise<void>((resolve) => {
      socket.write(data, 0, data.length, () => {
        resolve();
      });
    });

    const buf = await waitData;
    expect(buf.toString('hex')).toBe(data.toString('hex'));

    client.destroy();
    socket.destroy();
    server.close();
  });

  // TODO this test should be changed when testing real seqpacket
  it('should support writing a long data', async () => {
    await createTestPair(async (args) => {
      const { client, socket } = args;

      const buf = Buffer.allocUnsafe(1024 * 64);
      const waitData = new Promise<Buffer>((resolve) => {
        const receiveList: Buffer[] = []
        socket.on('data', (data) => {
          receiveList.push(data);
        });

        socket.on('error', err => {
          console.log(err)
        })

        socket.on('end', () => {
          const concated = Buffer.concat(receiveList)
          resolve(concated);
        })
      });

      client.write(buf, 0, buf.length);
      client.end()

      const recv = await waitData;

      expect(recv.length).toBe(buf.length);
      expect(recv.toString('hex')).toBe(buf.toString('hex'));
    })
  })

  it('should allow to write zeroed-data', async () => {
    await createTestPair(async (args) => {
      const {
        client,
        socket,
      } = args;

      const buf = Buffer.alloc(1024);
      buf.fill(0);
      const waitData = new Promise<Buffer>((resolve) => {
        socket.on('data', (data) => {
          resolve(data);
        });
      });
      await new Promise<void>((resolve) => {
        client.write(buf, 0, buf.length, () => {
          resolve()
        });
      })

      const recvBuf = await waitData;
      expect(recvBuf.length).toBe(buf.length);
      expect(recvBuf.toString('hex')).toBe(buf.toString('hex'))
    });
  });

  it('should work as expected', async () => {
    const server = new SeqpacketServer();
    const client = new SeqpacketSocket();

    const { p: waitConnectCb, resolve: resolveConnect } = createDefer();
    const { p: waitConnection, resolve: resolveConnection } = createDefer<{
      socket: SeqpacketSocket;
      addr: string;
    }>();

    server.listen(kServerpath);
    expect(server.address()).toBe(kServerpath);

    server.on('connection', (socket, addr) => {
      resolveConnection({
        socket,
        addr,
      });
    });

    client.connect(kServerpath, () => {
      resolveConnect();
    });

    const { socket, addr } = await waitConnection;
    expect(socket).toBeTruthy();
    expect(addr).toBe('');

    await waitConnectCb;

    const { p: waitWriteCb, resolve } = createDefer();
    const { p: waitDataCb, resolve: resolveWaitDataCb } = createDefer<Buffer>();
    socket.on('data', (buf) => {
      resolveWaitDataCb(buf);
    });

    // send some data from client to server
    const data = Buffer.from('hello');
    client.write(data, 0, data.length, () => {
      resolve();
    });

    await waitWriteCb;

    const recvBuf = await waitDataCb;
    expect(recvBuf.toString('hex')).toBe(data.toString('hex'));

    // send some data back
    const { p: waitClientData, resolve: resolveClientData } =
      createDefer<Buffer>();
    const { p: waitWriteBack, resolve: resolveWaitWriteBack } = createDefer();
    client.on('data', (buf) => {
      resolveClientData(buf);
    });

    socket.write(data, 0, data.length, () => {
      resolveWaitWriteBack();
    });

    await waitWriteBack;
    const buf = await waitClientData;
    expect(buf.toString('hex')).toBe(data.toString('hex'));

    // TODO should not destroy manually.
    socket.destroy();
    server.close();
    expect(() => server.listen(kServerpath)).toThrow();
    client.destroy();
  });

  it('should emit "_shutdown" and "end" event', async () => {
    await createTestPair(async (args) => {
      const { client, socket } = args;

      const dataToWrite: Buffer[] = [
        Buffer.alloc(64 * 1024),
        Buffer.alloc(32 * 1024),
      ]

      const { p, resolve } = createDefer()
      const { p: waitEnd, resolve: resolveEnd } = createDefer()

      socket.on('end', () => {
        resolveEnd()
      })

      client.on('_shutdown', () => {
        resolve()
      })

      for (const data of dataToWrite) {
        client.write(data, 0, data.length, () => {})
      }
      client.end()

      await p
      await waitEnd
    });
  });

  xit('should throw errors that we throw in callbacks of write()', async () => {
    await createTestPair(async (args) => {
      const {client} = args;

      const buf = Buffer.alloc(5);
      client.write(buf, 0, buf.length, () => {
        throw new Error('my_error');
      });
    });
  });

  it('should write the slice of buffer correctly', async () => {
    await createTestPair(async (args) => {
      const { client, socket } = args;

      const buf = Buffer.from('hello, world');

      const { p, resolve } = createDefer<Buffer>()

      socket.on('data', buf => {
        resolve(buf)
      })

      client.write(buf, 7, 5)

      const recvBuf = await p
      expect(recvBuf.toString()).toBe('world')
    })
  });

  it('should throw errors if we write() after socket closed', async () => {
    await createTestPair(async (args) => {
      const { client, socket } = args;

      client.destroy();
      const buf = Buffer.from('hello')
      expect(() => client.write(buf, 0, buf.length)).toThrow('socket has been shutdown')
    })
  });

  it('should throw errors when connect a invalid filepath', async () => {
    const client = new SeqpacketSocket()

    expect(() => client.connect(kServerpath)).toThrow()

    client.destroy();
  });

  it('should emit errors when the other side of socket get closed', async () => {
    await createTestPair(async args => {
      const { client, socket, server } = args;

      socket.destroy();
      server.close();

      const buf = Buffer.alloc(5);
      const { p, resolve } = createDefer();
      client.write(buf, 0, buf.length, () => {
        throw new Error('unexpected');
      });

      client.on('error', err => {
        resolve(err)
      })

      const err = await p
      expect(err).toBeTruthy();
    });
  });

  it('should work if we close sockets on "connect" events', async () => {
    const server = new SeqpacketServer()
    server.listen(kServerpath)

    const client = new SeqpacketSocket()

    client.connect(kServerpath, () => {
      client.destroy();
    })

    server.close()
  });

  it('should be okay if we call destroy() on "end" events', async () => {
    await createTestPair(async args => {
      const { client, socket } = args;

      const buf = Buffer.alloc(5);

      const { p, resolve } = createDefer()

      socket.on('end', () => {
        socket.destroy();
        resolve()
      })

      await new Promise<void>((resolve) => {
        client.write(buf, 0, buf.length);
        client.end(() => {
          resolve()
        })
      });

      await p
    });
  });

  // https://unix.stackexchange.com/questions/498395/detect-unix-domain-socket-deletion
  // it('should work when sock files get deleted', async () => {
  //   const server = new SeqpacketServer()

  //   server.listen(kServerpath)

  //   const { p, resolve } = createDefer();

  //   server.on('error', err => {
  //     resolve(err)
  //   })

  //   fs.unlinkSync(kServerpath)

  //   await p

  //   server.close()
  // })
});
