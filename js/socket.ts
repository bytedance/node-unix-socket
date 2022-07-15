import { isIPv4, isIP } from 'net';
import { socketNewSoReuseportFd, socketClose } from './addon';

/**
 * Create a TCP socket with SO_REUSEADDR and SO_REUSEPORT enabled.
 *
 * Use the returned fd to create a [net.Server](https://nodejs.org/docs/latest-v16.x/api/net.html#class-netserver):
 *
 * ```typescript
 * const fd = createReuseportFd(9229, '127.0.0.0');
 * const server = require('net').createServer();
 * server.listen({ fd }, () => { console.log('listen() successfully') })
 * ```
 *
 * @param port
 * @param host
 * @returns Return a fd binds to the address.
 */
export function createReuseportFd(
  port: number = 0,
  host: string = '0.0.0.0'
): number {
  if (!isIP(host)) {
    throw new Error('invalid host');
  }

  const domain = isIPv4(host) ? 'ipv4' : 'ipv6';

  const fd = socketNewSoReuseportFd(domain, port, host);

  return fd;
}

/**
 * Close a fd.
 *
 * Note that you don't need to manually close fd that is listened by net.Server.
 * @param fd
 */
export function closeFd(fd: number) {
  socketClose(fd);
}
