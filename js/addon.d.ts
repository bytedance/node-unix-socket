/* tslint:disable */
/* eslint-disable */

/* auto-generated by NAPI-RS */

export class SeqpacketSocketWrap {
  constructor()
  write(buf: Buffer, cb: (...args: any[]) => any): void
  close(): void
}
export class DgramSocketWrap {
  constructor(recvCb: (...args: any[]) => any)
  startRecv(): void
  /**
  * NOTE: Because we can't get the "this" js object of DgramSocketWrap instances,
  * we need to call ref_this manually in the js side to prevent the js object
  * from been garbage-collected.
  *
  * TODO Is there a way to get the js object in rust side?
  */
  refThis(thisObj: object): void
  bind(bindpath: string): void
  address(): string
  getRecvBufferSize(): number
  setRecvBufferSize(size: number): void
  getSendBufferSize(): number
  setSendBufferSize(size: number): void
  /**
  * buf, offset, length, path
  */
  sendTo(buf: Buffer, offset: number, length: number, path: string, cb: (...args: any[]) => any): void
  close(): void
}
