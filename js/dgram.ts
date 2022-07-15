import { EventEmitter } from 'events';
import {
  dgramAddress,
  dgramBind,
  dgramClose,
  dgramCreateSocket,
  dgramGetRecvBufferSize,
  dgramGetSendBufferSize,
  dgramSendTo,
  dgramSetRecvBufferSize,
  dgramSetSendBufferSize,
  dgramStartRecv,
} from './addon';

type FnRecv = (err: undefined | Error, buf: Buffer) => void;
type FnSend = (err: undefined | Error) => void;

function wrapSocket(obj: DgramSocket) {
  obj.emit = obj.emit.bind(obj);
  dgramCreateSocket(obj);
}

export class DgramSocket extends EventEmitter {
  private closed: boolean = false;

  constructor() {
    super();
    wrapSocket(this);
    dgramStartRecv(this);
    this.on('_data', this.onData);
    this.on('_error', this.onError);
  }

  private onData = (buf: Buffer, filepath: string) => {
    process.nextTick(() => {
      this.emit('data', buf, filepath);
    });
  };

  private onError = (err: Error) => {
    process.nextTick(() => {
      this.close();
      this.emit('error', err);
    });
  };

  private checkClosed() {
    if (this.closed) {
      throw new Error('DgramSocket has been closed');
    }
  }

  bind(socketPath: string) {
    this.checkClosed();
    dgramBind(this, socketPath);
  }

  sendTo(
    buf: Buffer,
    offset: number,
    length: number,
    destPath: string,
    onWrite?: FnSend
  ) {
    this.checkClosed();
    dgramSendTo(this, buf, offset, length, destPath, onWrite);
  }

  getRecvBufferSize() {
    return dgramGetRecvBufferSize(this);
  }

  setRecvBufferSize(size: number) {
    return dgramSetRecvBufferSize(this, size);
  }

  getSendBufferSize() {
    return dgramGetSendBufferSize(this);
  }

  setSendBufferSize(size: number) {
    return dgramSetSendBufferSize(this, size);
  }

  address(): string {
    return dgramAddress(this);
  }

  close() {
    if (this.closed) {
      return;
    }
    this.closed = true;
    dgramClose(this);
  }
}
