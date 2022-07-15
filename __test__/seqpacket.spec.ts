import * as path from 'path';
import * as fs from 'fs';
import { SeqpacketSocket, SeqpacketServer } from '../js/seqpacket';
import { kTmp, silently, createDefer, kIsDarwin, wait } from './util';

const kServerpath = path.resolve(kTmp, './seqpacket_server.sock');
const kInvalidPath = path.resolve(kTmp, './INVALID_PATH');

async function createTestPair(
  next: (args: {
    client: SeqpacketSocket;
    server: SeqpacketServer;
    socket: SeqpacketSocket;
  }) => Promise<any>,
  options: { autoClose: boolean } = { autoClose: true }
) {
  const { autoClose } = options
  const server = new SeqpacketServer();
  const client = new SeqpacketSocket();
  server.listen(kServerpath);

  const p = new Promise<SeqpacketSocket>((resolve, reject) => {
    server.on('connection', (socket) => {
      resolve(socket);
    });
  });

  client.connect(kServerpath);

  const socket = await p;

  await next({
    client,
    server,
    socket,
  });

  if (!autoClose) {
    return
  }
  socket.destroy();
  client.destroy();
  server.close();
}

if (!kIsDarwin) {
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

        const { p, resolve } = createDefer();

        socket.on('close', () => {
          resolve();
        });

        socket.destroy();

        await p;
      }

      {
        const server = new SeqpacketServer();

        const { p, resolve } = createDefer();

        server.on('close', () => {
          resolve();
        });

        server.close();

        await p;
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

        const buf = Buffer.allocUnsafe(1024 * 32);
        const waitData = new Promise<Buffer>((resolve) => {
          const receiveList: Buffer[] = [];
          socket.on('data', (data) => {
            receiveList.push(data);
          });

          socket.on('error', (err) => {
            console.log(err);
          });

          socket.on('end', () => {
            const concated = Buffer.concat(receiveList);
            resolve(concated);
          });
        });

        client.write(buf, 0, buf.length);
        client.end();

        const recv = await waitData;

        expect(recv.length).toBe(buf.length);
        expect(recv.toString('hex')).toBe(buf.toString('hex'));
      });
    });

    it('should allow to write zeroed-data', async () => {
      await createTestPair(async (args) => {
        const { client, socket } = args;

        const buf = Buffer.alloc(1024);
        buf.fill(0);
        const waitData = new Promise<Buffer>((resolve) => {
          socket.on('data', (data) => {
            resolve(data);
          });
        });
        await new Promise<void>((resolve) => {
          client.write(buf, 0, buf.length, () => {
            resolve();
          });
        });

        const recvBuf = await waitData;
        expect(recvBuf.length).toBe(buf.length);
        expect(recvBuf.toString('hex')).toBe(buf.toString('hex'));
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
      const { p: waitDataCb, resolve: resolveWaitDataCb } =
        createDefer<Buffer>();
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
        ];

        const { p, resolve } = createDefer();
        const { p: waitEnd, resolve: resolveEnd } = createDefer();

        socket.on('end', () => {
          resolveEnd();
        });

        client.on('_shutdown', () => {
          resolve();
        });

        for (const data of dataToWrite) {
          client.write(data, 0, data.length, () => {});
        }
        client.end();

        await p;
        await waitEnd;
      });
    });

    xit('should throw errors that we throw in callbacks of write()', async () => {
      await createTestPair(async (args) => {
        const { client } = args;

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

        const { p, resolve } = createDefer<Buffer>();

        socket.on('data', (buf) => {
          resolve(buf);
        });

        client.write(buf, 7, 5);

        const recvBuf = await p;
        expect(recvBuf.toString()).toBe('world');
      });
    });

    it('should throw errors if we write() after socket closed', async () => {
      await createTestPair(async (args) => {
        const { client, socket } = args;

        client.destroy();
        const buf = Buffer.from('hello');
        expect(() => client.write(buf, 0, buf.length)).toThrow(
          'SeqpacketSocket has been destroyed'
        );
      });
    });

    it('should throw errors when connect a invalid filepath', async () => {
      const client = new SeqpacketSocket();

      expect(() => client.connect(kServerpath)).toThrow();

      client.destroy();
    });

    it('should emit errors when the other side of socket get closed', async () => {
      await createTestPair(async (args) => {
        const { client, socket, server } = args;

        socket.destroy();
        server.close();

        const buf = Buffer.alloc(5);
        const { p, resolve } = createDefer();
        client.write(buf, 0, buf.length, () => {
          throw new Error('unexpected');
        });

        client.on('error', (err) => {
          resolve(err);
        });

        const err = await p;
        expect(err).toBeTruthy();
      });
    });

    it('should work if we close sockets on "connect" events', async () => {
      const server = new SeqpacketServer();
      server.listen(kServerpath);

      const client = new SeqpacketSocket();

      await new Promise<void>((resolve, reject) => {
        client.connect(kServerpath, async () => {
          resolve();
          client.destroy();
        });
      });

      server.close();
    });

    // NOTE: this test might be not stable
    it('should emit an error if we close the server before the client finish connecting', async () => {
      const server = new SeqpacketServer();
      server.listen(kServerpath);

      const client = new SeqpacketSocket();
      const { p, resolve } = createDefer<Error>();

      client.on('error', (err) => {
        resolve(err);
      });

      client.connect(kServerpath);

      server.close();

      const err = await p;

      expect(err.message).toContain('EBADF');

      client.destroy();
    });

    it('should be okay if we call destroy() on "end" events', async () => {
      await createTestPair(async (args) => {
        const { client, socket } = args;

        const buf = Buffer.alloc(5);

        const { p, resolve } = createDefer();

        socket.on('end', () => {
          socket.destroy();
          resolve();
        });

        await new Promise<void>((resolve) => {
          client.write(buf, 0, buf.length);
          client.end(() => {
            resolve();
          });
        });

        await p;
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

    it('should truncate a long buffer if the internal-read-buffer-size is not large enough', async () => {
      await createTestPair(async (args) => {
        const { client, socket } = args;

        const size = 128 * 1024;
        client.setInternalReadBufferSize(size);
        expect(client.getInternalReadBufferSize()).toBe(size);
        socket.setInternalReadBufferSize(size);
        expect(socket.getInternalReadBufferSize()).toBe(size);

        const data = Buffer.allocUnsafe(size);

        {
          client.write(data, 0, data.length);
          const buf = await new Promise<Buffer>((resolve, reject) => {
            socket.once('data', (buf) => {
              resolve(buf);
            });
          });
          expect(buf.length).toBe(data.length);
          expect(buf.toString('hex')).toBe(data.toString('hex'));
        }

        const smallSize = 32 * 1024;
        socket.setInternalReadBufferSize(smallSize);
        expect(socket.getInternalReadBufferSize()).toBe(smallSize);

        {
          client.write(data, 0, data.length);
          const buf = await new Promise<Buffer>((resolve, reject) => {
            socket.once('data', (buf) => {
              resolve(buf);
            });
          });

          expect(buf.length).toBe(smallSize);
          expect(buf.toString('hex')).toBe(
            data.slice(0, buf.length).toString('hex')
          );
        }
      });
    });

    it('should receive messages in order and keep messages length', async () => {
      await createTestPair(async (args) => {
        const { client, socket } = args;

        const dataToSend: Buffer[] = [];

        for (let i = 0; i < 10; i += 1) {
          const buf = Buffer.allocUnsafe(Math.random() * 100 + 1);
          dataToSend.push(buf);
        }

        let receivedIndex = 0;

        socket.on('data', (buf) => {
          expect(buf.toString('hex')).toBe(
            dataToSend[receivedIndex].toString('hex')
          );
          receivedIndex += 1;
        });

        const { p, resolve } = createDefer();
        socket.on('end', () => {
          resolve();
        });

        for (const data of dataToSend) {
          client.write(data, 0, data.length)
        }
        client.end()

        await p;

        expect(receivedIndex).toBe(dataToSend.length);
      });
    });

    it('should ref', async () => {
      await createTestPair(async (args) => {
        // TODO how to test
        const { client, server, socket } = args;

        server.ref();
        server.unref();
        client.ref();
        client.unref();
      });
    });

    it('should write whole buffer if "offset" and "length" are missed', async () => {
      await createTestPair(async (args) => {
        const { client, socket } = args;
        const buf = Buffer.from('hello, world');
        client.write(buf)

        const {p, resolve } = createDefer();
        socket.on('data', received => {
          expect(received.toString('hex')).toBe(buf.toString('hex'))
          resolve()
        })
        await p
      });
    });

    it('should emit "close" in sockets automatically when both read and write side of sockets are end', async () => {
      await createTestPair(async (args) => {
        const { client, socket, server } = args;

        const { p: p1, resolve: r1 } = createDefer();
        const { p: p2, resolve: r2 } = createDefer();

        client.on('close', () => {
          r1()
        });
        socket.on('close', () => {
          r2()
        })

        client.write(Buffer.alloc(1024 * 64))
        client.end()
        socket.write(Buffer.alloc(1024 * 64))
        socket.end()
        await Promise.all([p1, p2])

        server.close();
      }, {
        autoClose: false,
      })
    });
  });
} else {
  describe('seqpacket', () => {
    it('(tests skipped)', () => {});
  });
}
