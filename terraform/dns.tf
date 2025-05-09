resource "google_dns_managed_zone" "inband_traceroute" {
  name     = "inband-traceroute"
  dns_name = "inband-traceroute.net."
}
