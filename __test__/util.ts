import * as path from 'path'
import * as os from 'os'

export const kIsDarwin = os.platform() === 'darwin'

export const kTmp = path.resolve(__dirname, './.tmp')

export const kServerPath = path.resolve(kTmp, './server.sock');

export function wait(t: number) {
  return new Promise<void>((resolve, reject) => {
    setTimeout(() => {
      resolve()
    }, t)
  })
}

export function silently(fn: any) {
  try { fn() } catch (_) { }
}

export function createDefer<T>() {
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

const isWindows = os.platform() === 'win32';

export function hasIPv6() {
  const iFaces = os.networkInterfaces();
  const re = isWindows ? /Loopback Pseudo-Interface/ : /lo/;
  return Object.keys(iFaces).some((name) => {
    return re.test(name) &&
           iFaces[name]?.some(({ family }) => family === 'IPv6');
  });
}
