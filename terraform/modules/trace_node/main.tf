resource "vultr_instance" "instance" {
  hostname = trimsuffix(replace(var.dns_name, ".", "0"), "0")
  label    = trimsuffix(replace(var.dns_name, ".", "0"), "0")

  region = var.region
  plan   = "vc2-4c-8gb"

  os_id       = 2136 # Debian 12 x64
  enable_ipv6 = true
  user_scheme = "limited"
  ssh_key_ids = [var.ssh_key_id]
  script_id   = var.startup_script_id
}


resource "cloudflare_dns_record" "dns_records" {
  for_each = {
    main_ipv4 = {
      prefix = ""
      type   = "A"
      ip     = vultr_instance.instance.main_ip
    }
    main_ipv6 = {
      prefix = ""
      type   = "AAAA"
      ip     = vultr_instance.instance.v6_main_ip
    }
    ipv4_specific = {
      prefix = "ipv4."
      type   = "A"
      ip     = vultr_instance.instance.main_ip
    }
    ipv6_specific = {
      prefix = "ipv6."
      type   = "AAAA"
      ip     = vultr_instance.instance.v6_main_ip
    }
  }

  proxied = false
  zone_id = var.dns_zone_id
  name    = "${each.value.prefix}${trimsuffix(var.dns_name, ".")}"
  type    = each.value.type
  ttl     = 60
  content = each.value.ip


  // https://github.com/vultr/terraform-provider-vultr/issues/271
  lifecycle {
    ignore_changes = [
      content,
    ]
  }
}
