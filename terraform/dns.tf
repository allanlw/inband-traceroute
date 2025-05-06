resource "google_dns_managed_zone" "inband_traceroute" {
  name     = "inband-traceroute"
  dns_name = "inband-traceroute.net."
}

resource "google_dns_record_set" "dev_inband_traceroute_a" {
  count        = local.dev_mode != "none" ? 1 : 0
  name         = local.dev_domain
  managed_zone = google_dns_managed_zone.inband_traceroute.name
  type         = "A"
  ttl          = 60

  rrdatas = [
    local.dev_ipv4
  ]
}

resource "google_dns_record_set" "dev_inband_traceroute_aaaa" {
  count        = local.dev_mode != "none" ? 1 : 0
  name         = local.dev_domain
  managed_zone = google_dns_managed_zone.inband_traceroute.name
  type         = "AAAA"
  ttl          = 60

  rrdatas = [
    local.dev_ipv6
  ]
}


resource "google_dns_record_set" "ipv4_dev_inband_traceroute_a" {
  count        = local.dev_mode != "none" ? 1 : 0
  name         = "ipv4.${local.dev_domain}"
  managed_zone = google_dns_managed_zone.inband_traceroute.name
  type         = "A"
  ttl          = 60

  rrdatas = [
    local.dev_ipv4
  ]
}

resource "google_dns_record_set" "ipv6_dev_inband_traceroute_aaaa" {
  count        = local.dev_mode != "none" ? 1 : 0
  name         = "ipv6.${local.dev_domain}"
  managed_zone = google_dns_managed_zone.inband_traceroute.name
  type         = "AAAA"
  ttl          = 60

  rrdatas = [
    local.dev_ipv6
  ]
}

resource "google_dns_record_set" "dev_inband_traceroute_current_cname" {
  count        = local.dev_mode != "none" ? 1 : 0
  name         = "dev-current.inband-traceroute.net."
  managed_zone = google_dns_managed_zone.inband_traceroute.name
  type         = "CNAME"
  ttl          = 60

  rrdatas = [
    local.dev_domain
  ]
}
