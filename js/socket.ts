import { isIPv4, isIP } from 'net';
import { socketNewSoReuseportFd, socketClose } from './addon'

export function createReuseportFd(port: number = 0, host: string = '0.0.0.0'): number {
  if (!isIP(host)) {
    throw new Error('invalid host');
  }

  const domain = isIPv4(host) ? 'ipv4' : 'ipv6'

  const fd = socketNewSoReuseportFd(domain, port, host);

  return fd;
}

export function closeFd(fd: number) {
  socketClose(fd)
}
