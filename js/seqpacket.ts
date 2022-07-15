import { SeqpacketSocketWrap } from './addon'

export class SeqpacketSocket {
  private wrap: SeqpacketSocketWrap

  constructor() {
    this.wrap = new SeqpacketSocketWrap()
  }
}
