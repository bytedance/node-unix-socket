import * as path from 'path'

export const kTmp = path.resolve(__dirname, './.tmp')

export function wait(t: number) {
  return new Promise<void>((resolve, reject) => {
    setTimeout(() => {
      resolve()
    }, t)
  })
}

export function sliently(fn) {
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
