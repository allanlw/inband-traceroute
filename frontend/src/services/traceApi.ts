export interface EnrichedInfo {
  country: string | null
  country_code: string | null
  continent: string | null
  continent_code: string | null
  asn: string | null
  as_name: string | null
  as_domain: string | null
  ip: string | null
}

export interface TraceMessage {
  ttl: number
  hop_type: string
  addr: string | null
  rtt: number | null
  enriched_info: EnrichedInfo | null
}

export interface ReverseDnsMessage {
  ttl: number
  ip: string
  name: { Ok?: string; Err?: string }
}

export type TraceEvent = { Hop: TraceMessage } | { ReverseDns: ReverseDnsMessage }

export interface Node {
  dns_name: string
  ipv4: string
  ipv6: string
}

export type IpVersion = 'ipv4' | 'ipv6'

export interface NodeConnection {
  id: string
  ipVersion: IpVersion
  status: 'connecting' | 'connected' | 'disconnected' | 'error'
  lastMessage?: TraceMessage
  traceMessages: TraceMessage[]
  lastUpdate: Date
  error?: string
}

export interface NodeConnectionMap {
  [nodeId: string]: {
    ipv4: NodeConnection
    ipv6: NodeConnection
  }
}

export interface NodeMap {
  [nodeId: string]: Node
}

export type NodeEventCallback = (
  nodeId: string,
  ipVersion: IpVersion,
  message: TraceMessage | ReverseDnsMessage,
  eventType: 'Hop' | 'ReverseDns'
) => void
export type NodeStatusCallback = (nodeId: string, ipVersion: IpVersion, status: NodeConnection['status'], error?: string) => void

// Add TraceData type for use in components
export interface TraceData {
  [ttl: number]: {
    [nodeId: string]: {
      ipv4?: TraceMessage
      ipv6?: TraceMessage
    }
  }
}

class TraceAPI {
  private eventSources = new Map<string, { ipv4?: EventSource; ipv6?: EventSource }>()
  private baseUrl = 'https://inband-traceroute.net'

  async fetchNodes(): Promise<NodeMap> {
    const response = await fetch(`${this.baseUrl}/nodes.json`)
    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`)
    }
    return response.json()
  }

  private getNodeUrl(node: Node, ipVersion: IpVersion): string {
    const baseDomain = node.dns_name
    const prefix = ipVersion === 'ipv4' ? 'ipv4.' : 'ipv6.'
    return `https://${prefix}${baseDomain}/sse`
  }
}

export const traceApi = new TraceAPI()
