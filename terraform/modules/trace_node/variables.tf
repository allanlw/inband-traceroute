variable "ssh_key_id" {
  description = "ID of the Vultr SSH key to use"
  type        = string
}

variable "startup_script_id" {
  description = "ID of the Vultr startup script to use"
  type        = string
}

variable "dns_name" {
  description = "DNS name for instance"
  type        = string
}

variable "dns_zone_name" {
  description = "Google DNS managed zone name"
  type        = string
}

variable "region" {
  description = "Vultr region to deploy the instance in"
  type        = string
}
