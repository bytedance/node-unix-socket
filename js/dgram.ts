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
export type SendCb = (err: undefined | Error) => void;

function wrapSocket(obj: DgramSocket) {
  obj.emit = obj.emit.bind(obj);
  dgramCreateSocket(obj);
}

/**
 * DgramSocket is used to create a SOCK_DGRAM unix domain socket.
 * Currently DgramSocket doesn't work with `cluster` module.
 *
 * DgramSocket is also an `EventEmitter` and will emit events including:
 *
 * ### Event: `'data'`
 * - buffer `Buffer`
 * - path `string`
 *
 * Emitted when data is received. `path` indicates remote address information.
 *
 * ### Event: `'error'`
 * - error `Error`
 *
 * Emitted when an error occurs.
 *
 * ### Event: `'close'`
 * The 'close' event is emitted after a socket is closed with close().
 */
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
    this.close();
    this.emit('error', err);
  };

  private checkClosed() {
    if (this.closed) {
      throw new Error('DgramSocket has been closed');
    }
  }

  /**
   * Listen for datagram messages on a path.
   * @param socketPath
   */
  bind(socketPath: string) {
    this.checkClosed();
    dgramBind(this, socketPath);
  }

  /**
   * Send messages to the destination path.
   * @param buf
   * @param offset
   * @param length
   * @param destPath
   * @param onWrite
   */
  sendTo(
    buf: Buffer,
    offset: number,
    length: number,
    destPath: string,
    onWrite?: SendCb
  ) {
    this.checkClosed();
    dgramSendTo(this, buf, offset, length, destPath, onWrite);
  }

  /**
   * @returns the SO_RCVBUF socket receive buffer size in bytes.
   */
  getRecvBufferSize() {
    return dgramGetRecvBufferSize(this);
  }

  /**
   * Sets the SO_RCVBUF socket option. Sets the maximum socket receive buffer in bytes.
   * @param size
   * @returns
   */
  setRecvBufferSize(size: number) {
    return dgramSetRecvBufferSize(this, size);
  }

  /**
   * @returns the SO_SNDBUF socket send buffer size in bytes.
   */
  getSendBufferSize() {
    return dgramGetSendBufferSize(this);
  }

  /**
   * Sets the SO_SNDBUF socket option. Sets the maximum socket send buffer in bytes.
   * @param size
   * @returns
   */
  setSendBufferSize(size: number) {
    return dgramSetSendBufferSize(this, size);
  }

  /**
   * Returns the bound address.
   * @returns
   */
  address(): string {
    return dgramAddress(this);
  }

  /**
   * Close the underlying socket and stop listening for data on it.
   * @returns
   */
  close() {
    if (this.closed) {
      return;
    }
    this.closed = true;
    dgramClose(this);
  }
}
