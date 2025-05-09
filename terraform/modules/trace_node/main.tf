resource "vultr_instance" "instance" {
  hostname = trim(replace(var.dns_name, ".", "0"), "0")
  label    = trim(replace(var.dns_name, ".", "0"), "0")

  region = var.region
  plan   = "vc2-4c-8gb"

  os_id       = 2136 # Debian 12 x64
  enable_ipv6 = true
  user_scheme = "limited"
  ssh_key_ids = [var.ssh_key_id]
  script_id   = var.startup_script_id
}

resource "google_dns_record_set" "dev_inband_traceroute_a" {
  name         = var.dns_name
  managed_zone = var.dns_zone_name
  type         = "A"
  ttl          = 60
  rrdatas      = [vultr_instance.instance.main_ip]
}

resource "google_dns_record_set" "dev_inband_traceroute_aaaa" {
  name         = var.dns_name
  managed_zone = var.dns_zone_name
  type         = "AAAA"
  ttl          = 60
  rrdatas      = [vultr_instance.instance.v6_main_ip]
}

resource "google_dns_record_set" "ipv4_dev_inband_traceroute_a" {
  name         = "ipv4.${var.dns_name}"
  managed_zone = var.dns_zone_name
  type         = "A"
  ttl          = 60
  rrdatas      = [vultr_instance.instance.main_ip]
}

resource "google_dns_record_set" "ipv6_dev_inband_traceroute_aaaa" {
  name         = "ipv6.${var.dns_name}"
  managed_zone = var.dns_zone_name
  type         = "AAAA"
  ttl          = 60
  rrdatas      = [vultr_instance.instance.v6_main_ip]
}
