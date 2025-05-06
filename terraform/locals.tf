locals {
  dev_mode = "vultr" # "vultr", "scaleway", "none"

  # derived locals
  dev_ipv4       = local.dev_mode == "vultr" ? vultr_instance.dev[0].main_ip : local.dev_mode == "scaleway" ? scaleway_instance_ip.dev_ipv4[0].address : null
  dev_ipv6       = local.dev_mode == "vultr" ? vultr_instance.dev[0].v6_main_ip : local.dev_mode == "scaleway" ? scaleway_instance_ip.dev_ipv6[0].address : null
  instance_id    = local.dev_mode == "vultr" ? vultr_instance.dev[0].id : local.dev_mode == "scaleway" ? scaleway_instance_server.dev[0].id : null
  dev_short_hash = local.dev_mode != "none" ? substr(sha256(local.instance_id), 0, 4) : "foo"
  dev_domain     = "dev-${local.dev_short_hash}.inband-traceroute.net."
}


# This is a workaround for the fact that instances doesn't trigger replacement
# for changes to user_data by default
resource "terraform_data" "cloud_init" {
  input = replace(replace(file("${path.module}/init/cloud-init.yml"), "SSH_PUBKEY", var.ssh_pubkey), "SSH_DEPLOY_KEY_B64", base64encode(tls_private_key.inband_traceroute_deploy_key.private_key_openssh))
}
