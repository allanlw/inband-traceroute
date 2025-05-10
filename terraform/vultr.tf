resource "vultr_ssh_key" "inband_traceroute_tf" {
  name    = "inband-traceroute-tf"
  ssh_key = var.ssh_pubkey
}

resource "terraform_data" "init_script" {
  input = base64encode(
    templatefile(
      "${path.module}/init/cloud-init.yml.tftpl",
      {
        ssh_pubkey         = var.ssh_pubkey
        ssh_deploy_key_b64 = base64encode(tls_private_key.inband_traceroute_deploy_key.private_key_openssh)
        ipinfoio_token_b64 = base64encode(var.ipinfoio_token)
      }
    )
  )
}

resource "vultr_startup_script" "init" {
  name   = "inband-traceroute"
  script = sensitive(terraform_data.init_script.output)
  lifecycle {
    replace_triggered_by = [
      terraform_data.init_script.output,
    ]
  }
}

module "trace_node" {
  for_each = local.vultr_regions

  source = "./modules/trace_node"

  ssh_key_id        = vultr_ssh_key.inband_traceroute_tf.id
  startup_script_id = vultr_startup_script.init.id
  dns_zone_id       = cloudflare_zone.inband_traceroute.id
  dns_name          = "${each.key}.nodes.inband-traceroute.net."
  region            = each.key
}
