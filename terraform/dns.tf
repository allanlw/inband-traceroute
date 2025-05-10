resource "namecheap_domain_records" "inband_traceroute" {
  domain = local.domain_name
  mode   = "OVERWRITE" // Warning: this will remove all manually set records

  nameservers = cloudflare_zone.inband_traceroute.name_servers
}

resource "cloudflare_zone" "inband_traceroute" {
  account = {
    id = var.cloudflare_account_id
  }
  name = local.domain_name
  type = "full"
}
