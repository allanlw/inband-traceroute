resource "vultr_ssh_key" "inband_traceroute_tf" {
  name    = "inband-traceroute-tf"
  ssh_key = var.ssh_pubkey
}

resource "vultr_startup_script" "init" {
  name   = "inband-traceroute"
  script = sensitive(base64encode(replace(replace(file("${path.module}/init/cloud-init.yml"), "SSH_PUBKEY", var.ssh_pubkey), "SSH_DEPLOY_KEY_B64", base64encode(tls_private_key.inband_traceroute_deploy_key.private_key_openssh))))
}

module "trace_node" {
  count = 0

  source            = "./modules/trace_node"
  ssh_key_id        = vultr_ssh_key.inband_traceroute_tf.id
  startup_script_id = vultr_startup_script.init.id
  dns_zone_name     = google_dns_managed_zone.inband_traceroute.name
  dns_name          = "dev.inband-traceroute.net."
}
