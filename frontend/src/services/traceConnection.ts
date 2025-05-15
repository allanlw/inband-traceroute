import type { Node } from '@/services/traceApi'

export type TraceUpdateEvent = {
  hops: [number, any][];
  reverseDnsMap: { [ttl: number]: any };
  status: 'not-started' | 'in-progress' | 'done';
}

export class TraceConnection extends EventTarget {
  private eventSource: EventSource | null = null
  private traceData: { [ttl: number]: any } = {}
  private reverseDns: { [ttl: number]: any } = {}
  private _status: 'not-started' | 'in-progress' | 'done' = 'not-started'

  constructor(
    private node: Node,
    private protocol: 'IPv4' | 'IPv6',
    private getNodeUrl: (node: Node, protoKey: string) => string
  ) {
    super()
  }

  get status() {
    return this._status
  }

  connect() {
    this._status = 'in-progress'
    const protoKey = this.protocol === 'IPv4' ? 'ipv4' : 'ipv6'
    const url = this.getNodeUrl(this.node, protoKey)
    this.eventSource = new EventSource(url)
    this.eventSource.onmessage = (event) => {
      const evt = JSON.parse(event.data)
      if (evt && 'Hop' in evt) {
        const hop = evt.Hop
        this.traceData[hop.ttl] = hop
        this.publish()
      } else if (evt && 'ReverseDns' in evt) {
        const rdns = evt.ReverseDns
        for (const [ttl, hop] of Object.entries(this.traceData)) {
          if (hop.addr === rdns.ip) {
            this.reverseDns[Number(ttl)] = rdns
          }
        }
        this.publish()
      }
    }
    this.eventSource.onerror = (err) => {
      this.eventSource?.close()
      this.eventSource = null
      this._status = 'done'
      this.publish()
    }
  }

  disconnect() {
    this.eventSource?.close()
    this.eventSource = null
    this._status = 'done'
    this.publish()
  }

  addUpdateListener(listener: (event: TraceUpdateEvent) => void) {
    this.addEventListener('update', (e: Event) => {
      listener((e as CustomEvent<TraceUpdateEvent>).detail)
    })
  }

  private publish() {
    const hops = Object.entries(this.traceData)
      .map(([k, v]) => [Number(k), v] as [number, any])
      .sort((a, b) => b[0] - a[0])
    const event: TraceUpdateEvent = {
      hops,
      reverseDnsMap: { ...this.reverseDns },
      status: this._status
    }
    this.dispatchEvent(new CustomEvent<TraceUpdateEvent>('update', { detail: event }))
  }
}
