
output "dns_name" {
  description = "The DNS name of the node."
  value       = trimsuffix(var.dns_name, ".")
}

output "ipv4" {
  description = "The IPv4 address of the node."
  value       = vultr_instance.instance.main_ip
}

output "ipv6" {
  description = "The IPv6 address of the node."
  value       = vultr_instance.instance.v6_main_ip
}
