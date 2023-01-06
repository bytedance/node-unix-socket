import { EventEmitter } from 'events';
import {
  SeqpacketSocketWrap
} from './addon';

export type NotifyCb = () => void;

/**
 * SeqpacketServer is used to create a SOCK_SEQPACKET server.
 * Note that sockets of SOCK_SEQPACKET don't works on MacOS and currently SeqpacketServer doesn't work with `cluster` module, i.e. you can't share a SeqpacketServer across different Node.js processes.
 *
 * SeqpacketServer is also an `EventEmitter` and will emit events including:
 *
 * ### Event: `'connection'`:
 * - socket `SeqpacketSocket`
 * - bindpath `string`
 *
 * Emitted when a new connection is made.
 *
 * ### Event: `'error'`
 * - error `Error`
 *
 * Emitted when an error occurs.
 *
 * ### Event: `'close'`
 *
 * Emitted when the server closes.
 */
export class SeqpacketServer extends EventEmitter {
  private closed: boolean = false;
  private wrap: SeqpacketSocketWrap;

  constructor() {
    super();

    this.emit = this.emit.bind(this);
    this.wrap = new SeqpacketSocketWrap(this);
    // TODO currently we can't get this object in rust side
    this.wrap.init(this.wrap);

    this.on('_connection', this.onConnection);
    this.on('_error', this.onError);
  }

  private checkClosed() {
    if (this.closed) {
      throw new Error('SeqpacketServer has been closed');
    }
  }

  private onError = (err: Error) => {
    // TODO test this
    this.emit('error', err);
  };

  private onConnection = (fd: number, addr: string) => {
    const socket = new SeqpacketSocket(fd);
    this.emit('connection', socket, addr);
  };

  /**
   * Returns the bound address.
   * @returns
   */
  address(): string {
    this.checkClosed();
    return this.wrap.address();
  }

  /**
   * Stops the server from accepting new connections and keeps existing connections.
   *
   * This function is synchronous.
   * @returns
   */
  close() {
    if (this.closed) {
      return;
    }
    this.closed = true;
    this.wrap.close();
  }

  /**
   * Start a server listening for connections on the given path. This function is synchronous.
   * @param bindpath
   * @param backlog
   */
  listen(bindpath: string, backlog: number = 511) {
    this.checkClosed();
    this.wrap.listen(bindpath, backlog);
  }

  /**
   * Reference the server so that it will prevent Node.js process from exiting automatically.
   */
  ref() {
    this.wrap.uvRefer();
  }

  /**
   * Unreference the server so that it won't prevent Node.js process from exiting automatically.
   */
  unref() {
    this.wrap.uvUnrefer();
  }
}

/**
 * SeqpacketSocket is an abstraction of a SOCK_SEQPACKET socket.
 *
 * SeqpacketSocket is also an `EventEmitter` and will emit events including:
 *
 * ### Event: `'connect'`
 *
 * Emitted when a socket connection is successfully established.
 *
 * ### Event: `'data'`
 *
 * - buffer `Buffer`
 * Emitted when data is received. All message boundaries in incoming datagrams are preserved.
 *
 * ### Event: `'end'`
 * Emitted when the other end of the socket signals the end of transmission, thus ending the readable side of the socket.
 *
 * ### Event: `'error'`
 * - error `Error`
 * Emitted when an error occurs. The 'close' event will be called directly following this event.
 *
 * ### Event: `'close'`
 * Emitted once the socket is fully closed.
 */
export class SeqpacketSocket extends EventEmitter {
  private wrap: SeqpacketSocketWrap;
  private destroyed: boolean = false;
  private connectCb?: NotifyCb;
  private shutdownCb?: NotifyCb;
  private shutdown: boolean = false;
  private isEnd: boolean = false

  constructor(fd?: number) {
    super();

    this.emit = this.emit.bind(this);
    this.wrap = new SeqpacketSocketWrap(this, fd);
    // TODO currently we can't get this object in rust side
    this.wrap.init(this.wrap);

    if (fd) {
      this.wrap.startRecv();
    }
    this.on('_data', this.onData);
    this.on('end', this.onEnd);
    this.on('_connect', this.onConnect);
    this.on('_error', this.onError);
    this.on('_shutdown', this.onShutdown);
  }

  private onEnd = () => {
    this.isEnd = true;
    this.checkClose();
  }

  private onShutdown = () => {
    this.shutdown = true;
    this.checkClose();
    if (this.shutdownCb) {
      this.shutdownCb();
      this.shutdownCb = undefined;
    }
  };

  private onData = (buf: Buffer) => {
    this.emit('data', buf);
  };

  private checkDestroyed() {
    if (this.destroyed) {
      throw new Error('SeqpacketSocket has been destroyed');
    }
  }

  private onError = (err: Error) => {
    process.nextTick(() => {
      this.emit('error', err);
      this.destroy();
    });
  };

  private onConnect = () => {
    this.wrap.startRecv();
    this.emit('connect');
    if (this.connectCb) {
      this.connectCb();
      this.connectCb = undefined;
    }
  };

  private checkClose() {
    if (this.isEnd && this.shutdown) {
      this.destroy()
    }
  }

  // address(): string {
  //   this.checkDestroyed();
  //   return seqAddress(this);
  // }

  /**
   * Initiate a connection on a given socket.
   *
   * This function is asynchronous. When the connection is established, the 'connect' event will be emitted.
   * However, connect() will throw error synchronously if the 'serverPath' is not a valid Seqpacket server.
   * @param serverPath
   * @param connectCb
   */
  connect(serverPath: string, connectCb?: NotifyCb) {
    this.checkDestroyed();
    this.connectCb = connectCb;
    this.wrap.connect(serverPath);
  }

  /**
   * Sends data on the socket. The `cb` is called when data is written to operating system.
   * @param buf
   * @param offset
   * @param length
   * @param cb
   */
  write(buf: Buffer, offset?: number, length?: number, cb?: NotifyCb) {
    if (arguments.length === 1) {
      offset = 0
      length = buf.length
    }
    this.checkDestroyed();
    const v = offset || 0;
    this.wrap.write(buf, offset || 0, length || buf.length, cb);
  }

  /**
   * Half-closes the socket. i.e., it sends a FIN packet. It is possible the server will still send some data.
   * @param cb
   */
  end(cb?: NotifyCb) {
    this.shutdownCb = cb;
    this.wrap.shutdownWhenFlushed();
  }

  /**
   * Return the size of buffer that SeqpacketSocket uses to receive data. The data will be truncated if the buffer size is not large enough.
   *
   * Default size is 256KB.
   * @returns
   */
  getInternalReadBufferSize(): number {
    return this.wrap.getReadBufSize();
  }

  /**
   * Set the size of buffer that SeqpacketSocket uses to receive data.
   *
   * @param size
   */
  setInternalReadBufferSize(size: number) {
    this.wrap.setReadBufSize(size);
  }

  /**
   * Reference the socket so that it will prevent Node.js process from exiting automatically.
   */
  ref() {
    this.wrap.uvRefer();
  }

  /**
   * Unreference the socket so that it won't prevent Node.js process from exiting automatically.
   */
  unref() {
    this.wrap.uvUnrefer();
  }

  /**
   * Ensures that no more I/O activity happens on this socket. Destroys the stream and closes the connection.
   */
  destroy() {
    if (this.destroyed) {
      return;
    }
    this.destroyed = true;
    this.wrap.close();
  }

  /**
   * For test only
   * @ignore
   */
  _state() {
    return this.wrap.state()
  }
}
