/* tslint:disable */
/* eslint-disable */

/* auto-generated by NAPI-RS */

export function socketNewSoReuseportFd(domain: string, port: number, ip: string): number
export function socketClose(fd: number): void
export function initCleanupHook(): void
export class SeqpacketSocketWrap {
  constructor(ee: object, fd?: number | undefined | null)
  init(thisObj: object): void
  state(): number
  close(): void
  shutdownWrite(): void
  uvRefer(): void
  uvUnrefer(): void
  setReadBufSize(size: number): void
  getReadBufSize(): number
  startRecv(): void
  address(): string
  listen(bindpath: string, backlog: number): void
  connect(serverPath: string): void
  write(buf: Buffer, offset: number, length: number, cb?: (...args: any[]) => any | undefined | null): void
  shutdownWhenFlushed(): void
}
export class DgramSocketWrap {
  constructor(ee: object)
  init(thisObj: object): void
  startRecv(): void
  bind(bindpath: string): void
  address(): string
  getRecvBufferSize(): number
  setRecvBufferSize(size: number): void
  getSendBufferSize(): number
  setSendBufferSize(size: number): void
  sendTo(buf: Buffer, offset: number, length: number, path: string, cb?: (...args: any[]) => any | undefined | null): void
  close(): void
}
