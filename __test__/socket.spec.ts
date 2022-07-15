import * as net from 'net'
import { createReuseportFd as createFd, closeFd  } from '../js/index'
import { hasIPv6 } from './util'

describe('tcp', () => {
  describe('createFd', () => {
    it('should work', async () => {
      const host = '0.0.0.0'
      let port = 0;

      async function createServer() {
        const fd = createFd(port, host);

        const server = await new Promise<net.Server>((resolve, reject) => {
          const server = net.createServer()

          server.listen({
            fd,
          }, () => {
            resolve(server)
          })
        })

        port = (server.address() as any).port

        return server
      }

      const servers = [];
      for (let i = 0; i < 5; i += 1) {
        const server = await createServer()
        servers.push(server);
      }

      const pList = servers.map(server => {
        return new Promise((resolve, reject) => {
          server.once('connection', (socket) => {
            socket.on('data', buf => {
              resolve(buf)
            })
          })
        })
      })

      const buf = Buffer.from('hello');
      const socket = net.connect(port, host, () => {
        socket.write(buf);
      });

      const ret = await Promise.race(pList);
      expect(ret.toString()).toBe(buf.toString())

      socket.destroy();

      servers.forEach(server => server.close());
    })

    if (hasIPv6())  {
      it('should work with ipv6', async () => {
        const host = '::1'
        let port = 0;

        const fd = createFd(port, host);

        const server = await new Promise<net.Server>((resolve, reject) => {
          const server = net.createServer()

          server.listen({
            fd,
          }, () => {
            resolve(server)
          })
        })
        port = (server.address() as any).port
        const p = new Promise((resolve, reject) => {
          server.once('connection', (socket) => {
            socket.on('data', buf => {
              resolve(buf)
            })
          })
        })

        const buf = Buffer.from('hello');
        const socket = net.connect(port, host, () => {
          socket.write(buf);
        });

        const ret = await p;
        expect(ret.toString()).toBe(buf.toString())

        socket.destroy();

        server.close();
      });
    }
  })

  describe('closeFd', () => {
    it('should work', async () => {
      const fd = createFd(0)

      closeFd(fd)
    })
  });
})
