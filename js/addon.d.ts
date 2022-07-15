/* tslint:disable */
/* eslint-disable */

/* auto-generated by NAPI-RS */

export class SeqpacketSocketWrap {
  constructor(emit: (...args: any[]) => any, fd?: number | undefined | null)
  setReadBufSize(size: number): void
  startRecv(): void
  address(): string
  listen(bindpath: string, backlog: number): void
  connect(serverPath: string): void
  write(buf: Buffer, offset: number, length: number, cb?: (...args: any[]) => any | undefined | null): void
  close(): void
  shutdownWhenFlushed(): void
}
export class DgramSocketWrap {
  constructor(recvCb: (...args: any[]) => any)
  startRecv(): void
  refThis(thisObj: object): void
  bind(bindpath: string): void
  address(): string
  getRecvBufferSize(): number
  setRecvBufferSize(size: number): void
  getSendBufferSize(): number
  setSendBufferSize(size: number): void
  sendTo(buf: Buffer, offset: number, length: number, path: string, cb: (...args: any[]) => any): void
  close(): void
}
